mod cli;
mod configuration;
mod database;
mod error;
mod pdb;
mod resym_frontend;
mod winbindex;

use std::path::{Path, PathBuf};

use async_compression::tokio::write::GzipEncoder;
use configuration::{BinaryExtractedInformation, BinaryExtractedInformationFlags, OSDescription};
use database::{BinaryDatabase, DatabaseIndex, OSVersion};
use error::WinDiffError;
use futures::stream::StreamExt;
use goblin::{pe, Object};
use structopt::StructOpt;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncWriteExt},
};
use winbindex::DownloadedPEVersion;

use crate::{cli::WinDiffOpt, configuration::WinDiffConfiguration, error::Result, pdb::Pdb};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line options
    let opt = WinDiffOpt::from_args();
    println!("Using configuration file: {:?}", opt.configuration);

    // Parse configuration file
    let cfg = WinDiffConfiguration::from_file(&opt.configuration).await?;
    // Download requested PEs
    let tmp_directory = tempdir::TempDir::new("windiff")?;
    let output_directory = tmp_directory.path();
    let downloaded_pes = download_binaries(&cfg, output_directory).await?;
    println!("PEs downloaded!");
    // Download PDBs
    let downloaded_binaries = download_pdbs(downloaded_pes, output_directory).await;
    println!("PDBs downloaded!");

    // Extract information from PEs
    generate_databases(&cfg, &downloaded_binaries, &opt.output_directory).await?;
    println!(
        "Databases have been generated at {:?}",
        opt.output_directory
    );

    Ok(())
}

async fn download_binaries(
    cfg: &WinDiffConfiguration,
    output_directory: &Path,
) -> Result<Vec<DownloadedPEVersion>> {
    // Note(ergrelet): arbitrarily defined value
    const CONCURRENT_PE_DOWNLOADS: usize = 16;

    // Fetch all binaries concurrently and fold results into a single `Vec`
    let result: Vec<DownloadedPEVersion> =
        futures::stream::iter(cfg.binaries.keys().map(|binary_name| async move {
            println!("Fetching '{}' binaries ...", binary_name);

            // Retrieve the index file for that PE file
            let pe_index = winbindex::get_remote_index_for_pe(binary_name).await?;
            // Download all requested versions of this PE file
            let downloaded_pes: Vec<DownloadedPEVersion> =
                download_pe_versions(&cfg.oses, &pe_index, binary_name, output_directory).await;

            Ok(downloaded_pes)
        }))
        .buffer_unordered(CONCURRENT_PE_DOWNLOADS)
        .collect::<Vec<Result<Vec<DownloadedPEVersion>>>>()
        .await
        .into_iter()
        // Fold results for all binaries into a single `Vec`
        .fold(vec![], |mut acc, elem| {
            if let Ok(mut elem) = elem {
                acc.append(&mut elem);
            }
            acc
        });

    Ok(result)
}

async fn download_pe_versions(
    os_descriptions: &[OSDescription],
    pe_index: &serde_json::Value,
    binary_name: &str,
    output_directory: &Path,
) -> Vec<DownloadedPEVersion> {
    const CONCURRENT_PE_DOWNLOADS: usize = 16;

    // Download all requested versions concurrently
    futures::stream::iter(os_descriptions.iter().map(|os_desc| async {
        winbindex::download_pe_version(
            pe_index,
            binary_name,
            &os_desc.version,
            &os_desc.update,
            &os_desc.architecture,
            output_directory,
        )
        .await
    }))
    .buffer_unordered(CONCURRENT_PE_DOWNLOADS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result| async { result.ok() })
    .collect()
    .await
}

async fn generate_databases(
    cfg: &WinDiffConfiguration,
    downloaded_binaries: &[(DownloadedPEVersion, Option<PathBuf>)],
    output_directory: &Path,
) -> Result<()> {
    const CONCURRENT_DB_GENERATIONS: usize = 16;

    // Create directory tree if needed
    tokio::fs::create_dir_all(output_directory).await?;
    // Generate databases concurrently
    futures::stream::iter(
        downloaded_binaries
            .iter()
            .map(|(downloaded_pe, pdb_path)| async {
                generate_database_for_pe_version(cfg, downloaded_pe, pdb_path, output_directory)
                    .await?;
                Ok(())
            }),
    )
    .buffer_unordered(CONCURRENT_DB_GENERATIONS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<()>| async { result.ok() })
    .collect::<()>()
    .await;

    // Generate database index
    generate_index(cfg, output_directory).await?;

    Ok(())
}

async fn generate_database_for_pe_version(
    cfg: &WinDiffConfiguration,
    pe_version: &DownloadedPEVersion,
    pdb_path: &Option<PathBuf>,
    output_directory: &Path,
) -> Result<()> {
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
        let binary_desc = cfg.binaries.get(&pe_version.original_name).unwrap();
        let output_file = output_directory.join(format!(
            "{}_{}_{}_{}.json.gz",
            pe_version.original_name,
            pe_version.os_version,
            pe_version.os_update,
            pe_version.architecture.to_str()
        ));

        generate_database_for_pe(
            pe_version,
            pe,
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
    pe_version: &DownloadedPEVersion,
    pe: pe::PE<'_>,
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
            database.types = pdb.extract_types()?;
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

async fn generate_index(cfg: &WinDiffConfiguration, output_directory: &Path) -> Result<()> {
    let index = DatabaseIndex {
        // Map configuration's OSes
        oses: cfg
            .oses
            .iter()
            .map(|os| OSVersion {
                version: os.version.clone(),
                update: os.update.clone(),
                architecture: os.architecture.to_str().to_string(),
            })
            .collect(),
        // Map configuration's binaries
        binaries: cfg.binaries.keys().cloned().collect(),
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

async fn download_pdbs(
    downloaded_pes: Vec<DownloadedPEVersion>,
    output_directory: &Path,
) -> Vec<(DownloadedPEVersion, Option<PathBuf>)> {
    const CONCURRENT_PDB_DOWNLOADS: usize = 16;

    // Download all requested versions concurrently
    futures::stream::iter(downloaded_pes.into_iter().map(|pe_version| async move {
        let pdb_path_opt = pdb::download_pdb_for_pe(&pe_version.path, output_directory)
            .await
            .ok();

        Ok((pe_version, pdb_path_opt))
    }))
    .buffer_unordered(CONCURRENT_PDB_DOWNLOADS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<(DownloadedPEVersion, Option<PathBuf>)>| async { result.ok() })
    .collect()
    .await
}
