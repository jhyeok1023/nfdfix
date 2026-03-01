use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nfdfix")]
#[command(about = "Fix macOS NFD-normalized filenames")]
pub struct Cli {
    /// Target path (file or directory)
    pub path: PathBuf,

    /// Recursive scan
    #[arg(short, long)]
    pub recursive: bool,

    /// Dry run (do not rename)
    #[arg(long)]
    pub dry_run: bool,
}
