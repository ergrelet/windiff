mod cli;
mod configuration;
mod database;
mod error;
mod winbindex;

use std::path::Path;

use configuration::{BinaryExtractedInformation, BinaryExtractedInformationFlags, OSDescription};
use database::BinaryDatabase;
use futures::stream::StreamExt;
use goblin::{pe, Object};
use structopt::StructOpt;
use tokio::{fs::File, io::AsyncReadExt};
use winbindex::DownloadedPEVersion;

use crate::{cli::WinDiffOpt, configuration::WinDiffConfiguration, error::Result};

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

    // Extract information from PEs
    generate_databases(&cfg, &downloaded_pes, &opt.output_directory).await?;
    println!("Databases have generated at {:?}", opt.output_directory);

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
    downloaded_pes: &[DownloadedPEVersion],
    output_directory: &Path,
) -> Result<()> {
    const CONCURRENT_DB_GENERATIONS: usize = 16;

    // Create directory tree if needed
    tokio::fs::create_dir_all(output_directory).await?;
    // Generate databases concurrently
    futures::stream::iter(downloaded_pes.iter().map(|downloaded_pe| async {
        generate_database_for_pe_version(cfg, downloaded_pe, output_directory).await?;
        Ok(())
    }))
    .buffer_unordered(CONCURRENT_DB_GENERATIONS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<()>| async { result.ok() })
    .collect::<()>()
    .await;

    Ok(())
}

async fn generate_database_for_pe_version(
    cfg: &WinDiffConfiguration,
    pe_version: &DownloadedPEVersion,
    output_directory: &Path,
) -> Result<()> {
    // Open file
    let mut file = File::open(&pe_version.path).await?;

    // Read file
    let mut file_data = vec![];
    let _read_bytes = file.read_to_end(&mut file_data).await?;

    // Parse PE and generate database
    if let Object::PE(pe) = Object::parse(&file_data)? {
        let output_file = output_directory.join(format!(
            "{}_{}_{}_{}.json",
            pe_version.original_name,
            pe_version.os_version,
            pe_version.os_update,
            pe_version.architecture.to_str()
        ));
        let binary_desc = cfg.binaries.get(&pe_version.original_name).unwrap();
        generate_database_for_pe(
            pe_version,
            pe,
            &binary_desc.extracted_information,
            output_file,
        )
        .await?;
    } else {
        println!("unsupported format");
    }

    Ok(())
}

async fn generate_database_for_pe(
    pe_version: &DownloadedPEVersion,
    pe: pe::PE<'_>,
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

    // Serialize database
    let json_data = serde_json::to_vec(&database)?;

    // Create file and copy JSON data
    let mut output_file = File::create(output_path.as_ref()).await?;
    tokio::io::copy(&mut json_data.as_slice(), &mut output_file).await?;

    Ok(())
}
