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
    #[structopt(short, long)]
    /// Enable "low storage" mode. This might be needed if you run the tool in
    /// a constrained environment (e.g., a CI runner)
    pub low_storage_mode: bool,
}
