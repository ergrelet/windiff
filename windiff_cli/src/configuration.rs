use crate::error::Result;

use enumflags2::{bitflags, BitFlag, BitFlags};
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use std::{collections::BTreeMap, path::Path};
use tokio::{fs::File, io::AsyncReadExt};

/// TODO
#[derive(Deserialize)]
pub struct WinDiffConfiguration {
    pub oses: Vec<OSDescription>,
    pub binaries: BTreeMap<String, BinaryDescription>,
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
    #[serde(deserialize_with = "deserialize_flags")]
    pub extracted_information: BinaryExtractedInformation,
}

#[bitflags]
#[repr(u16)]
#[derive(Copy, Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BinaryExtractedInformationFlags {
    Exports,
    DebugSymbols,
}

pub type BinaryExtractedInformation = BitFlags<BinaryExtractedInformationFlags>;

impl WinDiffConfiguration {
    pub async fn from_file(path: &Path) -> Result<Self> {
        // Open file
        let mut file = File::open(path).await?;

        // Read file
        let mut file_data = vec![];
        let _read_bytes = file.read_to_end(&mut file_data).await?;

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

pub fn deserialize_flags<'de, D, T>(d: D) -> std::result::Result<BitFlags<T>, D::Error>
where
    D: Deserializer<'de>,
    T: BitFlag + DeserializeOwned,
{
    let flags = Vec::<T>::deserialize(d)?;
    Ok(BitFlags::from_iter(flags))
}
