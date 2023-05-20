use std::collections::BTreeSet;

use serde::Serialize;

#[derive(Serialize, Debug, Default)]
pub struct DatabaseIndex {
    pub oses: Vec<OSVersion>,
    pub binaries: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct OSVersion {
    pub version: String,
    pub update: String,
    pub architecture: String,
}

#[derive(Serialize, Debug, Default)]
pub struct BinaryDatabase {
    pub metadata: BinaryMetadata,
    /// Exported symbols
    pub exports: BTreeSet<String>,
    /// Debug symbols
    pub symbols: BTreeSet<String>,
    /// Compiled modules
    pub modules: BTreeSet<String>,
}

#[derive(Serialize, Debug, Default)]
pub struct BinaryMetadata {
    pub name: String,
    pub version: String,
    pub architecture: String,
}
