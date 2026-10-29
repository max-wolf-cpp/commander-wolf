use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use mlua::{Lua, Table};
use ratatui::style::Color;

use crate::theme::Rgb;

const ICONS_BY_FILENAME: &str = include_str!("../../Vendor/NvimWebDevicons/icons_by_filename.lua");
const ICONS_BY_FILE_EXTENSION: &str =
    include_str!("../../Vendor/NvimWebDevicons/icons_by_file_extension.lua");
const LIGHT_ICONS_BY_FILENAME: &str =
    include_str!("../../Vendor/NvimWebDevicons/Light/icons_by_filename.lua");
const LIGHT_ICONS_BY_FILE_EXTENSION: &str =
    include_str!("../../Vendor/NvimWebDevicons/Light/icons_by_file_extension.lua");

#[derive(Debug)]
pub(super) struct IconDefinition {
    pub(super) glyph: String,
    pub(super) color: Option<Rgb>,
}

#[derive(Debug)]
pub(super) struct IconCatalog {
    by_filename: HashMap<String, IconDefinition>,
    by_extension: HashMap<String, IconDefinition>,
    light_by_filename: HashMap<String, IconDefinition>,
    light_by_extension: HashMap<String, IconDefinition>,
    default: IconDefinition,
}

static CATALOG: OnceLock<IconCatalog> = OnceLock::new();

pub(super) fn catalog() -> &'static IconCatalog {
    CATALOG.get_or_init(|| {
        IconCatalog::load()
            .unwrap_or_else(|error| panic!("vendored nvim-web-devicons data is invalid: {error}"))
    })
}

impl IconCatalog {
    fn load() -> Result<Self, String> {
        Ok(Self {
            by_filename: load_table(ICONS_BY_FILENAME, true)?,
            by_extension: load_table(ICONS_BY_FILE_EXTENSION, false)?,
            light_by_filename: load_table(LIGHT_ICONS_BY_FILENAME, true)?,
            light_by_extension: load_table(LIGHT_ICONS_BY_FILE_EXTENSION, false)?,
            default: IconDefinition {
                glyph: "".into(),
                color: None,
            },
        })
    }

    pub(super) fn file_icon(&self, name: &str, light: bool) -> &IconDefinition {
        let basename = Path::new(name)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(name);
        let lower = basename.to_lowercase();
        if let Some(icon) = light
            .then(|| self.light_by_filename.get(&lower))
            .flatten()
            .or_else(|| self.by_filename.get(&lower))
        {
            return icon;
        }

        let mut remainder = basename;
        while let Some((_, extension)) = remainder.split_once('.') {
            if let Some(icon) = light
                .then(|| self.light_by_extension.get(extension))
                .flatten()
                .or_else(|| self.by_extension.get(extension))
            {
                return icon;
            }
            remainder = extension;
        }

        &self.default
    }
}

pub(crate) fn matched_file_colors(
    colors: &[Rgb; 16],
    background: Rgb,
    indexed: bool,
    light: bool,
) -> HashMap<Rgb, Color> {
    let catalog = catalog();
    let tables = [
        &catalog.by_filename,
        &catalog.by_extension,
        &catalog.light_by_filename,
        &catalog.light_by_extension,
    ];

    tables
        .into_iter()
        .take(if light { 4 } else { 2 })
        .flat_map(|table| table.values())
        .filter_map(|icon| icon.color)
        .map(|target| {
            let index = crate::palette_colors::nearest_color(target, colors, background);

            (
                target,
                if indexed {
                    Color::Indexed(index as u8)
                } else {
                    colors[index].into()
                },
            )
        })
        .collect()
}

fn load_table(
    source: &str,
    lowercase_keys: bool,
) -> Result<HashMap<String, IconDefinition>, String> {
    let lua = Lua::new();
    let table = lua
        .load(source)
        .eval::<Table>()
        .map_err(|error| error.to_string())?;

    table
        .pairs::<String, Table>()
        .map(|pair| {
            let (key, value) = pair.map_err(|error| error.to_string())?;
            let glyph = value
                .get::<String>("icon")
                .map_err(|error| error.to_string())?;
            let color = value
                .get::<String>("color")
                .map_err(|error| error.to_string())
                .and_then(|color| Rgb::parse(&color))?;

            Ok((
                if lowercase_keys {
                    key.to_lowercase()
                } else {
                    key
                },
                IconDefinition {
                    glyph,
                    color: Some(color),
                },
            ))
        })
        .collect()
}
