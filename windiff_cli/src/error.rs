use std::io;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, WinDiffError>;

/// TODO
#[derive(Error, Debug)]
pub enum WinDiffError {
    #[error("io error: {0}")]
    IoError(#[from] io::Error),
    #[error("json error: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("url parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("PE file not found in the index")]
    FileNotFoundInIndex,
}
