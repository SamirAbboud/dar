use clap::{Parser, Subcommand};
use colored::*;

mod commands;
mod config;
mod services;
mod utils;
mod store;

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

    /// Stop active background services
    Stop,

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

        /// Secure the site with an SSL/TLS certificate on link
        #[arg(short, long)]
        secure: bool,
    },

    /// Secure a local .test site with SSL/TLS certificate
    Secure {
        /// Optional custom domain name (defaults to target directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Optional target directory path (defaults to current working directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Unlink a site and remove its local Nginx configuration
    Unlink {
        /// Optional custom domain name of the site to unlink (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Optional path to the project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Unsecure a site by removing SSL certificates and reverting to HTTP
    Unsecure {
        /// Optional custom domain name of the site to unsecure (defaults to directory name)
        #[arg(short, long)]
        name: Option<String>,

        /// Optional path to the project directory (defaults to current directory)
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Manage isolated PHP runtimes
    Php {
        /// List installed PHP versions
        #[arg(short, long)]
        list: bool,

        /// Display the currently active PHP version
        #[arg(short, long)]
        current: bool,

        /// Switch or install a specific PHP version (e.g. `dar php -u 8.2`)
        #[arg(short = 'u', long = "use")]
        use_version: Option<String>,
    },

    /// Execute Composer commands using the active isolated PHP runtime
    Composer {
        /// Raw arguments passed directly to composer (e.g. `dar composer require laravel/framework`)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },

    /// Manage background infrastructure services
    Services {
        /// List available and running service containers
        #[arg(short, long)]
        list: bool,

        /// Add and start a new service container
        #[arg(short, long)]
        add: Option<String>,

        /// Start a stopped service container
        #[arg(short = 's', long)]
        start: Option<String>,

        /// Stop a running service container
        #[arg(short = 't', long)]
        stop: Option<String>,

        /// Restart a service container
        #[arg(short = 'r', long)]
        restart: Option<String>,

        /// Remove a service container
        #[arg(short = 'd', long)]
        remove: Option<String>,
    },

    /// List all linked local development sites
    Sites,
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Install => commands::install::run(),
        Commands::Start => commands::start::run(),
        Commands::Stop => commands::stop::run(),
        Commands::Parked => commands::park::get_parked_folders(),
        Commands::Link { name, path, secure } => commands::link::run(name, path, secure),
        Commands::Secure { name, path } => commands::secure::run(name, path),
        Commands::Unlink { name, path } => commands::unlink::run(name, path),
        Commands::Unsecure { name, path } => commands::unsecure::run(name, path),
        Commands::Php { current, use_version, .. } => {
            if let Some(version) = use_version {
                commands::php::use_version(version)
            } else if current {
                commands::php::current()
            } else {
                commands::php::list()
            }
        }
        Commands::Composer { args } => commands::composer::run(&args),
        Commands::Services {
            list: _,
            add,
            start,
            stop,
            restart,
            remove,
        } => {
            if let Some(svc) = add {
                commands::services::add(&svc)
            } else if let Some(svc) = start {
                commands::services::start(&svc)
            } else if let Some(svc) = stop {
                commands::services::stop(&svc)
            } else if let Some(svc) = restart {
                commands::services::restart(&svc)
            } else if let Some(svc) = remove {
                commands::services::remove(&svc)
            } else {
                commands::services::list()
            }
        }
        Commands::Sites => commands::sites::run(),
    };

    if let Err(err_msg) = result {
        eprintln!("{} {}", "❌ Error:".red().bold(), err_msg);
        std::process::exit(1);
    }
}
