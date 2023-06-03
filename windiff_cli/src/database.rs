use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use async_compression::tokio::write::GzipEncoder;
use futures::StreamExt;
use goblin::{pe, Object};
use serde::Serialize;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    configuration::{
        BinaryExtractedInformation, BinaryExtractedInformationFlags, WinDiffConfiguration,
    },
    error::{Result, WinDiffError},
    pdb::Pdb,
    resym_frontend::WinDiffApp,
    syscalls::extract_syscalls,
    winbindex::DownloadedPEVersion,
};

/// Database index.
/// This contains the list of all OS versions and binaries for which we have
/// generated databases.
#[derive(Serialize, Debug, Default)]
pub struct DatabaseIndex {
    pub oses: BTreeSet<OSVersion>,
    pub binaries: BTreeSet<String>,
}

/// A version of Windows, defined as a triplet
#[derive(Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OSVersion {
    pub version: String,
    pub update: String,
    pub architecture: String,
}

#[derive(Serialize, Debug, Default)]
pub struct BinaryDatabase {
    pub metadata: BinaryMetadata,
    /// Exported symbols
    pub exports: BTreeSet<String>,
    /// Debug symbols
    pub symbols: BTreeSet<String>,
    /// Compiled modules
    pub modules: BTreeSet<String>,
    /// Debug types (type identifier -> reconstructed type)
    pub types: BTreeMap<String, String>,
    // Syscalls detected in the binary (for relevant executables)
    pub syscalls: BTreeMap<u32, String>,
}

#[derive(Serialize, Debug, Default)]
pub struct BinaryMetadata {
    pub name: String,
    pub version: String,
    pub architecture: String,
}

pub async fn generate_databases(
    cfg: &WinDiffConfiguration,
    downloaded_binaries: &[(DownloadedPEVersion, Option<PathBuf>)],
    generate_index: bool,
    output_directory: &Path,
) -> Result<()> {
    const CONCURRENT_DB_GENERATIONS: usize = 16;

    let windiff_app = WinDiffApp::new()?;

    // Create directory tree if needed
    tokio::fs::create_dir_all(output_directory).await?;
    // Generate databases concurrently
    futures::stream::iter(
        downloaded_binaries
            .iter()
            .map(|(downloaded_pe, pdb_path)| async {
                let windiff_app = &windiff_app;
                generate_database_for_pe_version(
                    cfg,
                    windiff_app,
                    downloaded_pe,
                    pdb_path,
                    output_directory,
                )
                .await?;
                Ok(())
            }),
    )
    .buffer_unordered(CONCURRENT_DB_GENERATIONS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<()>| async { result.ok() })
    .collect::<()>()
    .await;

    if generate_index {
        // Generate database index
        generate_database_index(downloaded_binaries, output_directory).await?;
    }

    Ok(())
}

async fn generate_database_for_pe_version(
    cfg: &WinDiffConfiguration,
    windiff_app: &WinDiffApp,
    pe_version: &DownloadedPEVersion,
    pdb_path: &Option<PathBuf>,
    output_directory: &Path,
) -> Result<()> {
    log::trace!(
        "Generating database for PE '{}_{}_{}_{}'",
        pe_version.original_name,
        pe_version.os_version,
        pe_version.os_update,
        pe_version.architecture.to_str()
    );

    // Open file
    let mut file = File::open(&pe_version.path).await?;

    // Read file
    let mut file_data = vec![];
    let _read_bytes = file.read_to_end(&mut file_data).await?;

    // Parse PE and generate database
    if let Object::PE(pe) = Object::parse(&file_data)? {
        let pdb = if let Some(pdb_path) = pdb_path {
            Some(Pdb::new(pdb_path.clone())?)
        } else {
            None
        };
        let binary_desc = cfg
            .binaries
            .get(&pe_version.original_name)
            .ok_or_else(|| WinDiffError::FileNotFoundInConfiguration)?;
        let output_file = output_directory.join(format!(
            "{}_{}_{}_{}.json.gz",
            pe_version.original_name,
            pe_version.os_version,
            pe_version.os_update,
            pe_version.architecture.to_str()
        ));

        generate_database_for_pe(
            windiff_app,
            pe_version,
            pe,
            &file_data,
            pdb,
            &binary_desc.extracted_information,
            output_file,
        )
        .await?;

        Ok(())
    } else {
        Err(WinDiffError::UnsupportedExecutableFormat)
    }
}

async fn generate_database_for_pe(
    windiff_app: &WinDiffApp,
    pe_version: &DownloadedPEVersion,
    pe: pe::PE<'_>,
    pe_data: &[u8],
    pdb: Option<Pdb<'_>>,
    extracted_information: &BinaryExtractedInformation,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let mut database = BinaryDatabase::default();
    // Metadata
    database.metadata.name = pe_version.original_name.clone();
    database.metadata.version = pe_version.pe_version.clone();
    database.metadata.architecture = pe_version.architecture.to_str().to_string();

    // Extract exports
    if extracted_information.contains(BinaryExtractedInformationFlags::Exports) {
        database.exports = pe
            .exports
            .iter()
            .filter_map(|exp| Some(exp.name?.to_string()))
            .collect();
    }
    // Extract information from the PDB file if available
    if let Some(mut pdb) = pdb {
        // Extract debug symbols
        if extracted_information.contains(BinaryExtractedInformationFlags::DebugSymbols) {
            database.symbols = pdb.extract_symbols()?;
        }
        // Extract compiled modules
        if extracted_information.contains(BinaryExtractedInformationFlags::Modules) {
            database.modules = pdb.extract_modules()?;
        }
        // Extract debug types
        if extracted_information.contains(BinaryExtractedInformationFlags::Types) {
            database.types = windiff_app
                .extract_types_from_pdb(&pdb.file_path)?
                .into_iter()
                .collect();
        }
        // Extract syscalls
        if extracted_information.contains(BinaryExtractedInformationFlags::Syscalls) {
            database.syscalls = extract_syscalls(pe, pe_data)?.into_iter().collect();
        }
    }

    // Serialize database
    let json_data = serde_json::to_vec(&database)?;

    // Create file and copy compressed JSON data
    let output_file = File::create(output_path.as_ref()).await?;
    let mut gz = GzipEncoder::new(output_file);
    gz.write_all(json_data.as_slice()).await?;
    gz.shutdown().await?;

    Ok(())
}

pub async fn generate_database_index(
    downloaded_binaries: &[(DownloadedPEVersion, Option<PathBuf>)],
    output_directory: &Path,
) -> Result<()> {
    log::trace!("Generating database index");

    let index = DatabaseIndex {
        // Map configuration's OSes
        oses: downloaded_binaries
            .iter()
            .map(|(pe_version, _)| OSVersion {
                version: pe_version.os_version.clone(),
                update: pe_version.os_update.clone(),
                architecture: pe_version.architecture.to_str().to_string(),
            })
            .collect(),
        // Map configuration's binaries
        binaries: downloaded_binaries
            .iter()
            .map(|(pe_version, _)| pe_version.original_name.clone())
            .collect(),
    };

    // Serialize index
    let json_data = serde_json::to_vec(&index)?;

    // Create file and copy compressed JSON data
    let output_path = output_directory.join("index.json.gz");
    let output_file = File::create(output_path).await?;
    let mut gz = GzipEncoder::new(output_file);
    gz.write_all(json_data.as_slice()).await?;
    gz.shutdown().await?;

    Ok(())
}
