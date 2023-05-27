use std::path::{Path, PathBuf};

use futures::stream::StreamExt;

use crate::{
    configuration::{OSDescription, WinDiffConfiguration},
    error::Result,
    pdb,
    winbindex::{self, DownloadedPEVersion},
};

pub async fn download_binaries(
    cfg: &WinDiffConfiguration,
    output_directory: &Path,
) -> Result<Vec<DownloadedPEVersion>> {
    // Note(ergrelet): arbitrarily defined value
    const CONCURRENT_PE_DOWNLOADS: usize = 16;

    // Fetch all binaries concurrently and fold results into a single `Vec`
    let result: Vec<DownloadedPEVersion> =
        futures::stream::iter(cfg.binaries.keys().map(|binary_name| async move {
            log::trace!("Fetching '{}' binaries ...", binary_name);

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
        let download_result = winbindex::download_pe_version(
            pe_index,
            binary_name,
            &os_desc.version,
            &os_desc.update,
            &os_desc.architecture,
            output_directory,
        )
        .await;
        if download_result.is_err() {
            log::warn!(
                "Failed to download PE '{}' (version '{}-{}-{}')",
                binary_name,
                &os_desc.version,
                &os_desc.update,
                os_desc.architecture.to_str(),
            );
        }

        download_result
    }))
    .buffer_unordered(CONCURRENT_PE_DOWNLOADS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result| async { result.ok() })
    .collect()
    .await
}

pub async fn download_pdbs(
    downloaded_pes: Vec<DownloadedPEVersion>,
    output_directory: &Path,
) -> Vec<(DownloadedPEVersion, Option<PathBuf>)> {
    const CONCURRENT_PDB_DOWNLOADS: usize = 16;

    // Download all requested versions concurrently
    futures::stream::iter(downloaded_pes.into_iter().map(|pe_version| async move {
        let pdb_path_opt = if let Ok(pdb_path) =
            pdb::download_pdb_for_pe(&pe_version.path, output_directory).await
        {
            Some(pdb_path)
        } else {
            log::warn!(
                "Failed to download PDB for PE '{}', version {}-{}",
                pe_version.original_name,
                pe_version.os_version,
                pe_version.os_update
            );
            None
        };

        Ok((pe_version, pdb_path_opt))
    }))
    .buffer_unordered(CONCURRENT_PDB_DOWNLOADS)
    // Ignore errors and simply skip the corresponding files
    .filter_map(|result: Result<(DownloadedPEVersion, Option<PathBuf>)>| async { result.ok() })
    .collect()
    .await
}
