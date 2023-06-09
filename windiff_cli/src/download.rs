use std::path::{Path, PathBuf};

use futures::stream::StreamExt;

use crate::{
    configuration::{OSDescription, WinDiffConfiguration},
    error::Result,
    pdb,
    winbindex::{self, DownloadedPEVersion},
};

pub async fn download_all_binaries(
    cfg: &WinDiffConfiguration,
    output_directory: &Path,
    concurrent_downloads: usize,
) -> Result<Vec<DownloadedPEVersion>> {
    // Fetch all binaries concurrently and fold results into a single `Vec`
    let result: Vec<DownloadedPEVersion> =
        futures::stream::iter(cfg.binaries.keys().map(|binary_name| async move {
            log::trace!("Fetching '{}' binaries ...", binary_name);

            download_single_binary(
                binary_name,
                &cfg.oses,
                output_directory,
                concurrent_downloads,
            )
            .await
        }))
        .buffer_unordered(concurrent_downloads)
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

pub async fn download_single_binary(
    binary_name: &str,
    os_descriptions: &[OSDescription],
    output_directory: &Path,
    concurrent_downloads: usize,
) -> Result<Vec<DownloadedPEVersion>> {
    // Retrieve the index file for that PE file
    let pe_index = winbindex::get_remote_index_for_pe(binary_name).await?;

    // Download all requested versions of this PE file
    Ok(download_pe_versions(
        os_descriptions,
        &pe_index,
        binary_name,
        output_directory,
        concurrent_downloads,
    )
    .await)
}

async fn download_pe_versions(
    os_descriptions: &[OSDescription],
    pe_index: &serde_json::Value,
    binary_name: &str,
    output_directory: &Path,
    concurrent_downloads: usize,
) -> Vec<DownloadedPEVersion> {
    // Download all requested versions concurrently
    futures::stream::iter(os_descriptions.iter().map(|os_desc| async {
        let download_result = winbindex::download_pe_version(
            pe_index,
            binary_name,
            &os_desc.version,
            &os_desc.update,
            &os_desc.architecture,
            output_directory,
        )
        .await;
        if let Err(err) = &download_result {
            log::warn!(
                "Failed to download PE '{}' (version '{}-{}-{}'): {}",
                binary_name,
                &os_desc.version,
                &os_desc.update,
                os_desc.architecture.to_str(),
                err,
            );
        }

        download_result
    }))
    .buffer_unordered(concurrent_downloads)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result| async { result.ok() })
    .collect()
    .await
}

pub async fn download_all_pdbs(
    downloaded_pes: Vec<DownloadedPEVersion>,
    output_directory: &Path,
    concurrent_downloads: usize,
) -> Vec<(DownloadedPEVersion, Option<PathBuf>)> {
    // Download all requested versions concurrently
    futures::stream::iter(
        downloaded_pes.into_iter().map(|pe_version| async move {
            download_single_pdb(pe_version, output_directory).await
        }),
    )
    .buffer_unordered(concurrent_downloads)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<(DownloadedPEVersion, Option<PathBuf>)>| async { result.ok() })
    .collect()
    .await
}

async fn download_single_pdb(
    pe_version: DownloadedPEVersion,
    output_directory: &Path,
) -> Result<(DownloadedPEVersion, Option<PathBuf>)> {
    let pdb_path_opt: Option<PathBuf> =
        match pdb::download_pdb_for_pe(&pe_version.path, output_directory).await {
            Ok(pdb_path) => Some(pdb_path),
            Err(err) => {
                log::warn!(
                    "Failed to download PDB for PE '{}' (version '{}-{}'): {}",
                    pe_version.original_name,
                    pe_version.os_version,
                    pe_version.os_update,
                    err
                );
                None
            }
        };

    Ok((pe_version, pdb_path_opt))
}
