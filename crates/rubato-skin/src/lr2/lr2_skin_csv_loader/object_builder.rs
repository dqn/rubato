use crate::reexports::Resolution;

use super::LR2SkinCSVLoaderState;

pub trait LR2SkinLoaderAccess {
    /// Get mutable reference to the base CSV loader state.
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState;

    /// Load and parse the CSV skin file, routing commands through the appropriate dispatcher.
    ///
    /// Override this in subclass loaders that have their own command routing
    /// (e.g., play skins route through `process_play_command`, select skins through
    /// `process_select_command`). The default implementation routes all commands
    /// through the base `process_csv_command`.
    fn load_skin_data(
        &mut self,
        path: &std::path::Path,
        state: Option<&dyn crate::reexports::MainState>,
    ) -> anyhow::Result<()> {
        self.csv_mut().load_skin0(path, state)
    }

    /// Assemble accumulated loader state into SkinObjects and add them to the Skin.
    /// Called after CSV parsing completes to convert parsed source data into drawable objects.
    fn assemble_objects(&mut self, skin: &mut crate::skin::Skin);

    /// Return the parsed ranktime (ms) for result/course-result skins.
    /// Defaults to 0 for non-result skin types.
    fn ranktime(&self) -> i32 {
        0
    }

    /// Apply play-skin-specific properties to the Skin struct.
    /// Only play skin loaders override this; other types use the default no-op.
    fn apply_play_properties_to_skin(&self, _skin: &mut crate::skin::Skin) {
        // default no-op for non-play skin types
    }
}

/// Create the appropriate LR2 skin loader for the given SkinType.
fn create_lr2_loader(
    skin_type: &crate::skin_type::SkinType,
    src: Resolution,
    dst: Resolution,
    usecim: bool,
    skinpath: String,
) -> Option<Box<dyn LR2SkinLoaderAccess>> {
    use crate::skin_type::SkinType;
    match skin_type {
        SkinType::MusicSelect => Some(Box::new(
            crate::lr2::lr2_select_skin_loader::LR2SelectSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::Decide => Some(Box::new(
            crate::lr2::lr2_decide_skin_loader::LR2DecideSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::Result => Some(Box::new(
            crate::lr2::lr2_result_skin_loader::LR2ResultSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::CourseResult => Some(Box::new(
            crate::lr2::lr2_course_result_skin_loader::LR2CourseResultSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        SkinType::SkinSelect => Some(Box::new(
            crate::lr2::lr2_skin_select_skin_loader::LR2SkinSelectSkinLoaderState::new(
                src, dst, usecim, skinpath,
            ),
        )),
        st if st.is_play() => Some(Box::new(
            crate::lr2::lr2_play_skin_loader::LR2PlaySkinLoaderState::new(
                *st, src, dst, usecim, skinpath,
            ),
        )),
        _ => None,
    }
}

/// Load an LR2 skin from a .lr2skin file path.
///
/// Pipeline: header load -> loader create -> CSV parse -> apply properties -> assemble objects -> return Skin.
pub fn load_lr2_skin(
    path: &std::path::Path,
    skin_type: &crate::skin_type::SkinType,
    dst: Resolution,
) -> Option<crate::skin::Skin> {
    use crate::skin_header::{self, SkinHeader};

    let skinpath = path.parent()?.to_str()?.to_string();

    // 1. Load header
    let mut header_loader = crate::lr2::lr2_skin_header_loader::LR2SkinHeaderLoader::new(&skinpath);
    let header_data = header_loader.load_skin(path, None).ok()?;

    // 2. Build SkinHeader from LR2SkinHeaderData
    let mut skin_header = SkinHeader::new();
    skin_header.skin_type_id = skin_header::TYPE_LR2SKIN;
    if let Some(st) = header_data.skin_type {
        skin_header.set_skin_type(st);
    }
    skin_header.set_name(header_data.name.clone());
    skin_header.set_author(header_data.author.clone());
    skin_header.set_path(path.to_path_buf());
    if let Some(ref res) = header_data.resolution {
        skin_header.resolution = res.clone();
    }
    // Convert lr2_skin_header_loader custom types -> skin_header custom types
    let options: Vec<skin_header::CustomOption> = header_data
        .custom_options
        .iter()
        .map(|o| {
            skin_header::CustomOption::new(o.name.clone(), o.option.clone(), o.contents.clone())
        })
        .collect();
    skin_header.options = options;
    let files: Vec<skin_header::CustomFile> = header_data
        .custom_files
        .iter()
        .map(|f| skin_header::CustomFile::new(f.name.clone(), f.path.clone(), f.def.clone()))
        .collect();
    skin_header.files = files;
    let offsets: Vec<skin_header::CustomOffset> = header_data
        .custom_offsets
        .iter()
        .map(|o| skin_header::CustomOffset::new(o.name.clone(), o.id, o.caps))
        .collect();
    skin_header.offsets = offsets;

    // 3. Create Skin
    let mut skin = crate::skin::Skin::new(skin_header);

    // 4. Create appropriate loader and parse CSV
    let src = header_data.resolution.unwrap_or(Resolution {
        width: 640.0,
        height: 480.0,
    });
    let mut loader = create_lr2_loader(skin_type, src, dst, false, skinpath)?;

    // Transfer header options to loader's op map
    for option in &header_data.custom_options {
        for &opt in &option.option {
            let val = if option.selected_option() == opt {
                1
            } else {
                0
            };
            loader.csv_mut().base.op.insert(opt, val);
        }
    }

    // Transfer custom file mappings to loader's filemap
    for file in &header_data.custom_files {
        if let Some(filename) = file.selected_filename() {
            loader
                .csv_mut()
                .filemap
                .insert(file.path.clone(), filename.to_string());
        }
    }

    // Parse the CSV file (routes through subclass-specific command dispatcher)
    if let Err(e) = loader.load_skin_data(path, None) {
        log::warn!("LR2 CSV skin load failed: {}: {}", path.display(), e);
        return None;
    }

    // 5. Apply accumulated properties to skin
    loader.csv_mut().apply_to_skin(&mut skin);

    // 6. Assemble parsed source data into SkinObjects
    loader.assemble_objects(&mut skin);

    // 7. Transfer play-skin-specific properties
    loader.apply_play_properties_to_skin(&mut skin);

    // 8. Transfer result-skin-specific ranktime
    skin.ranktime = loader.ranktime();

    Some(skin)
}
