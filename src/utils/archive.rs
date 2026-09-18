use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tar::Archive;
use std::time::Duration;
use std::thread::sleep;

pub fn fetch_and_extract(
    src_dir: &Path,
    name: &str,
    folder_name: &str,
    tarball_name: &str,
    download_url: &str,
) -> Result<PathBuf, String> {
    let target_folder = src_dir.join(folder_name);
    if target_folder.exists() {
        return Ok(target_folder);
    }

    let tarball_path = src_dir.join(tarball_name);

    if !tarball_path.exists() {
        println!(" Downloading {} static source...", name);

        let mut attempts = 0;
        let max_attempts = 3;

        let response = loop {
            attempts += 1;
            match ureq::get(download_url).call() {
                Ok(res) => break res,
                Err(e) => {
                    if attempts >= max_attempts {
                        return Err(format!(
                            "Failed to download {} after {} attempts: {}",
                            name, max_attempts, e
                        ));
                    }
                    println!(
                        " ⚠️ Network issue during {} download. Retrying ({}/{})...",
                        name, attempts, max_attempts
                    );
                    sleep(Duration::from_secs(2));
                }
            }
        };

        let mut reader = response.into_body().into_reader();
        let mut out = File::create(&tarball_path)
            .map_err(|e| format!("Failed to create file {}: {}", tarball_path.display(), e))?;

        io::copy(&mut reader, &mut out)
            .map_err(|e| format!("Error saving {} tarball: {}", name, e))?;
    }

    println!(" Extracting {} archive...", name);
    let tar_gz = File::open(&tarball_path)
        .map_err(|e| format!("Failed to open {} tarball: {}", name, e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    fs::create_dir_all(&target_folder)
        .map_err(|e| format!("Failed to create folder {}: {}", target_folder.display(), e))?;

    archive
        .unpack(&target_folder)
        .map_err(|e| format!("Failed to extract {} archive: {}", name, e))?;

    Ok(target_folder)
}

pub fn find_file_recursive(dir: &Path, filename: &str) -> Option<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.file_name() == Some(std::ffi::OsStr::new(filename)) {
                return Some(path);
            } else if path.is_dir() {
                if let Some(found) = find_file_recursive(&path, filename) {
                    return Some(found);
                }
            }
        }
    }
    None
}

pub fn resolve_build_dir(path: &Path) -> PathBuf {
    if let Ok(entries) = fs::read_dir(path) {
        let items: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        if items.len() == 1 && items[0].is_dir() {
            return items[0].clone();
        }
    }
    path.to_path_buf()
}
