use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Parser)]
#[command(
    name = "dar",
    version,
    about = "Development environment tool",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install Dar environment (nginx include, folders, PHP-FPM)
    Install,

    /// Update managed global tools (Composer, Laravel Installer)
    Update {
        /// Specific tool to update (e.g. "composer", "laravel"). Updates all if omitted.
        #[arg(value_name = "TOOL")]
        tool: Option<String>,
    },

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

        /// Isolate current project site to a specific PHP version (e.g. `dar php -i 8.2` or `dar php -i`)
        #[arg(short = 'i', long = "isolate", num_args(0..=1), default_missing_value = "")]
        isolate: Option<String>,

        /// Remove PHP version isolation for the current project site
        #[arg(short = 'U', long = "unisolate", alias = "un")]
        unisolate: bool,

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

    /// Execute Laravel Installer commands using the active isolated PHP runtime
    Laravel {
        /// Raw arguments passed directly to laravel installer (e.g. `dar laravel new my-app`)
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

    /// Clear caches, temporary logs, or wipe the ~/.dar environment
    #[command(alias = "clean")]
    Clear {
        /// Clean temporary logs (~/.dar/logs/*.log, Nginx buffer folders)
        #[arg(short, long)]
        logs: bool,

        /// Clean downloaded build source tarballs (~/.dar/src) and Composer cache
        #[arg(short, long)]
        cache: bool,

        /// Wipe installed PHP versions (~/.dar/php/versions) and managed binaries (~/.dar/bin)
        #[arg(short, long)]
        runtimes: bool,

        /// Nuclear option: Wipe the entire ~/.dar directory
        #[arg(short, long)]
        all: bool,

        /// Skip interactive confirmation prompts
        #[arg(short, long)]
        force: bool,
    },

    /// Tail and manage logs for Nginx, PHP-FPM, sites, and Docker containers
    #[command(alias = "logs")]
    Log {
        /// Tail Nginx access and error logs (~/.dar/logs/nginx/)
        #[arg(short, long)]
        nginx: bool,

        /// Tail primary PHP log or a specific isolated version (e.g. --php or --php 8.3)
        #[arg(short, long, value_name = "VERSION", num_args = 0..=1, default_missing_value = "current")]
        php: Option<String>,

        /// Tail access and error logs for a specific site (e.g. --site my-app)
        #[arg(short, long, value_name = "SITE")]
        site: Option<String>,

        /// Tail container output for a Docker service (e.g. --docker sqlstudio)
        #[arg(short, long, value_name = "SERVICE")]
        docker: Option<String>,

        /// Number of lines to output from the end of the log file
        #[arg(short, long, default_value = "20")]
        lines: usize,

        /// Continuously follow live log output
        #[arg(short, long)]
        follow: bool,
    },
}
