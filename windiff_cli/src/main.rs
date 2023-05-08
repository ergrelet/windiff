mod cli;
mod configuration;
mod error;
mod winbindex;

use std::path::Path;

use configuration::OSDescription;
use futures::stream::StreamExt;
use structopt::StructOpt;
use winbindex::DownloadedPEVersion;

use crate::{cli::WinDiffOpt, configuration::WinDiffConfiguration, error::Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line options
    let opt = WinDiffOpt::from_args();
    println!("Using configuration file: {:?}", opt.configuration);

    // Parse configuration file
    let cfg = WinDiffConfiguration::from_file(&opt.configuration)?;
    // Download requested PEs
    let tmp_directory = tempdir::TempDir::new("windiff")?;
    let output_directory = tmp_directory.path();
    let downloaded_pes = download_binaries(&cfg, output_directory).await?;
    println!("{:?}", downloaded_pes);

    println!("PEs downloaded!");

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
        futures::stream::iter(cfg.binaries.iter().map(|binary_desc| async move {
            println!("Fetching '{}' binaries ...", binary_desc.name);

            // Retrieve the index file for that PE file
            let pe_index = winbindex::get_remote_index_for_pe(&binary_desc.name).await?;
            // Download all requested versions of this PE file
            let downloaded_pes: Vec<DownloadedPEVersion> =
                download_pe_versions(&cfg.oses, &pe_index, &binary_desc.name, output_directory)
                    .await;

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
