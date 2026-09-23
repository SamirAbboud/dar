use comfy_table::presets::UTF8_BORDERS_ONLY;
use comfy_table::{Attribute, Cell, Color as TableColor, Table};

/// Creates a unified comfy-table instance with pre-configured borders and styled headers.
pub fn create_table(headers: &[&str]) -> Table {
    let mut table = Table::new();
    table.load_preset(UTF8_BORDERS_ONLY);

    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|&header| {
            Cell::new(header)
                .fg(TableColor::Magenta)
                .add_attribute(Attribute::Bold)
        })
        .collect();

    table.set_header(header_cells);
    table
}
