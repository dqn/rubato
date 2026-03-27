// Skin header loading and LR2 conversion helpers.
// Dispatches to JSON, Lua, or LR2 loaders based on file extension.

use std::path::Path;

use log::error;

use crate::core::config::Config;
use rubato_skin::skin_header::SkinHeader;

/// Load a skin header from a file path.
///
/// Dispatches to the correct loader based on file extension:
/// - `.json` -> JSONSkinLoader
/// - `.luaskin` -> LuaSkinLoader
/// - other (`.lr2skin`) -> LR2SkinHeaderLoader
///
/// Returns `None` if the file cannot be parsed as a valid skin header.
pub fn load_skin_header(path: &Path, config: &Config) -> Option<SkinHeader> {
    use rubato_skin::json::json_skin_loader::JSONSkinLoader;
    use rubato_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderLoader;
    use rubato_skin::lua::lua_skin_loader::LuaSkinLoader;
    use rubato_skin::reexports::Resolution;
    use rubato_skin::skin_data_converter::convert_header_data;

    let path_string = path.to_string_lossy().to_lowercase();
    let default_src = Resolution {
        width: 1280.0,
        height: 720.0,
    };
    let default_dst = Resolution {
        width: 1920.0,
        height: 1080.0,
    };

    if path_string.ends_with(".json") {
        let mut loader = JSONSkinLoader::new();
        let header_data = loader.load_header(path)?;
        let src = header_data.source_resolution.clone().unwrap_or(default_src);
        Some(convert_header_data(&header_data, &src, &default_dst))
    } else if path_string.ends_with(".luaskin") {
        let mut loader = LuaSkinLoader::new();
        let header_data = loader.load_header(path)?;
        let src = header_data.source_resolution.clone().unwrap_or(default_src);
        Some(convert_header_data(&header_data, &src, &default_dst))
    } else {
        let mut loader = LR2SkinHeaderLoader::new(&config.paths.skinpath);
        match loader.load_skin(path, None) {
            Ok(lr2_data) => Some(convert_lr2_header_data(&lr2_data)),
            Err(e) => {
                error!("Failed to load LR2 skin header {:?}: {}", path, e);
                None
            }
        }
    }
}

/// Convert LR2SkinHeaderData to SkinHeader.
///
/// Maps the LR2-specific header data types to the common SkinHeader type
/// used by the launcher UI.
pub(super) fn convert_lr2_header_data(
    data: &rubato_skin::lr2::lr2_skin_header_loader::LR2SkinHeaderData,
) -> SkinHeader {
    use rubato_skin::skin_header::{CustomFile, CustomOffset, CustomOption, TYPE_LR2SKIN};

    let mut header = SkinHeader::default();

    header.skin_type_id = TYPE_LR2SKIN;

    if let Some(skin_type) = data.skin_type {
        header.set_skin_type(skin_type);
    }

    header.set_name(data.name.clone());
    header.set_author(data.author.clone());

    if let Some(ref path) = data.path {
        header.set_path(path.clone());
    }

    if let Some(ref res) = data.resolution {
        header.resolution = res.clone();
        header.set_source_resolution(res.clone());
    }

    // Convert custom options
    let options: Vec<CustomOption> = data
        .custom_options
        .iter()
        .map(convert_lr2_custom_option)
        .collect();
    header.options = options;

    // Convert custom files
    let files: Vec<CustomFile> = data
        .custom_files
        .iter()
        .map(convert_lr2_custom_file)
        .collect();
    header.files = files;

    // Convert custom offsets
    let offsets: Vec<CustomOffset> = data
        .custom_offsets
        .iter()
        .map(convert_lr2_custom_offset)
        .collect();
    header.offsets = offsets;

    header
}

fn convert_lr2_custom_option(
    o: &rubato_skin::lr2::lr2_skin_header_loader::CustomOption,
) -> rubato_skin::skin_header::CustomOption {
    rubato_skin::skin_header::CustomOption::new(
        o.name.clone(),
        o.option.clone(),
        o.contents.clone(),
    )
}

fn convert_lr2_custom_file(
    f: &rubato_skin::lr2::lr2_skin_header_loader::CustomFile,
) -> rubato_skin::skin_header::CustomFile {
    rubato_skin::skin_header::CustomFile::new(f.name.clone(), f.path.clone(), f.def.clone())
}

fn convert_lr2_custom_offset(
    o: &rubato_skin::lr2::lr2_skin_header_loader::CustomOffset,
) -> rubato_skin::skin_header::CustomOffset {
    rubato_skin::skin_header::CustomOffset::new(o.name.clone(), o.id, o.caps)
}
