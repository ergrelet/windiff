use std::path::PathBuf;
use structopt::StructOpt;

const PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Debug, StructOpt)]
#[structopt(name = PKG_NAME, about = "A CLI utility that generates JSON databases for windiff.")]
pub struct WinDiffOpt {
    /// Path to the configuration file
    #[structopt(parse(from_os_str))]
    pub configuration: PathBuf,
    /// Path to the output directory that'll contain the generated files.
    #[structopt(default_value = "", parse(from_os_str))]
    pub output_directory: PathBuf,
    /// Enable "low storage" mode. This might be needed if you run the tool in
    /// a constrained environment (e.g., a CI runner)
    #[structopt(short, long)]
    pub low_storage_mode: bool,
    /// Number of concurrent downloads to perform while downloading files. Defaults to 64.
    #[structopt(short, long, default_value = "64")]
    pub concurrent_downloads: usize,
}
