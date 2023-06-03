use goblin::pe;

use crate::{
    configuration::OSArchitecture,
    error::{Result, WinDiffError},
};

/// Extract syscalls found in a given PE.
///
/// Note(ergrelet): the current implementation is pretty fragile as it depends
/// on pattern matching and only supports userland binaries (i.e., ntdll.dll
/// and win32u.dll). However, this should be pretty fast and should be runnable
/// on a lot of PEs without costing too much compute power.
pub fn extract_syscalls(pe: pe::PE<'_>, pe_data: &[u8]) -> Result<Vec<(u32, String)>> {
    // Select syscall extraction implementation depending on the PE's target architecture
    let extract_syscall_impl =
        if pe.header.coff_header.machine == OSArchitecture::Amd64.to_machine_type() {
            // AMD64
            extract_syscall_id_amd64
        } else if pe.header.coff_header.machine == OSArchitecture::Arm64.to_machine_type() {
            // ARM64
            extract_syscall_id_arm64
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
                if let Some(syscall_id) = extract_syscall_impl(&pe_data[export_offset..]) {
                    return Some((syscall_id, export_name));
                }
            }

            None
        })
        .collect();

    Ok(syscalls)
}

fn extract_syscall_id_amd64(export_data: &[u8]) -> Option<u32> {
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

        Some(syscall_id)
    } else {
        None
    }
}

fn extract_syscall_id_arm64(export_data: &[u8]) -> Option<u32> {
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

            return Some(syscall_id);
        }
    }

    None
}
