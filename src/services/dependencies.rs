use crate::utils::archive::fetch_and_extract;
use std::path::{Path, PathBuf};

/// Downloads and extracts the PCRE2 source tarball if not already present in `~/.dar/src`.
pub fn ensure_pcre2_source(src_dir: &Path, version: &str) -> Result<PathBuf, String> {
    fetch_and_extract(
        src_dir,
        "PCRE2",
        &format!("pcre2-{}", version),
        &format!("pcre2-{}.tar.gz", version),
        &format!(
            "https://github.com/PCRE2Project/pcre2/releases/download/pcre2-{}/pcre2-{}.tar.gz",
            version, version
        ),
    )
}

/// Downloads and extracts the OpenSSL source tarball if not already present in `~/.dar/src`.
pub fn ensure_openssl_source(src_dir: &Path, version: &str) -> Result<PathBuf, String> {
    fetch_and_extract(
        src_dir,
        "OpenSSL",
        &format!("openssl-{}", version),
        &format!("openssl-{}.tar.gz", version),
        &format!(
            "https://github.com/openssl/openssl/releases/download/openssl-{}/openssl-{}.tar.gz",
            version, version
        ),
    )
}

/// Downloads and extracts the zlib source tarball if not already present in `~/.dar/src`.
pub fn ensure_zlib_source(src_dir: &Path, version: &str) -> Result<PathBuf, String> {
    fetch_and_extract(
        src_dir,
        "zlib",
        &format!("zlib-{}", version),
        &format!("zlib-{}.tar.gz", version),
        &format!(
            "https://github.com/madler/zlib/releases/download/v{}/zlib-{}.tar.gz",
            version, version
        ),
    )
}

/// Downloads and extracts the Nginx source tarball if not already present in `~/.dar/src`.
pub fn ensure_nginx_source(src_dir: &Path, version: &str) -> Result<PathBuf, String> {
    fetch_and_extract(
        src_dir,
        "Nginx",
        &format!("nginx-{}", version),
        &format!("nginx-{}.tar.gz", version),
        &format!("https://nginx.org/download/nginx-{}.tar.gz", version),
    )
}
