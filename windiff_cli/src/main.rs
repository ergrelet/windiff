mod cli;
mod configuration;
mod database;
mod download;
mod error;
mod pdb;
mod resym_frontend;
mod syscalls;
mod winbindex;

use database::generate_database_index;
use env_logger::Env;
use structopt::StructOpt;

use crate::{
    cli::WinDiffOpt,
    configuration::WinDiffConfiguration,
    database::generate_databases,
    download::{download_all_binaries, download_all_pdbs, download_single_binary},
    error::Result,
};

const PACKAGE_NAME: &str = env!("CARGO_PKG_NAME");

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        Env::default().default_filter_or(format!("{}=info", PACKAGE_NAME)),
    )
    .init();

    // Parse command-line options
    let opt = WinDiffOpt::from_args();
    log::info!("Using configuration file: {:?}", opt.configuration);

    // Parse configuration file
    let cfg = WinDiffConfiguration::from_file(&opt.configuration).await?;

    if opt.low_storage_mode {
        low_storage_mode(opt, cfg).await
    } else {
        normal_mode(opt, cfg).await
    }
}

async fn low_storage_mode(opt: WinDiffOpt, cfg: WinDiffConfiguration) -> Result<()> {
    let mut download_binaries_acc = vec![];
    for pe_name in cfg.binaries.keys() {
        let tmp_directory = tempdir::TempDir::new(PACKAGE_NAME)?;
        let tmp_directory_path = tmp_directory.path();

        log::info!("Downloading binaries for '{}' ...", pe_name);
        let downloaded_pes = download_single_binary(pe_name, &cfg.oses, tmp_directory_path).await?;
        let mut downloaded_binaries = download_all_pdbs(downloaded_pes, tmp_directory_path).await;
        // Extract information from PEs and generate databases for all versions
        log::info!("Generating databases for '{}' ...", pe_name);
        generate_databases(&cfg, &downloaded_binaries, false, &opt.output_directory).await?;

        // Move binary info into the global vec
        download_binaries_acc.append(&mut downloaded_binaries);
    }

    // Generate database index from the global vec
    generate_database_index(&download_binaries_acc, &opt.output_directory).await?;
    log::info!(
        "Databases have been generated at {:?}",
        opt.output_directory
    );

    Ok(())
}

async fn normal_mode(opt: WinDiffOpt, cfg: WinDiffConfiguration) -> Result<()> {
    let tmp_directory = tempdir::TempDir::new(PACKAGE_NAME)?;
    let tmp_directory_path = tmp_directory.path();

    // Download requested PEs
    log::info!("Downloading PEs...");
    let downloaded_pes = download_all_binaries(&cfg, tmp_directory_path).await?;
    log::trace!("PEs downloaded!");

    // Download PDBs
    log::info!("Downloading PDBs...");
    let downloaded_binaries = download_all_pdbs(downloaded_pes, tmp_directory_path).await;
    log::trace!("PDBs downloaded!");

    // Extract information from PEs
    log::info!("Generating databases...");
    generate_databases(&cfg, &downloaded_binaries, true, &opt.output_directory).await?;
    log::info!(
        "Databases have been generated at {:?}",
        opt.output_directory
    );

    Ok(())
}
