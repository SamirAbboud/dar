use crate::services::composer::update_composer;
use crate::services::laravel::update_laravel;
use crate::utils::paths::get_dar_dir;

pub fn run(target: Option<&str>) -> Result<(), String> {
    let dar_dir = get_dar_dir()?;

    match target {
        Some("composer") => update_composer(&dar_dir),
        Some("laravel") => update_laravel(&dar_dir),
        Some(unknown) => Err(format!(
            "Unknown tool: '{}'. Supported tools: composer, laravel",
            unknown
        )),
        None => {
            update_composer(&dar_dir)?;
            update_laravel(&dar_dir)?;
            Ok(())
        }
    }
}
