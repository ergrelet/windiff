use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    str::FromStr,
};

use futures::StreamExt;
use goblin::{pe::debug::DebugData, Object};
use pdb::FallibleIterator;
use tokio::{fs::File, io::AsyncReadExt};

use crate::error::{Result, WinDiffError};

const MSDL_FILE_DOWNLOAD_BASE_URL: &str = "https://msdl.microsoft.com/download/symbols/";

pub struct Pdb<'p> {
    pub file_path: PathBuf,
    pdb: pdb::PDB<'p, std::fs::File>,
}

impl<'p> Pdb<'p> {
    pub fn new(file_path: PathBuf) -> Result<Self> {
        let pdb_file = std::fs::File::open(&file_path)?;
        let pdb = pdb::PDB::open(pdb_file)?;

        Ok(Self { file_path, pdb })
    }

    pub fn extract_symbols(&mut self) -> Result<BTreeSet<String>> {
        log::trace!("Extracting symbols from {:?}", self.file_path);

        let mut symbols = BTreeSet::new();

        // Global symbols
        let symbol_table = self.pdb.global_symbols()?;
        symbols.append(&mut walk_symbols(symbol_table.iter())?);

        // Modules' private symbols
        let dbi = self.pdb.debug_information()?;
        let mut modules = dbi.modules()?;
        while let Some(module) = modules.next()? {
            let info = match self.pdb.module_info(&module)? {
                Some(info) => info,
                None => {
                    continue;
                }
            };

            symbols.append(&mut walk_symbols(info.symbols()?)?);
        }

        Ok(symbols)
    }

    pub fn extract_modules(&mut self) -> Result<BTreeSet<String>> {
        log::trace!("Extracting modules from {:?}", self.file_path);

        let mut result = BTreeSet::new();

        // Modules' private symbols
        let dbi = self.pdb.debug_information()?;
        let mut modules = dbi.modules()?;
        while let Some(module) = modules.next()? {
            result.insert(module.module_name().to_string());
        }

        Ok(result)
    }
}

pub async fn download_pdb_for_pe(pe_path: &Path, output_directory: &Path) -> Result<PathBuf> {
    // Open file
    let mut file = File::open(&pe_path).await?;

    // Read file
    let mut file_data = vec![];
    let _read_bytes = file.read_to_end(&mut file_data).await?;

    // Parse PE and download corresponding PDB
    if let Object::PE(pe) = Object::parse(&file_data)? {
        // Generate PDB url
        let pe_dbg_data = pe
            .debug_data
            .ok_or_else(|| WinDiffError::MissingExecutableDebugInfo("DebugData".to_string()))?;
        let pdb_download_url = generate_pdb_download_url(&pe_dbg_data)?;
        log::debug!("Found download URL for PDB: {}", pdb_download_url.as_str());

        // Download PDB
        let output_pdb_path = format!(
            "{}.pdb",
            pe_path
                .file_stem()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
        );
        let output_file_path = output_directory.join(output_pdb_path);
        download_file(pdb_download_url, &output_file_path).await?;

        Ok(output_file_path)
    } else {
        Err(WinDiffError::UnsupportedExecutableFormat)
    }
}

fn generate_pdb_download_url(debug_data: &DebugData) -> Result<reqwest::Url> {
    let base_url = reqwest::Url::from_str(MSDL_FILE_DOWNLOAD_BASE_URL)?;

    let code_view_info = debug_data.codeview_pdb70_debug_info.ok_or_else(|| {
        WinDiffError::MissingExecutableDebugInfo("CodeView debug info".to_string())
    })?;
    let pdb_guid = debug_data
        .guid()
        .ok_or_else(|| WinDiffError::MissingExecutableDebugInfo("PDB GUID".to_string()))?;
    let pdb_age = code_view_info.age;
    // Convert PDB name to UTF-8 and remove trailing zeroes
    let pdb_name = std::str::from_utf8(code_view_info.filename)?.trim_end_matches(char::from(0));

    // “%s\%s\%s%x\%s” % (serverPath, pdbName, guid, age, pdbName)
    // https://randomascii.wordpress.com/2013/03/09/symbols-the-microsoft-way/
    Ok(base_url.join(
        format!(
            "{}/{}{:x}/{}",
            pdb_name,
            guid_to_str(&pdb_guid)?,
            pdb_age,
            pdb_name
        )
        .as_str(),
    )?)
}

fn guid_to_str(guid: &[u8; 16]) -> Result<String> {
    // 4 bytes -> u32 (BE)
    let (int_bytes, rest) = guid.split_at(std::mem::size_of::<u32>());
    let first_part = u32::from_le_bytes(int_bytes.try_into()?);
    // 2 bytes -> u16 (LE)
    let (int_bytes, rest) = rest.split_at(std::mem::size_of::<u16>());
    let second_part = u16::from_le_bytes(int_bytes.try_into()?);
    // 2 bytes -> u16 (LE)
    let (int_bytes, rest) = rest.split_at(std::mem::size_of::<u16>());
    let third_part = u16::from_le_bytes(int_bytes.try_into()?);
    // 2 bytes -> u16 (BE)
    let (int_bytes, rest) = rest.split_at(std::mem::size_of::<u16>());
    let fourth_part = u16::from_be_bytes(int_bytes.try_into()?);
    // 6 bytes
    let last_part = hex::encode(rest);

    Ok(format!(
        "{:08X}{:04X}{:04X}{:04X}{}",
        first_part, second_part, third_part, fourth_part, last_part
    ))
}

async fn download_file(url: reqwest::Url, output_file_path: &Path) -> Result<()> {
    // Get PE file and write its content to a file
    let http_response = reqwest::get(url).await?.error_for_status()?;
    let mut output_file = File::create(&output_file_path).await?;
    let mut byte_stream = http_response.bytes_stream();
    while let Some(item) = byte_stream.next().await {
        tokio::io::copy(&mut item?.as_ref(), &mut output_file).await?;
    }

    Ok(())
}

fn walk_symbols(mut symbols: pdb::SymbolIter<'_>) -> Result<BTreeSet<String>> {
    let mut result = BTreeSet::new();
    while let Some(symbol) = symbols.next()? {
        if let Ok(value) = dump_symbol(&symbol) {
            result.insert(value);
        }
    }

    Ok(result)
}

fn dump_symbol(symbol: &pdb::Symbol<'_>) -> Result<String> {
    match symbol.parse()? {
        // Public symbols?
        pdb::SymbolData::Public(data) => Ok(if data.function {
            format!("{}()", data.name)
        } else {
            data.name.to_string().to_string()
        }),
        // Global variables
        pdb::SymbolData::Data(data) => Ok(data.name.to_string().to_string()),
        // Functions and methods
        pdb::SymbolData::Procedure(data) => Ok(format!("{}()", data.name)),
        _ => {
            // ignore everything else
            Err(WinDiffError::UnsupportedExecutableFormat)
        }
    }
}
