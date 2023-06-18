use std::collections::BTreeMap;

use goblin::pe;

use crate::{
    configuration::OSArchitecture,
    error::{Result, WinDiffError},
    pdb::Pdb,
};

/// Extract syscalls found in a given PE (if supported for that PE).
pub fn extract_syscalls(
    pe: pe::PE<'_>,
    pe_data: &[u8],
    pdb: &mut Pdb,
) -> Result<Vec<(u32, String)>> {
    // Get the PE's name
    let pe_name = pe
        .export_data
        .as_ref()
        .ok_or_else(|| {
            WinDiffError::MissingExecutableExportInfo("missing export data".to_string())
        })?
        .name
        .ok_or_else(|| {
            WinDiffError::MissingExecutableExportInfo("missing binary name".to_string())
        })?;
    log::trace!("Extracting syscalls from {:?}", pe_name);

    // Handle ntoskrnl.exe
    if pe_name.eq_ignore_ascii_case("ntoskrnl.exe") {
        let symbols = pdb.extract_symbols_with_offset(false)?;
        return extract_ntoskrnl_syscalls(&pe, pe_data, &symbols);
    }
    // Handle win32k.sys
    if pe_name.eq_ignore_ascii_case("win32k.sys") {
        let symbols = pdb.extract_symbols_with_offset(false)?;
        return extract_win32k_syscalls(&pe, pe_data, &symbols);
    }

    // Handle user-mode binaries (i.e., ntdll.dll and win32u.dll)
    // Select syscall extraction implementation depending on the PE's target architecture
    extract_syscalls_from_user_binaries(&pe, pe_data)
}

fn extract_ntoskrnl_syscalls(
    pe: &pe::PE<'_>,
    pe_data: &[u8],
    symbols: &BTreeMap<u32, String>,
) -> Result<Vec<(u32, String)>> {
    //  Find the service table 'W32pServiceTable' and 'W32pServiceLimit'
    let service_table_info =
        find_service_table(pe, pe_data, symbols, "KiServiceTable", "KiServiceLimit")?;

    // Extract syscalls from the service table
    extract_syscalls_from_service_table(pe, pe_data, symbols, &service_table_info)
}

fn extract_win32k_syscalls(
    pe: &pe::PE<'_>,
    pe_data: &[u8],
    symbols: &BTreeMap<u32, String>,
) -> Result<Vec<(u32, String)>> {
    const WIN32K_SYSCALL_TABLE_ID: u32 = 0x1000;

    //  Find the service table 'W32pServiceTable' and 'W32pServiceLimit'
    let service_table_info =
        find_service_table(pe, pe_data, symbols, "W32pServiceTable", "W32pServiceLimit")?;

    // Extract syscalls from the service table
    extract_syscalls_from_service_table(pe, pe_data, symbols, &service_table_info)
        // Include win32k's table identifier in the syscall number
        .map(|mut syscalls| {
            syscalls
                .iter_mut()
                .for_each(|syscall: &mut (u32, String)| syscall.0 |= WIN32K_SYSCALL_TABLE_ID);

            syscalls
        })
}

/// Find a service table start offset and size given its
/// their symbols's names.
fn find_service_table(
    pe: &pe::PE<'_>,
    pe_data: &[u8],
    symbols: &BTreeMap<u32, String>,
    service_table_name: &str,
    service_table_size_name: &str,
) -> Result<(u32, u32)> {
    // Find the service table's offset and size
    let mut service_table_info = (0_u32, 0_u32);
    for (symbol_offset, symbol_name) in symbols {
        if symbol_name == service_table_name {
            service_table_info.0 = rva_to_offset(*symbol_offset as usize, pe)? as u32;
        } else if symbol_name == service_table_size_name {
            let service_limit_offset = rva_to_offset(*symbol_offset as usize, pe)?;
            let service_limit_bytes =
                &pe_data[service_limit_offset..service_limit_offset + std::mem::size_of::<u32>()];
            let service_limit = u32::from_le_bytes(service_limit_bytes.try_into()?);
            service_table_info.1 = service_limit;
        }

        if service_table_info.0 != 0 && service_table_info.1 != 0 {
            // Exit early if we've found what we were looking for
            return Ok(service_table_info);
        }
    }

    Err(WinDiffError::SystemServiceTableNotFoundError)
}

/// Extract (and symbolize) syscall list from a service table
fn extract_syscalls_from_service_table(
    pe: &pe::PE<'_>,
    pe_data: &[u8],
    symbols: &BTreeMap<u32, String>,
    service_table_info: &(u32, u32),
) -> Result<Vec<(u32, String)>> {
    // Determine the service table's content (RVAs vs VAs)
    let service_table_contains_rva: bool =
        does_service_table_contain_rva(pe.image_base as u64, pe_data, symbols, service_table_info);

    // Determine the size of elements in the service table
    let size_of_table_element = if service_table_contains_rva {
        std::mem::size_of::<u32>()
    } else {
        std::mem::size_of::<u64>()
    } as u32;
    // Walk through the service table
    let mut result = Vec::with_capacity(service_table_info.1 as usize);
    for syscall_id in 0..service_table_info.1 {
        let current_offset_in_table =
            (service_table_info.0 + size_of_table_element * syscall_id) as usize;
        let syscall_impl_offset_bytes = &pe_data
            [current_offset_in_table..current_offset_in_table + size_of_table_element as usize];

        // Parse service table element
        if service_table_contains_rva {
            // This service table contains relative virtual addresses
            let syscall_impl_rva = u32::from_le_bytes(syscall_impl_offset_bytes.try_into()?);
            let symbol_name = symbols
                .get(&syscall_impl_rva)
                .ok_or_else(|| WinDiffError::SystemServiceTableParsingError)?;
            result.push((syscall_id, symbol_name.clone()));
        } else {
            // This service table contains virtual addresses
            let syscall_impl_va = u64::from_le_bytes(syscall_impl_offset_bytes.try_into()?);
            let syscall_impl_rva = (syscall_impl_va - pe.image_base as u64) as u32;
            let symbol_name = symbols
                .get(&syscall_impl_rva)
                .ok_or_else(|| WinDiffError::SystemServiceTableParsingError)?;
            result.push((syscall_id, symbol_name.clone()));
        }
    }

    Ok(result)
}

/// Determine if the given service table seems to contain RVAs or VAs
///
/// The logic is pretty basic and simply checks if the first element seems to be a virtual address.
fn does_service_table_contain_rva(
    image_base: u64,
    pe_data: &[u8],
    symbols: &BTreeMap<u32, String>,
    service_table_info: &(u32, u32),
) -> bool {
    let current_offset_in_table = service_table_info.0 as usize;
    let syscall_impl_offset_bytes =
        &pe_data[current_offset_in_table..current_offset_in_table + std::mem::size_of::<u64>()];
    let syscall_impl_va = u64::from_le_bytes(
        syscall_impl_offset_bytes
            .try_into()
            .expect("invalid array (this is a bug)"),
    );
    let syscall_impl_rva = syscall_impl_va.saturating_sub(image_base) as u32;

    syscall_impl_rva == 0 || symbols.get(&syscall_impl_rva).is_none()
}

/// Convert an RVA to a file offset
fn rva_to_offset(rva: usize, pe: &pe::PE<'_>) -> Result<usize> {
    pe::utils::find_offset(
        rva,
        &pe.sections,
        pe.header
            .optional_header
            .ok_or_else(|| WinDiffError::MissingExecutableOptionalHeader)?
            .windows_fields
            .file_alignment,
        &pe::options::ParseOptions { resolve_rva: true },
    )
    .ok_or_else(|| WinDiffError::MissingExecutableOptionalHeader)
}

/// Handle user-mode binaries (i.e., ntdll.dll and win32u.dll)
///
/// Note(ergrelet): the current implementation is pretty fragile as it depends
/// on pattern matching.. However, this should be pretty fast and should be runnable
/// on a lot of PEs without costing too much compute power.
fn extract_syscalls_from_user_binaries(
    pe: &pe::PE<'_>,
    pe_data: &[u8],
) -> Result<Vec<(u32, String)>> {
    let extract_syscall_impl =
        if pe.header.coff_header.machine == OSArchitecture::Amd64.to_machine_type() {
            // AMD64
            extract_user_syscall_id_amd64
        } else if pe.header.coff_header.machine == OSArchitecture::Arm64.to_machine_type() {
            // ARM64
            extract_user_syscall_id_arm64
        } else {
            // Not supported
            return Err(WinDiffError::UnsupportedArchitecture);
        };

    let syscalls = pe
        .exports
        .iter()
        .enumerate()
        .filter_map(|(i, export)| {
            let export_name = if let Some(name) = export.name {
                name.to_string()
            } else {
                format!("Ordinal{}", i)
            };

            if let Some(export_offset) = export.offset {
                if let Some((syscall_id, export_name)) =
                    extract_syscall_impl(export_name, &pe_data[export_offset..])
                {
                    return Some((syscall_id, export_name));
                }
            }

            None
        })
        .collect();

    Ok(syscalls)
}

fn extract_user_syscall_id_amd64(export_name: String, export_data: &[u8]) -> Option<(u32, String)> {
    // We want to match the following stub:
    // mov r10, rcx
    // mov eax, IMM32 ; <- syscall id
    // test byte ptr ds:IMM32, 1
    // jnz short syscall_interrupt
    // syscall
    // ret
    // syscall_interrupt:
    // int 0x2E
    // ret
    const SYSCALL_STUB_ENTRY_BYTES: [u8; 4] = [0x4c, 0x8b, 0xd1, 0xb8];
    const SYSCALL_INST_BYTES: [u8; 2] = [0x0f, 0x05];
    const SYSCALL_INST_OFFSET_AMD64: usize = 0x12;

    if export_data[..SYSCALL_STUB_ENTRY_BYTES.len()] == SYSCALL_STUB_ENTRY_BYTES
        && export_data
            [SYSCALL_INST_OFFSET_AMD64..SYSCALL_INST_OFFSET_AMD64 + SYSCALL_INST_BYTES.len()]
            == SYSCALL_INST_BYTES
    {
        let syscall_id_offset = SYSCALL_STUB_ENTRY_BYTES.len();
        let syscall_id_bytes =
            &export_data[syscall_id_offset..syscall_id_offset + std::mem::size_of::<u32>()];
        let syscall_id = u32::from_le_bytes(syscall_id_bytes.try_into().ok()?);

        Some((syscall_id, export_name))
    } else {
        None
    }
}

fn extract_user_syscall_id_arm64(export_name: String, export_data: &[u8]) -> Option<(u32, String)> {
    // We want to match the following stub:
    // SVC IMM16 ; <- syscall ID
    // RET

    // https://developer.arm.com/documentation/ddi0602/2022-06/Base-Instructions/SVC--Supervisor-Call-
    let first_inst_bytes = &export_data[..4];
    let first_inst_u32 = u32::from_le_bytes(first_inst_bytes.try_into().ok()?);
    let is_first_inst_svc = (first_inst_u32 & 0xF == 1) && (first_inst_u32 >> 21 == 0x6a0);
    if is_first_inst_svc {
        const RET_INST_ARM64: [u8; 4] = [0xc0, 0x03, 0x5f, 0xd6];
        let is_second_inst_ret = export_data[4..8] == RET_INST_ARM64;
        if is_second_inst_ret {
            // Extract IMM16
            let syscall_id = (first_inst_u32 >> 5) & 0xFFFF;

            return Some((syscall_id, export_name));
        }
    }

    None
}
