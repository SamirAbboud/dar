use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use comfy_table::presets::UTF8_BORDERS_ONLY;
use comfy_table::{Attribute, Cell, Color as TableColor, Table};

/// Reads `parked.txt` from `dar_dir` and resolves paths into (Name, Path) pairs.
fn read_parked_folders(dar_dir: &Path) -> Vec<(String, String)> {
    let store_path = dar_dir.join("parked.txt");
    let content = fs::read_to_string(&store_path).unwrap_or_default();
    let home_dir = dirs::home_dir();

    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            let expanded = match (line.strip_prefix('~'), &home_dir) {
                (Some(rest), Some(home)) => format!("{}{}", home.display(), rest),
                _ => line.to_string(),
            };

            let name = PathBuf::from(&expanded)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| expanded.clone());

            (name, expanded)
        })
        .collect()
}

/// Formats and prints the parked folders table to stdout.
fn print_parked_folders(rows: &[(String, String)]) {
    if rows.is_empty() {
        println!("{}", "No folders are currently parked.".cyan());
        return;
    }

    let indigo = TableColor::Rgb {
        r: 187,
        g: 154,
        b: 247,
    };

    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);

    table.set_header(vec![
        Cell::new("Name").fg(indigo).add_attribute(Attribute::Bold),
        Cell::new("Path").fg(indigo).add_attribute(Attribute::Bold),
    ]);

    for (name, path) in rows {
        table.add_row(vec![
            Cell::new(name).add_attribute(Attribute::Bold),
            Cell::new(path),
        ]);
    }

    println!("{table}");
}

/// Lists all parked folders in `dar_dir` and prints them to stdout.
fn list_parked_folders(dar_dir: &Path) -> Result<(), String> {
    let rows = read_parked_folders(dar_dir);
    print_parked_folders(&rows);
    Ok(())
}

pub fn run() -> Result<(), String> {
    let home = dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;
    let dar_dir = home.join(".dar");

    list_parked_folders(&dar_dir)?;
    Ok(())
}
