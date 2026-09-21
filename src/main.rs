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

    /// Start background services (Nginx & PHP-FPM)
    Start,

    /// List all parked development directories
    Parked,

    /// Link current directory or specified name as a local dev site
    Link {
        /// Optional custom domain name (defaults to target directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Optional target directory path (defaults to current working directory)
        #[arg(short, long)]
        path: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Install => commands::install::run(),
        Commands::Start => commands::start::run(),
        Commands::Parked => commands::park::get_parked_folders(),
        Commands::Link { name, path } => commands::link::run(name, path),
    };

    if let Err(err_msg) = result {
        eprintln!("{} {}", "❌ Error:".red().bold(), err_msg);
        std::process::exit(1);
    }
}
