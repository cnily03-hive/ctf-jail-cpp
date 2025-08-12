use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "jailbox")]
#[command(about = "Web UI for CTF sandbox execution environment")]
#[command(version = "0.1.0")]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start the web server
    Listen {
        /// Server port
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// Server host address
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// Context directory path
        #[arg(short, long, default_value = "./context")]
        context: PathBuf,

        /// Rune script file path
        #[arg(short, long)]
        exec: Option<PathBuf>,
    },
    /// Run the collect function and return results
    Collect {
        /// Rune script file path
        #[arg(short, long)]
        exec: Option<PathBuf>,

        /// Context directory path
        #[arg(short, long, default_value = "./context")]
        context: PathBuf,

        /// Whether to parse JSON output
        #[arg(short = 'P', long, default_value = "false")]
        parse: bool,
    },
    /// Run the check function and return results
    Check {
        /// Rune script file path
        #[arg(short, long)]
        exec: Option<PathBuf>,

        /// User input
        #[arg(short, long, alias = "user-input")]
        input: String,

        /// Context directory path
        #[arg(short, long, default_value = "./context")]
        context: PathBuf,

        /// Whether to parse JSON output
        #[arg(short = 'P', long, default_value = "false")]
        parse: bool,
    },
}
