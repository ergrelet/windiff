use async_compression::tokio::bufread::GzipDecoder;
use futures::StreamExt;
use serde::Deserialize;
use tokio::{fs::File, io::AsyncReadExt};

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{configuration::OSArchitecture, error::Result};

const WINBINDEX_BY_FILENAME_BASE_URL: &str =
    "https://winbindex.m417z.com/data/by_filename_compressed/";
const MSDL_FILE_DOWNLOAD_BASE_URL: &str = "https://msdl.microsoft.com/download/symbols/";

#[derive(Debug)]
pub struct DownloadedPEVersion {
    pub path: PathBuf,
    pub original_name: String,
    pub os_version: String,
    pub os_update: String,
    pub architecture: OSArchitecture,
    pub pe_version: String,
}

pub async fn get_remote_index_for_pe(pe_name: &str) -> Result<serde_json::Value> {
    let index_file_url = generate_index_file_url(pe_name)?;
    // Get compressed index file
    let http_response = reqwest::get(index_file_url).await?.error_for_status()?;
    let compressed_index_file = http_response.bytes().await?;

    // Decompress and parse the index file
    parse_compressed_index_file(&compressed_index_file[..]).await
}

pub async fn download_pe_version(
    pe_index: &serde_json::Value,
    pe_name: &str,
    os_version: &str,
    os_update: &str,
    os_architecture: &OSArchitecture,
    output_directory: &Path,
) -> Result<DownloadedPEVersion> {
    log::trace!(
        "Downloading PE '{}_{}_{}_{}'",
        pe_name,
        os_version,
        os_update,
        os_architecture.to_str()
    );

    let pe_info = get_pe_info_from_index(pe_index, os_version, os_update, os_architecture)?;
    let pe_download_url = generate_file_download_url(pe_name, &pe_info)?;
    log::debug!(
        "Found download URL for version '{}-{}': {}",
        os_version,
        os_update,
        pe_download_url.as_str()
    );

    // Get PE file and write its content to a file
    let http_response = reqwest::get(pe_download_url).await?.error_for_status()?;
    let output_file_path = output_directory.join(format!(
        "{}_{}_{}_{}",
        os_version,
        os_update,
        os_architecture.to_str(),
        pe_name
    ));
    let mut output_file = File::create(&output_file_path).await?;
    let mut byte_stream = http_response.bytes_stream();
    while let Some(item) = byte_stream.next().await {
        tokio::io::copy(&mut item?.as_ref(), &mut output_file).await?;
    }

    Ok(DownloadedPEVersion {
        path: output_file_path,
        original_name: pe_name.to_string(),
        os_version: os_version.to_string(),
        os_update: os_update.to_string(),
        architecture: *os_architecture,
        pe_version: pe_info.version.unwrap_or_default(),
    })
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileObject {
    file_info: FileInformation,
    windows_versions: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct FileInformation {
    machine_type: u32,
    virtual_size: u64,
    timestamp: u32,
    version: Option<String>,
    // Note(ergrelet): there are other fields but we don't use them at the moment, so we ignore them
}

fn generate_index_file_url(pe_name: &str) -> Result<reqwest::Url> {
    // Note(ergrelet): static str that we control, unwrap shouldn't fail
    let base_url = reqwest::Url::from_str(WINBINDEX_BY_FILENAME_BASE_URL).unwrap();

    Ok(base_url.join(format!("{}.json.gz", pe_name).as_str())?)
}

fn generate_file_download_url(pe_name: &str, file_info: &FileInformation) -> Result<reqwest::Url> {
    // "%s/%s/%08X%x/%s" % (serverName, peName, timeStamp, imageSize, peName)
    // https://randomascii.wordpress.com/2013/03/09/symbols-the-microsoft-way/
    // Note(ergrelet): static str that we control, unwrap shouldn't fail
    let base_url = reqwest::Url::from_str(MSDL_FILE_DOWNLOAD_BASE_URL).unwrap();
    let timestamp = file_info.timestamp;
    let image_size = file_info.virtual_size;

    Ok(base_url
        .join(format!("{}/", pe_name).as_str())?
        .join(format!("{:08X}{:x}/", timestamp, image_size).as_str())?
        .join(pe_name)?)
}

async fn parse_compressed_index_file(data: &[u8]) -> Result<serde_json::Value> {
    let mut gz = GzipDecoder::new(data);
    let mut decompressed_index_file = vec![];
    gz.read_to_end(&mut decompressed_index_file).await?;

    Ok(serde_json::from_slice(&decompressed_index_file)?)
}

fn get_pe_info_from_index(
    json_index: &serde_json::Value,
    os_version: &str,
    os_update: &str,
    os_architecture: &OSArchitecture,
) -> Result<FileInformation> {
    if let Some(file_map) = json_index.as_object() {
        for file_object in file_map.values() {
            if let Ok(file_object) = serde_json::from_value::<FileObject>(file_object.clone()) {
                if is_file_architecture_correct(&file_object, os_architecture)
                    && is_file_version_correct(&file_object, os_version, os_update)
                {
                    return Ok(file_object.file_info);
                }
            }
        }
    }

    Err(crate::error::WinDiffError::FileNotFoundInIndex)
}

fn is_file_architecture_correct(
    file_object: &FileObject,
    os_architecture: &OSArchitecture,
) -> bool {
    file_object.file_info.machine_type == os_architecture.to_machine_type()
}

fn is_file_version_correct(file_object: &FileObject, os_version: &str, os_update: &str) -> bool {
    if let Some(version_info) = file_object.windows_versions.get(os_version) {
        // Found the OS version
        if version_info.get(os_update).is_some() {
            // Found OS update
            return true;
        }
    }

    false
}
