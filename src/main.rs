use clap::Parser;
use clap_complete::generate;
use colored::*;

mod cli;
mod commands;
mod config;
mod services;
mod utils;
mod store;

use cli::{Cli, Commands};

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Install => commands::install::run(),
        Commands::Update { tool } => commands::update::run(tool.as_deref()),
        Commands::Parked => commands::park::get_parked_folders(),
        Commands::Link { name, path, secure } => commands::link::run(name, path, secure),
        Commands::Secure { name, path } => commands::secure::run(name, path),
        Commands::Unlink { name, path } => commands::unlink::run(name, path),
        Commands::Unsecure { name, path } => commands::unsecure::run(name, path),
        Commands::Php {
            list,
            current,
            use_version,
            isolate,
            unisolate,
            args,
        } => {
            if unisolate {
                commands::php::unisolate()
            } else if let Some(version) = isolate {
                let target = if version.is_empty() { None } else { Some(version) };
                commands::php::isolate(target)
            } else if let Some(version) = use_version {
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
        Commands::Laravel { args } => commands::laravel::run(&args),
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
        Commands::Clear {
            logs,
            cache,
            runtimes,
            all,
            force,
        } => commands::clear::run(logs, cache, runtimes, all, force),
        Commands::Log {
            nginx,
            php,
            site,
            docker,
            lines,
            follow,
        } => commands::log::run(nginx, php, site, docker, lines, follow),
        Commands::Status => commands::status::run(),
    };

    if let Err(err_msg) = result {
        eprintln!("{} {}", "❌ Error:".red().bold(), err_msg);
        std::process::exit(1);
    }
}
