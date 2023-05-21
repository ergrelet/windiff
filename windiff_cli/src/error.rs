use thiserror::Error;

pub type Result<T> = std::result::Result<T, WinDiffError>;

/// TODO
#[derive(Error, Debug)]
pub enum WinDiffError {
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("json error: {0}")]
    JSONError(#[from] serde_json::Error),
    #[error("url parsing error: {0}")]
    UrlParseError(#[from] url::ParseError),
    #[error("reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("goblin error: {0}")]
    GoblinError(#[from] goblin::error::Error),
    #[error("crossbeam_channel error: {0}")]
    CrossbeamChannelRecvError(#[from] crossbeam_channel::RecvError),
    #[error("resym_core error: {0}")]
    ResymCoreError(#[from] resym_core::ResymCoreError),
    #[error("TryFromSlice error: {0}")]
    TryFromSliceError(#[from] std::array::TryFromSliceError),
    #[error("utf8 error error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("pdb error: {0}")]
    PDBError(#[from] pdb::Error),
    #[error("resym backend error: {0}")]
    ResymBackendError(String),
    #[error("PE file not found in the index")]
    FileNotFoundInIndex,
    #[error("unsupported executable format given")]
    UnsupportedExecutableFormat,
}
