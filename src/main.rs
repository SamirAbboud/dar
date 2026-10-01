use clap::{Parser, Subcommand};
use clap_complete::{generate, Shell};
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

        /// Raw arguments passed directly to the PHP binary (e.g. `dar php -v` or `dar php -S ...`)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
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

        /// Start all core services or a specific service container
        #[arg(short = 's', long, num_args(0..=1), default_missing_value = "")]
        start: Option<Option<String>>,

        /// Stop all core services or a specific running service container
        #[arg(short = 't', long, num_args(0..=1), default_missing_value = "")]
        stop: Option<Option<String>>,

        /// Restart all core services or a specific service container
        #[arg(short = 'r', long, num_args(0..=1), default_missing_value = "")]
        restart: Option<Option<String>>,

        /// Remove a service container
        #[arg(short = 'd', long)]
        remove: Option<String>,
    },

    /// List all linked local development sites
    Sites,

    /// Generate shell auto-completions for Bash, Zsh, Fish, etc.
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Install => commands::install::run(),
        Commands::Parked => commands::park::get_parked_folders(),
        Commands::Link { name, path, secure } => commands::link::run(name, path, secure),
        Commands::Secure { name, path } => commands::secure::run(name, path),
        Commands::Unlink { name, path } => commands::unlink::run(name, path),
        Commands::Unsecure { name, path } => commands::unsecure::run(name, path),
        Commands::Php {
            list,
            current,
            use_version,
            args,
        } => {
            if let Some(version) = use_version {
                commands::php::use_version(version)
            } else if list {
                commands::php::list()
            } else if current {
                commands::php::current()
            } else if !args.is_empty() {
                // Execute raw php binary with forwarded CLI arguments
                commands::php::passthrough(&args)
            } else {
                // Fallback when running plain `dar php`
                commands::php::current()
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
            } else if let Some(svc_opt) = start {
                let name = svc_opt.as_deref().filter(|s| !s.is_empty());
                commands::services::start(name)
            } else if let Some(svc_opt) = stop {
                let name = svc_opt.as_deref().filter(|s| !s.is_empty());
                commands::services::stop(name)
            } else if let Some(svc_opt) = restart {
                let name = svc_opt.as_deref().filter(|s| !s.is_empty());
                commands::services::restart(name)
            } else if let Some(svc) = remove {
                commands::services::remove(&svc)
            } else {
                commands::services::list()
            }
        }
        Commands::Sites => commands::sites::run(),
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            let mut cmd = Cli::command();
            let bin_name = cmd.get_name().to_string();
            generate(shell, &mut cmd, bin_name, &mut std::io::stdout());
            Ok(())
        }
    };

    if let Err(err_msg) = result {
        eprintln!("{} {}", "❌ Error:".red().bold(), err_msg);
        std::process::exit(1);
    }
}
