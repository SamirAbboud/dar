use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use colored::*;

use crate::utils::paths::get_dar_dir;

pub fn run(
    nginx: bool,
    php: Option<String>,
    site: Option<String>,
    docker: Option<String>,
    lines: usize,
    follow: bool,
) -> Result<(), String> {
    // 1. Docker Service Logging
    if let Some(service) = docker {
        return stream_docker_logs(&service, lines, follow);
    }

    // 2. File-based Target Resolution
    let log_files = resolve_log_targets(nginx, php, site)?;

    if log_files.is_empty() {
        println!("No matching log files found.");
        return Ok(());
    }

    // 3. Output Initial Trailing Lines
    for path in &log_files {
        if log_files.len() > 1 {
            println!("\n===> Log: {} <===", path.display());
        }
        tail_file(path, lines)?;
    }

    // 4. Live Tailing (--follow)
    if follow {
        if log_files.len() == 1 {
            follow_single_file(&log_files[0])?;
        } else {
            follow_multiple_files(log_files)?;
        }
    }

    Ok(())
}

/// Resolves requested log flags to absolute file paths directly under ~/.dar/logs/
fn resolve_log_targets(
    nginx: bool,
    php: Option<String>,
    site: Option<String>,
) -> Result<Vec<PathBuf>, String> {
    let dar_logs = get_dar_dir()
        .map_err(|e| format!("Failed to resolve ~/.dar directory: {}", e))?
        .join("logs");
    let mut targets = Vec::new();

    let default_mode = !nginx && php.is_none() && site.is_none();

    // Nginx logs (error.log / access.log directly under ~/.dar/logs/)
    if nginx || default_mode {
        let error_log = dar_logs.join("error.log");
        let access_log = dar_logs.join("access.log");
        if error_log.exists() {
            targets.push(error_log);
        }
        if nginx && access_log.exists() {
            targets.push(access_log);
        }
    }

    // PHP logs (php-fpm.log or php<VERSION>-fpm.log directly under ~/.dar/logs/)
    if let Some(version) = php {
        let log_name = if version == "current" {
            "php-fpm.log".to_string()
        } else {
            format!("php{}-fpm.log", version)
        };
        let php_log = dar_logs.join(log_name);
        if php_log.exists() {
            targets.push(php_log);
        } else {
            eprintln!("Warning: PHP log file not found at {}", php_log.display());
        }
    } else if default_mode {
        let global_php = dar_logs.join("php-fpm.log");
        if global_php.exists() {
            targets.push(global_php);
        }
    }

    // Site-specific logs (<SITE>.error.log / <SITE>.access.log directly under ~/.dar/logs/)
    if let Some(site_name) = site {
        let site_error = dar_logs.join(format!("{}.error.log", site_name));
        let site_access = dar_logs.join(format!("{}.access.log", site_name));

        if site_error.exists() {
            targets.push(site_error);
        }
        if site_access.exists() {
            targets.push(site_access);
        }
    }

    Ok(targets)
}

/// Prints the last N lines from a file
fn tail_file(path: &Path, lines: usize) -> Result<(), String> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => {
            eprintln!("Could not open log file: {}", path.display());
            return Ok(());
        }
    };

    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
    let start = all_lines.len().saturating_sub(lines);

    for line in &all_lines[start..] {
        println!("{}", colorize_line(line));
    }

    Ok(())
}

/// Continuously follows live output for a single log file
fn follow_single_file(path: &Path) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("Failed to open log file: {}", e))?;
    file.seek(SeekFrom::End(0))
        .map_err(|e| format!("Failed to seek to end of file: {}", e))?;

    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line) {
            Ok(0) => thread::sleep(Duration::from_millis(250)),
            Ok(_) => {
                print!("{}", colorize_line(&line));
                line.clear();
            }
            Err(e) => return Err(format!("Error reading log file: {}", e)),
        }
    }
}

/// Concurrently tails multiple log files using thread polling
fn follow_multiple_files(paths: Vec<PathBuf>) -> Result<(), String> {
    let mut handles = vec![];

    for path in paths {
        let handle = thread::spawn(move || {
            let prefix = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("log");

            let mut file = match File::open(&path) {
                Ok(f) => f,
                Err(_) => return,
            };
            let _ = file.seek(SeekFrom::End(0));

            let mut reader = BufReader::new(file);
            let mut line = String::new();

            loop {
                match reader.read_line(&mut line) {
                    Ok(0) => thread::sleep(Duration::from_millis(250)),
                    Ok(_) => {
                        print!("[{}] {}", prefix.bold().blue(), colorize_line(&line));
                        line.clear();
                    }
                    Err(_) => break,
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    Ok(())
}

/// Streams Docker logs using native `docker logs` command execution
fn stream_docker_logs(service: &str, lines: usize, follow: bool) -> Result<(), String> {
    let container_name = format!("dar--{}", service);

    let mut cmd = Command::new("docker");
    cmd.arg("logs")
        .arg("--tail")
        .arg(lines.to_string());

    if follow {
        cmd.arg("-f");
    }

    // Capture stdout
    cmd.arg(&container_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to execute 'docker logs' for container '{}': {}", container_name, e))?;

    // Read stdout
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(Result::ok) {
            println!("{}", colorize_line(&line));
        }
    }

    // Read stderr (where MySQL and many official images write logs)
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(Result::ok) {
            println!("{}", colorize_line(&line));
        }
    }

    let _ = child.wait();
    Ok(())
}

/// Colorizes log output lines based on severity and status codes
fn colorize_line(line: &str) -> String {
    let lower = line.to_lowercase();

    if lower.contains("[error]") || lower.contains("error:") || lower.contains("[alert]") {
        line.red().to_string()
    } else if lower.contains("[warning]") || lower.contains("[warn]") || lower.contains("warning:") || lower.contains("notice:") {
        line.yellow().to_string()
    } else if lower.contains("[system]") {
        line.cyan().to_string()
    } else if lower.contains("http/1.1\" 200") || lower.contains("http/2.0\" 200") {
        line.green().to_string()
    } else if lower.contains("http/1.1\" 5") || lower.contains("http/2.0\" 5") {
        line.red().to_string()
    } else {
        line.to_string()
    }
}
