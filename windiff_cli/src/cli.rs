use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "example", about = "An example of StructOpt usage.")]
pub struct WinDiffOpt {
    /// Path to the configuration file
    #[structopt(parse(from_os_str))]
    pub configuration: PathBuf,
}
