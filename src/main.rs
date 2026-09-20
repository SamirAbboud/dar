use clap::{Parser, Subcommand};
use colored::*;

mod commands;
mod config;
mod services;
mod utils;

#[derive(Parser)]
#[command(
    name = "dar",
    version,
    about = "Development environment tool",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install Dar environment (nginx include, folders, PHP-FPM)
    Install,
    /// List all parked development directories
    Parked,
    /// Link current directory or specified name as a local dev site
    Link {
        /// Optional site name (defaults to current folder name)
        name: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Install => commands::install::run(),
        Commands::Parked => commands::park::get_parked_folders(),
        Commands::Link { name } => commands::link::run(name),
    };

    if let Err(err_msg) = result {
        eprintln!("{} {}", "❌ Error:".red().bold(), err_msg);
        std::process::exit(1);
    }
}
