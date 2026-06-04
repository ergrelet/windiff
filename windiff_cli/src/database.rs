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
    /// Each map associates an OS path suffix ("version_update_architecture") to
    /// the set of binaries that produced non-empty data of the corresponding
    /// kind for that OS version. Used by the frontend to filter the binary
    /// dropdown on the Debug Symbols, Modules and (Reconstructed) Types tabs.
    /// These are PDB-derived and independent: a public PDB commonly has symbols
    /// but no private types, so a binary can appear in one map and not another.
    pub binaries_with_symbols: BTreeMap<String, BTreeSet<String>>,
    pub binaries_with_modules: BTreeMap<String, BTreeSet<String>>,
    pub binaries_with_types: BTreeMap<String, BTreeSet<String>>,
}

/// Per-OS-version sets of binaries that produced non-empty PDB-derived data,
/// grouped by information kind. Built while generating databases and serialized
/// into the [`DatabaseIndex`].
#[derive(Debug, Default)]
pub struct BinariesWithInfo {
    pub symbols: BTreeMap<String, BTreeSet<String>>,
    pub modules: BTreeMap<String, BTreeSet<String>>,
    pub types: BTreeMap<String, BTreeSet<String>>,
}

impl BinariesWithInfo {
    /// Records, for a single database, which information kinds were non-empty.
    fn record(&mut self, os_suffix: &str, binary_name: &str, presence: ExtractedInfoPresence) {
        let insert = |map: &mut BTreeMap<String, BTreeSet<String>>| {
            map.entry(os_suffix.to_owned())
                .or_default()
                .insert(binary_name.to_owned());
        };
        if presence.has_symbols {
            insert(&mut self.symbols);
        }
        if presence.has_modules {
            insert(&mut self.modules);
        }
        if presence.has_types {
            insert(&mut self.types);
        }
    }

    /// Merges another instance into this one (used to accumulate across the
    /// per-binary passes of low-storage mode).
    pub fn merge(&mut self, other: BinariesWithInfo) {
        for (suffix, binaries) in other.symbols {
            self.symbols.entry(suffix).or_default().extend(binaries);
        }
        for (suffix, binaries) in other.modules {
            self.modules.entry(suffix).or_default().extend(binaries);
        }
        for (suffix, binaries) in other.types {
            self.types.entry(suffix).or_default().extend(binaries);
        }
    }
}

/// Which information kinds a generated database ended up containing.
#[derive(Debug, Clone, Copy, Default)]
struct ExtractedInfoPresence {
    has_symbols: bool,
    has_modules: bool,
    has_types: bool,
}

/// A version of Windows, defined as a triplet
#[derive(Serialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct OSVersion {
    pub version: String,
    pub update: String,
    pub build_number: String,
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
) -> Result<BinariesWithInfo> {
    const CONCURRENT_DB_GENERATIONS: usize = 16;

    let windiff_app = WinDiffApp::new()?;

    // Create directory tree if needed
    tokio::fs::create_dir_all(output_directory).await?;
    // Generate databases concurrently
    let binaries_with_info = futures::stream::iter(downloaded_binaries.iter().map(
        |(downloaded_pe, pdb_path)| async {
            let windiff_app = &windiff_app;
            let presence = generate_database_for_pe_version(
                cfg,
                windiff_app,
                downloaded_pe,
                pdb_path,
                output_directory,
            )
            .await?;
            // Report which information kinds this version produced, alongside
            // the OS suffix and binary name they belong to.
            Ok((
                os_path_suffix(downloaded_pe),
                downloaded_pe.original_name.clone(),
                presence,
            ))
        },
    ))
    .buffer_unordered(CONCURRENT_DB_GENERATIONS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<(String, String, ExtractedInfoPresence)>| async { result.ok() })
    // Fold into per-kind maps of OS suffix -> binaries with that information
    .fold(
        BinariesWithInfo::default(),
        |mut acc, (os_suffix, binary_name, presence)| async move {
            acc.record(&os_suffix, &binary_name, presence);
            acc
        },
    )
    .await;

    if generate_index {
        // Generate database index
        generate_database_index(downloaded_binaries, &binaries_with_info, output_directory).await?;
    }

    Ok(binaries_with_info)
}

/// Builds the "version_update_architecture" suffix used for database file names
/// and as the key shared with the frontend's `osVersionToPathSuffix`.
fn os_path_suffix(pe_version: &DownloadedPEVersion) -> String {
    format!(
        "{}_{}_{}",
        pe_version.os_version,
        pe_version.os_update,
        pe_version.architecture.to_str()
    )
}

async fn generate_database_for_pe_version(
    cfg: &WinDiffConfiguration,
    windiff_app: &WinDiffApp,
    pe_version: &DownloadedPEVersion,
    pdb_path: &Option<PathBuf>,
    output_directory: &Path,
) -> Result<ExtractedInfoPresence> {
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
            "{}_{}.json.gz",
            pe_version.original_name,
            os_path_suffix(pe_version)
        ));

        let presence = generate_database_for_pe(
            windiff_app,
            pe_version,
            pe,
            &file_data,
            pdb,
            &binary_desc.extracted_information,
            output_file,
        )
        .await?;

        Ok(presence)
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
) -> Result<ExtractedInfoPresence> {
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
            database.symbols = pdb.extract_symbols(true)?;
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
            database.syscalls = extract_syscalls(pe, pe_data, &mut pdb)?
                .into_iter()
                .collect();
        }
    }

    let presence = ExtractedInfoPresence {
        has_symbols: !database.symbols.is_empty(),
        has_modules: !database.modules.is_empty(),
        has_types: !database.types.is_empty(),
    };

    // Serialize database
    let json_data = serde_json::to_vec(&database)?;

    // Create file and copy compressed JSON data
    let output_file = File::create(output_path.as_ref()).await?;
    let mut gz = GzipEncoder::new(output_file);
    gz.write_all(json_data.as_slice()).await?;
    gz.shutdown().await?;

    Ok(presence)
}

pub async fn generate_database_index(
    downloaded_binaries: &[(DownloadedPEVersion, Option<PathBuf>)],
    binaries_with_info: &BinariesWithInfo,
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
                build_number: pe_version.os_build_number.clone(),
                architecture: pe_version.architecture.to_str().to_string(),
            })
            .collect(),
        // Map configuration's binaries
        binaries: downloaded_binaries
            .iter()
            .map(|(pe_version, _)| pe_version.original_name.clone())
            .collect(),
        binaries_with_symbols: binaries_with_info.symbols.clone(),
        binaries_with_modules: binaries_with_info.modules.clone(),
        binaries_with_types: binaries_with_info.types.clone(),
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
