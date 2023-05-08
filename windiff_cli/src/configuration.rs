use crate::error::Result;

use serde::Deserialize;
use std::{io::Read, path::Path};

/// TODO
#[derive(Deserialize)]
pub struct WinDiffConfiguration {
    pub oses: Vec<OSDescription>,
    pub binaries: Vec<BinaryDescription>,
}

/// TODO
#[derive(Deserialize)]
pub struct OSDescription {
    pub version: String,
    pub update: String,
    pub architecture: OSArchitecture,
}

/// TODO
#[derive(Deserialize, Debug, Copy, Clone)]
#[serde(rename_all = "lowercase")]
pub enum OSArchitecture {
    I386,
    Wow64,
    Amd64,
    Arm,
    Arm64,
}

/// Binary description
#[derive(Deserialize)]
pub struct BinaryDescription {
    pub name: String,
    pub extracted_information: Vec<BinaryExtractedInformation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum BinaryExtractedInformation {
    Exports,
}

impl WinDiffConfiguration {
    pub fn from_file(path: &Path) -> Result<Self> {
        // Open file
        let mut file = std::fs::File::open(path)?;

        // Read file
        let mut file_data = vec![];
        let _read_bytes = file.read_to_end(&mut file_data)?;

        // Parse JSON and return result
        Ok(serde_json::from_slice(&file_data)?)
    }
}

impl OSArchitecture {
    pub fn to_str(self) -> &'static str {
        match self {
            OSArchitecture::I386 => "i386",
            OSArchitecture::Wow64 => "wow64",
            OSArchitecture::Amd64 => "amd64",
            OSArchitecture::Arm => "arm64.arm",
            OSArchitecture::Arm64 => "arm64",
        }
    }

    // https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#machine-types
    pub fn to_machine_type(self) -> u32 {
        match self {
            OSArchitecture::I386 | OSArchitecture::Wow64 => 0x14c,
            OSArchitecture::Amd64 => 0x8664,
            OSArchitecture::Arm => 0x1c0,
            OSArchitecture::Arm64 => 0xaa64,
        }
    }
}
