use crate::commands::link;

/// Secures a local `.test` site with an SSL certificate.
/// 
/// This is a wrapper around [`link::run`] with the `secure` flag set to `true`.
/// It generates an SSL certificate via `mkcert`, writes HTTPS Nginx virtual host 
/// rules, and reloads Nginx in a single pass.
///
/// Accepts optional `--name` and `--path` overrides.
pub fn run(name: Option<String>, path: Option<String>) -> Result<(), String> {
    link::run(name, path, true)
}
