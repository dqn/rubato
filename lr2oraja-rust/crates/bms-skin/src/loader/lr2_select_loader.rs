// LR2 Select skin loader.
//
// Handles state-specific commands for the music selection screen:
// - SRC_BAR_BODY / DST_BAR_BODY_OFF / DST_BAR_BODY_ON — Bar body images
// - BAR_CENTER / BAR_AVAILABLE — Bar positioning
// - SRC_BAR_LAMP / DST_BAR_LAMP — Clear lamps
// - SRC_BAR_MY_LAMP / DST_BAR_MY_LAMP — Player lamps
// - SRC_BAR_RIVAL_LAMP / DST_BAR_RIVAL_LAMP — Rival lamps
// - SRC_BAR_LEVEL / DST_BAR_LEVEL — Level display
// - SRC_BAR_TROPHY / DST_BAR_TROPHY — Trophy icons
// - SRC_BAR_LABEL / DST_BAR_LABEL — Label images
// - SRC_BAR_TITLE / DST_BAR_TITLE — Title text
// - SRC_NOTECHART / DST_NOTECHART — Note distribution chart
// - SRC_BPMCHART / DST_BPMCHART — BPM chart
// - SRC_BAR_FLASH / SRC_BAR_RANK / SRC_README — Stubs
//
// Ported from LR2SelectSkinLoader.java.

use crate::image_handle::ImageHandle;
use crate::loader::lr2_csv_loader::{Lr2CsvState, nonzero_timer, parse_field, parse_int_pub};
use crate::music_select_skin::MusicSelectSkinConfig;
use crate::skin::Skin;
use crate::skin_bar::{
    BAR_COUNT, BAR_LABEL_COUNT, BAR_LAMP_COUNT, BAR_LEVEL_COUNT, BAR_TEXT_COUNT, BAR_TROPHY_COUNT,
    SkinBar,
};
use crate::skin_bpm_graph::SkinBpmGraph;
use crate::skin_distribution_graph::SkinDistributionGraph;
use crate::skin_image::SkinImage;
use crate::skin_number::{SkinNumber, ZeroPadding};
use crate::skin_source::{build_number_source_set, split_grid};
use crate::skin_text::{FontType, SkinText, TextAlign};

// ---------------------------------------------------------------------------
// Lamp group mapping (Java lampg table)
// ---------------------------------------------------------------------------

/// Maps LR2 lamp IDs to beatoraja lamp IDs.
/// Index = LR2 lamp ID, value = list of beatoraja lamp IDs to set.
const LAMPG: &[&[usize]] = &[
    &[0],        // 0: NO PLAY
    &[1],        // 1: FAILED
    &[4, 2, 3],  // 2: EASY (maps to EASY, ASSIST_EASY, LIGHT_ASSIST_EASY)
    &[5],        // 3: NORMAL
    &[6, 7],     // 4: HARD (maps to HARD, EXHARD)
    &[7],        // 5: EXH
    &[8, 9, 10], // 6: FC (maps to FC, PERFECT, MAX)
    &[9],        // 7: PERFECT
    &[10],       // 8: MAX
    &[2],        // 9: ASSIST
    &[3],        // 10: L-ASSIST
];

// ---------------------------------------------------------------------------
// Select state
// ---------------------------------------------------------------------------

/// Internal state for select skin loading.
#[derive(Default)]
pub struct Lr2SelectState {
    /// The song bar being constructed.
    pub skinbar: SkinBar,
    /// Bar image textures [up to 10 sets], each containing animation frame regions.
    barimage: Vec<Option<Vec<ImageHandle>>>,
    /// Bar animation cycle (ms).
    barcycle: i32,
    /// On-state bar images [BAR_COUNT] — lazily created on first DST_BAR_BODY_ON.
    barimageon: Vec<Option<SkinImage>>,
    /// Off-state bar images [BAR_COUNT] — lazily created on first DST_BAR_BODY_OFF.
    barimageoff: Vec<Option<SkinImage>>,
    /// Whether the SkinBar has been added to skin.objects.
    bar_added: bool,
    /// Index of the SkinBar in skin.objects.
    bar_obj_idx: Option<usize>,
    /// Temporary lamp images per LR2 lamp ID.
    lamp_images: Vec<Option<SkinImage>>,
    /// Temporary player lamp images per LR2 lamp ID.
    my_lamp_images: Vec<Option<SkinImage>>,
    /// Temporary rival lamp images per LR2 lamp ID.
    rival_lamp_images: Vec<Option<SkinImage>>,
    /// Center bar index (set by BAR_CENTER).
    center_bar: usize,
    /// Clickable bar indices (set by BAR_AVAILABLE).
    clickable_bar: Vec<usize>,
    /// Note chart object index.
    note_chart_idx: Option<usize>,
    /// BPM chart object index.
    bpm_chart_idx: Option<usize>,
}

impl Lr2SelectState {
    #[allow(dead_code)] // Used in tests; production uses Default with lazy initialization
    fn new() -> Self {
        Self {
            barimage: vec![None; 10],
            barimageon: vec![None; BAR_COUNT],
            barimageoff: vec![None; BAR_COUNT],
            lamp_images: vec![None; BAR_LAMP_COUNT],
            my_lamp_images: vec![None; BAR_LAMP_COUNT],
            rival_lamp_images: vec![None; BAR_LAMP_COUNT],
            ..Default::default()
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: create SkinImage from barimage array
// ---------------------------------------------------------------------------

/// Creates a SkinImage from the bar image sets. Each set is a
/// SkinImageSource::Frames with the same cycle. Matches Java:
/// `new SkinImage(barimage, 0, barcycle, null)`
fn make_bar_skin_image(barimage: &[Option<Vec<ImageHandle>>], barcycle: i32) -> SkinImage {
    use crate::skin_image::SkinImageSource;

    let sources: Vec<SkinImageSource> = barimage
        .iter()
        .filter_map(|opt| {
            opt.as_ref().map(|images| SkinImageSource::Frames {
                images: images.clone(),
                timer: None,
                cycle: barcycle,
            })
        })
        .collect();

    SkinImage {
        sources,
        ..Default::default()
    }
}

/// Creates a SkinImage from inline frames with timer and cycle.
fn make_lamp_skin_image(images: Vec<ImageHandle>, timer: Option<i32>, cycle: i32) -> SkinImage {
    SkinImage::from_frames(images, timer, cycle)
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Processes a select-screen specific LR2 command.
///
/// Returns true if the command was handled.
pub fn process_select_command(
    cmd: &str,
    fields: &[&str],
    skin: &mut Skin,
    state: &mut Lr2CsvState,
    select_state: &mut Lr2SelectState,
) -> bool {
    match cmd {
        // -- Bar body --
        "SRC_BAR_BODY" => {
            src_bar_body(fields, state, select_state);
            true
        }
        "DST_BAR_BODY_OFF" => {
            dst_bar_body_off(fields, skin, state, select_state);
            true
        }
        "DST_BAR_BODY_ON" => {
            dst_bar_body_on(fields, skin, state, select_state);
            true
        }
        "BAR_CENTER" => {
            select_state.center_bar = parse_field(fields, 1) as usize;
            true
        }
        "BAR_AVAILABLE" => {
            let start = parse_field(fields, 1);
            let end = parse_field(fields, 2);
            if start >= 0 && end >= start {
                select_state.clickable_bar = (start as usize..=end as usize).collect();
            }
            true
        }

        // -- Lamps --
        "SRC_BAR_LAMP" => {
            src_bar_lamp(fields, state, select_state);
            true
        }
        "DST_BAR_LAMP" => {
            dst_bar_lamp(fields, state, select_state);
            true
        }
        "SRC_BAR_MY_LAMP" => {
            src_bar_my_lamp(fields, state, select_state);
            true
        }
        "DST_BAR_MY_LAMP" => {
            dst_bar_my_lamp(fields, state, select_state);
            true
        }
        "SRC_BAR_RIVAL_LAMP" => {
            src_bar_rival_lamp(fields, state, select_state);
            true
        }
        "DST_BAR_RIVAL_LAMP" => {
            dst_bar_rival_lamp(fields, state, select_state);
            true
        }

        // -- Level --
        "SRC_BAR_LEVEL" => {
            src_bar_level(fields, state, select_state);
            true
        }
        "DST_BAR_LEVEL" => {
            dst_bar_level(fields, state, select_state);
            true
        }

        // -- Trophy --
        "SRC_BAR_TROPHY" => {
            src_bar_trophy(fields, state, select_state);
            true
        }
        "DST_BAR_TROPHY" => {
            dst_bar_trophy(fields, state, select_state);
            true
        }

        // -- Label --
        "SRC_BAR_LABEL" => {
            src_bar_label(fields, state, select_state);
            true
        }
        "DST_BAR_LABEL" => {
            dst_bar_label(fields, state, select_state);
            true
        }

        // -- Title text --
        "SRC_BAR_TITLE" => {
            src_bar_title(fields, state, select_state);
            true
        }
        "DST_BAR_TITLE" => {
            dst_bar_title(fields, state, select_state);
            true
        }

        // -- Charts --
        "SRC_NOTECHART" => {
            let graph = SkinDistributionGraph::default();
            let idx = skin.objects.len();
            skin.add(graph.into());
            select_state.note_chart_idx = Some(idx);
            true
        }
        "DST_NOTECHART" => {
            if let Some(idx) = select_state.note_chart_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }
        "SRC_BPMCHART" => {
            let graph = SkinBpmGraph::default();
            let idx = skin.objects.len();
            skin.add(graph.into());
            select_state.bpm_chart_idx = Some(idx);
            true
        }
        "DST_BPMCHART" => {
            if let Some(idx) = select_state.bpm_chart_idx {
                state.apply_dst_to(idx, fields, skin);
            }
            true
        }

        // -- Stubs (Java also has empty implementations) --
        "SRC_BAR_FLASH" | "DST_BAR_FLASH" | "SRC_BAR_RANK" | "DST_BAR_RANK" | "SRC_README"
        | "DST_README" => true,

        _ => false,
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_BODY / DST_BAR_BODY_OFF / DST_BAR_BODY_ON
// ---------------------------------------------------------------------------

fn src_bar_body(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let idx = values[1] as usize;
    if idx >= 10 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );

    // Convert ImageRegion grid into ImageHandle Vec (one handle per frame).
    // For bar body, each grid cell is a full-image handle.
    let handles: Vec<ImageHandle> = grid.iter().map(|r| r.handle).collect();
    select_state.barimage[idx] = if handles.is_empty() {
        None
    } else {
        Some(handles)
    };
    select_state.barcycle = values[9];
}

fn dst_bar_body_off(
    fields: &[&str],
    skin: &mut Skin,
    state: &Lr2CsvState,
    select_state: &mut Lr2SelectState,
) {
    let values = parse_int_pub(fields);
    let pos = values[1] as usize;
    if pos >= BAR_COUNT {
        return;
    }

    // Lazily create the SkinImage for this bar slot
    if select_state.barimageoff[pos].is_none() {
        select_state.barimageoff[pos] = Some(make_bar_skin_image(
            &select_state.barimage,
            select_state.barcycle,
        ));
    }

    // Apply DST to the bar image's base
    if let Some(ref mut img) = select_state.barimageoff[pos] {
        state.apply_dst_to_base(&mut img.base, fields, &[]);
    }

    // Add the SkinBar to skin.objects on first DST_BAR_BODY_OFF
    ensure_bar_added(skin, select_state);
}

fn dst_bar_body_on(
    fields: &[&str],
    skin: &mut Skin,
    state: &Lr2CsvState,
    select_state: &mut Lr2SelectState,
) {
    let values = parse_int_pub(fields);
    let pos = values[1] as usize;
    if pos >= BAR_COUNT {
        return;
    }

    // Lazily create the SkinImage for this bar slot
    if select_state.barimageon[pos].is_none() {
        select_state.barimageon[pos] = Some(make_bar_skin_image(
            &select_state.barimage,
            select_state.barcycle,
        ));
    }

    // Apply DST to the bar image's base
    if let Some(ref mut img) = select_state.barimageon[pos] {
        state.apply_dst_to_base(&mut img.base, fields, &[]);
    }

    ensure_bar_added(skin, select_state);
}

/// Ensures the SkinBar has been added to skin.objects exactly once.
fn ensure_bar_added(skin: &mut Skin, select_state: &mut Lr2SelectState) {
    if !select_state.bar_added {
        let idx = skin.objects.len();
        skin.add(SkinBar::default().into());
        select_state.bar_obj_idx = Some(idx);
        select_state.bar_added = true;
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_LAMP / DST_BAR_LAMP
// ---------------------------------------------------------------------------

fn src_bar_lamp(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let lr2_id = values[1] as usize;
    if lr2_id >= LAMPG.len() {
        return;
    }
    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    let handles: Vec<ImageHandle> = grid.iter().map(|r| r.handle).collect();
    if handles.is_empty() {
        return;
    }

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let img = make_lamp_skin_image(handles, timer, cycle);

    // Store at the first mapped beatoraja lamp ID
    let lamps = LAMPG[lr2_id];
    if let Some(&first_lamp) = lamps.first()
        && first_lamp < BAR_LAMP_COUNT
    {
        select_state.lamp_images[first_lamp] = Some(img);
    }
}

fn dst_bar_lamp(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let lr2_id = values[1] as usize;
    if lr2_id >= LAMPG.len() {
        return;
    }

    let lamps = LAMPG[lr2_id];
    for &lamp_id in lamps {
        if lamp_id >= BAR_LAMP_COUNT {
            continue;
        }
        if select_state.lamp_images[lamp_id].is_some() {
            // Apply DST to existing lamp
            if let Some(ref mut img) = select_state.lamp_images[lamp_id] {
                state.apply_dst_to_base(&mut img.base, fields, &[]);
            }
        } else if let Some(&first) = lamps.first() {
            // Alias to first lamp in group
            if first < BAR_LAMP_COUNT
                && let Some(first_img) = &select_state.lamp_images[first]
            {
                select_state.lamp_images[lamp_id] = Some(first_img.clone());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_MY_LAMP / DST_BAR_MY_LAMP
// ---------------------------------------------------------------------------

fn src_bar_my_lamp(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let lr2_id = values[1] as usize;
    if lr2_id >= LAMPG.len() {
        return;
    }
    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    let handles: Vec<ImageHandle> = grid.iter().map(|r| r.handle).collect();
    if handles.is_empty() {
        return;
    }

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let img = make_lamp_skin_image(handles, timer, cycle);

    let lamps = LAMPG[lr2_id];
    if let Some(&first_lamp) = lamps.first()
        && first_lamp < BAR_LAMP_COUNT
    {
        select_state.my_lamp_images[first_lamp] = Some(img);
    }
}

fn dst_bar_my_lamp(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let lr2_id = values[1] as usize;
    if lr2_id >= LAMPG.len() {
        return;
    }

    let lamps = LAMPG[lr2_id];
    for &lamp_id in lamps {
        if lamp_id >= BAR_LAMP_COUNT {
            continue;
        }
        if select_state.my_lamp_images[lamp_id].is_some() {
            if let Some(ref mut img) = select_state.my_lamp_images[lamp_id] {
                state.apply_dst_to_base(&mut img.base, fields, &[]);
            }
        } else if let Some(&first) = lamps.first()
            && first < BAR_LAMP_COUNT
            && let Some(first_img) = &select_state.my_lamp_images[first]
        {
            select_state.my_lamp_images[lamp_id] = Some(first_img.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_RIVAL_LAMP / DST_BAR_RIVAL_LAMP
// ---------------------------------------------------------------------------

fn src_bar_rival_lamp(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let lr2_id = values[1] as usize;
    if lr2_id >= LAMPG.len() {
        return;
    }
    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    let handles: Vec<ImageHandle> = grid.iter().map(|r| r.handle).collect();
    if handles.is_empty() {
        return;
    }

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let img = make_lamp_skin_image(handles, timer, cycle);

    let lamps = LAMPG[lr2_id];
    if let Some(&first_lamp) = lamps.first()
        && first_lamp < BAR_LAMP_COUNT
    {
        select_state.rival_lamp_images[first_lamp] = Some(img);
    }
}

fn dst_bar_rival_lamp(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let lr2_id = values[1] as usize;
    if lr2_id >= LAMPG.len() {
        return;
    }

    let lamps = LAMPG[lr2_id];
    for &lamp_id in lamps {
        if lamp_id >= BAR_LAMP_COUNT {
            continue;
        }
        if select_state.rival_lamp_images[lamp_id].is_some() {
            if let Some(ref mut img) = select_state.rival_lamp_images[lamp_id] {
                state.apply_dst_to_base(&mut img.base, fields, &[]);
            }
        } else if let Some(&first) = lamps.first()
            && first < BAR_LAMP_COUNT
            && let Some(first_img) = &select_state.rival_lamp_images[first]
        {
            select_state.rival_lamp_images[lamp_id] = Some(first_img.clone());
        }
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_LEVEL / DST_BAR_LEVEL
// ---------------------------------------------------------------------------

fn src_bar_level(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let level_id = values[1] as usize;
    if level_id >= BAR_LEVEL_COUNT {
        return;
    }

    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let divx = values[7].max(1);
    let divy = values[8].max(1);
    if divx * divy < 10 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    if grid.is_empty() {
        return;
    }

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let (digit_sources, minus_digit_sources, zeropadding_override) =
        build_number_source_set(&grid, timer, cycle);

    let total_frames = divx * divy;
    let (keta, zero_padding) = if total_frames % 24 == 0 {
        (
            values[13] + 1,
            ZeroPadding::from_i32(if fields.len() > 14 && !fields[14].is_empty() {
                values[14]
            } else {
                2
            }),
        )
    } else {
        (
            values[13],
            ZeroPadding::from_i32(zeropadding_override.unwrap_or(0)),
        )
    };

    let num = SkinNumber {
        keta,
        zero_padding,
        space: values[15],
        digit_sources,
        minus_digit_sources,
        image_timer: timer,
        image_cycle: cycle,
        ..Default::default()
    };

    select_state.skinbar.bar_level[level_id] = Some(num);
}

fn dst_bar_level(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let level_id = values[1] as usize;
    if level_id >= BAR_LEVEL_COUNT {
        return;
    }

    if let Some(ref mut num) = select_state.skinbar.bar_level[level_id] {
        state.apply_dst_to_base(&mut num.base, fields, &[]);
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_TROPHY / DST_BAR_TROPHY
// ---------------------------------------------------------------------------

fn src_bar_trophy(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let trophy_id = values[1] as usize;
    if trophy_id >= BAR_TROPHY_COUNT {
        return;
    }

    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    let handles: Vec<ImageHandle> = grid.iter().map(|r| r.handle).collect();
    if handles.is_empty() {
        return;
    }

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let img = make_lamp_skin_image(handles, timer, cycle);

    select_state.skinbar.trophy[trophy_id] = Some(img);
}

fn dst_bar_trophy(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let trophy_id = values[1] as usize;
    if trophy_id >= BAR_TROPHY_COUNT {
        return;
    }

    if let Some(ref mut img) = select_state.skinbar.trophy[trophy_id] {
        state.apply_dst_to_base(&mut img.base, fields, &[]);
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_LABEL / DST_BAR_LABEL
// ---------------------------------------------------------------------------

fn src_bar_label(fields: &[&str], _state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let label_id = values[1] as usize;
    if label_id >= BAR_LABEL_COUNT {
        return;
    }

    let gr = values[2];
    if gr >= 100 {
        return;
    }

    let handle = ImageHandle(gr as u32);
    let divx = values[7].max(1);
    let divy = values[8].max(1);
    let grid = split_grid(
        handle, values[3], values[4], values[5], values[6], divx, divy,
    );
    let handles: Vec<ImageHandle> = grid.iter().map(|r| r.handle).collect();
    if handles.is_empty() {
        return;
    }

    let timer = nonzero_timer(values[10]);
    let cycle = values[9];
    let img = make_lamp_skin_image(handles, timer, cycle);

    select_state.skinbar.label[label_id] = Some(img);
}

fn dst_bar_label(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let label_id = values[1] as usize;
    if label_id >= BAR_LABEL_COUNT {
        return;
    }

    if let Some(ref mut img) = select_state.skinbar.label[label_id] {
        state.apply_dst_to_base(&mut img.base, fields, &[]);
    }
}

// ---------------------------------------------------------------------------
// SRC_BAR_TITLE / DST_BAR_TITLE
// ---------------------------------------------------------------------------

fn src_bar_title(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let text_id = values[1] as usize;
    if text_id >= BAR_TEXT_COUNT {
        return;
    }

    let font_index = values[2] as usize;
    let font_type = state
        .fontlist
        .get(font_index)
        .and_then(|opt| opt.as_ref())
        .map(|key| FontType::Bitmap {
            path: key.clone(),
            bitmap_type: 0,
        })
        .unwrap_or(FontType::Default);

    let text = SkinText {
        align: TextAlign::from_i32(values[4]),
        font_type,
        font_size: 24.0,
        ..Default::default()
    };

    select_state.skinbar.text[text_id] = Some(text);
}

fn dst_bar_title(fields: &[&str], state: &Lr2CsvState, select_state: &mut Lr2SelectState) {
    let values = parse_int_pub(fields);
    let text_id = values[1] as usize;
    if text_id >= BAR_TEXT_COUNT {
        return;
    }

    if let Some(ref mut text) = select_state.skinbar.text[text_id] {
        state.apply_dst_to_base(&mut text.base, fields, &[]);
    }
}

// ---------------------------------------------------------------------------
// Finalize: copy temporary state into SkinBar
// ---------------------------------------------------------------------------

/// Finalizes the select state by copying temporary images into the SkinBar.
/// Called after all SRC/DST commands have been processed.
fn finalize_select_state(select_state: &mut Lr2SelectState) {
    // Copy bar body images
    for i in 0..BAR_COUNT {
        select_state.skinbar.bar_image_on[i] = select_state.barimageon[i].take();
        select_state.skinbar.bar_image_off[i] = select_state.barimageoff[i].take();
    }

    // Copy lamp images
    for i in 0..BAR_LAMP_COUNT {
        select_state.skinbar.lamp[i] = select_state.lamp_images[i].take();
        select_state.skinbar.my_lamp[i] = select_state.my_lamp_images[i].take();
        select_state.skinbar.rival_lamp[i] = select_state.rival_lamp_images[i].take();
    }
}

// ---------------------------------------------------------------------------
// Collect config
// ---------------------------------------------------------------------------

/// Collects select state into MusicSelectSkinConfig after loading completes.
pub fn collect_select_config(
    skin: &Skin,
    select_state: &mut Lr2SelectState,
) -> Option<MusicSelectSkinConfig> {
    finalize_select_state(select_state);

    let distribution_graph = select_state.note_chart_idx.and_then(|idx| {
        skin.objects.get(idx).and_then(|obj| {
            if let crate::skin_object_type::SkinObjectType::DistributionGraph(g) = obj {
                Some(g.clone())
            } else {
                None
            }
        })
    });
    Some(MusicSelectSkinConfig {
        bar: Some(select_state.skinbar.clone()),
        distribution_graph,
        center_bar: select_state.center_bar,
        clickable_bar: select_state.clickable_bar.clone(),
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_header::SkinHeader;
    use bms_config::resolution::Resolution;
    use std::collections::HashMap;

    fn make_skin() -> (Skin, Lr2CsvState) {
        let mut header = SkinHeader::default();
        header.resolution = Resolution::Sd;
        header.source_resolution = Some(Resolution::Sd);
        header.destination_resolution = Some(Resolution::Hd);
        let skin = Skin::new(header);
        let state = Lr2CsvState::new(Resolution::Sd, Resolution::Hd, &HashMap::new());
        (skin, state)
    }

    fn make_select_state() -> Lr2SelectState {
        Lr2SelectState::new()
    }

    #[test]
    fn test_bar_center() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let fields: Vec<&str> = "#BAR_CENTER,12".split(',').collect();
        assert!(process_select_command(
            "BAR_CENTER",
            &fields,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert_eq!(ss.center_bar, 12);
    }

    #[test]
    fn test_bar_available() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let fields: Vec<&str> = "#BAR_AVAILABLE,5,50".split(',').collect();
        assert!(process_select_command(
            "BAR_AVAILABLE",
            &fields,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert_eq!(ss.clickable_bar.len(), 46);
        assert_eq!(ss.clickable_bar[0], 5);
        assert_eq!(*ss.clickable_bar.last().unwrap(), 50);
    }

    #[test]
    fn test_bar_body_src_dst() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        // values[1]=5 (bar type idx), values[2]=0 (image gr)
        let src: Vec<&str> = "#SRC_BAR_BODY,5,0,0,0,200,30,1,1,0,0".split(',').collect();
        assert!(process_select_command(
            "SRC_BAR_BODY",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.barimage[5].is_some());

        let dst: Vec<&str> = "#DST_BAR_BODY_OFF,3,0,100,50,200,30,0,255,255,255,255"
            .split(',')
            .collect();
        assert!(process_select_command(
            "DST_BAR_BODY_OFF",
            &dst,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.barimageoff[3].is_some());
        assert!(ss.bar_added);
    }

    #[test]
    fn test_bar_body_on() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_BAR_BODY,0,3,0,0,200,30,1,1,0,0".split(',').collect();
        process_select_command("SRC_BAR_BODY", &src, &mut skin, &mut state, &mut ss);

        let dst: Vec<&str> = "#DST_BAR_BODY_ON,5,0,100,50,200,30,0,255,255,255,255"
            .split(',')
            .collect();
        assert!(process_select_command(
            "DST_BAR_BODY_ON",
            &dst,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.barimageon[5].is_some());
    }

    #[test]
    fn test_src_bar_lamp_maps_to_lampg() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        // SRC_BAR_LAMP with LR2 ID 2 (EASY) -> maps to lamp IDs [4, 2, 3]
        let src: Vec<&str> = "#SRC_BAR_LAMP,2,0,0,0,20,20,1,1,0,0".split(',').collect();
        assert!(process_select_command(
            "SRC_BAR_LAMP",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        // First lamp in LAMPG[2] = 4
        assert!(ss.lamp_images[4].is_some());
    }

    #[test]
    fn test_src_bar_level() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        // Create SRC_BAR_LEVEL with 10 cells (divx=10, divy=1)
        let src: Vec<&str> = "#SRC_BAR_LEVEL,3,0,0,0,200,20,10,1,0,0,0,0,4,0,0"
            .split(',')
            .collect();
        assert!(process_select_command(
            "SRC_BAR_LEVEL",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.skinbar.bar_level[3].is_some());
        let num = ss.skinbar.bar_level[3].as_ref().unwrap();
        assert_eq!(num.keta, 4);
    }

    #[test]
    fn test_src_bar_trophy() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_BAR_TROPHY,1,0,0,0,30,30,1,1,0,0".split(',').collect();
        assert!(process_select_command(
            "SRC_BAR_TROPHY",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.skinbar.trophy[1].is_some());
    }

    #[test]
    fn test_src_bar_label() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_BAR_LABEL,2,0,0,0,30,30,1,1,0,0".split(',').collect();
        assert!(process_select_command(
            "SRC_BAR_LABEL",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.skinbar.label[2].is_some());
    }

    #[test]
    fn test_src_bar_title() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_BAR_TITLE,0,0,0,1,0".split(',').collect();
        assert!(process_select_command(
            "SRC_BAR_TITLE",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.skinbar.text[0].is_some());
        assert_eq!(
            ss.skinbar.text[0].as_ref().unwrap().align,
            TextAlign::Center
        );
    }

    #[test]
    fn test_note_chart() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_NOTECHART,0,0,0,0,200,100,1,1,0,0"
            .split(',')
            .collect();
        assert!(process_select_command(
            "SRC_NOTECHART",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.note_chart_idx.is_some());
        assert_eq!(skin.object_count(), 1);

        let dst: Vec<&str> =
            "#DST_NOTECHART,0,0,100,50,200,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
                .split(',')
                .collect();
        assert!(process_select_command(
            "DST_NOTECHART",
            &dst,
            &mut skin,
            &mut state,
            &mut ss
        ));
    }

    #[test]
    fn test_bpm_chart() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_BPMCHART,0,0,0,0,200,100,1,1,0,0".split(',').collect();
        assert!(process_select_command(
            "SRC_BPMCHART",
            &src,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(ss.bpm_chart_idx.is_some());
    }

    #[test]
    fn test_stubs_return_true() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();
        let fields: Vec<&str> = vec!["#CMD"];

        assert!(process_select_command(
            "SRC_BAR_FLASH",
            &fields,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(process_select_command(
            "SRC_BAR_RANK",
            &fields,
            &mut skin,
            &mut state,
            &mut ss
        ));
        assert!(process_select_command(
            "SRC_README",
            &fields,
            &mut skin,
            &mut state,
            &mut ss
        ));
    }

    #[test]
    fn test_unhandled_returns_false() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();
        let fields: Vec<&str> = vec!["#UNKNOWN"];

        assert!(!process_select_command(
            "UNKNOWN", &fields, &mut skin, &mut state, &mut ss
        ));
    }

    #[test]
    fn test_collect_select_config() {
        let (skin, _) = make_skin();
        let mut ss = make_select_state();
        ss.center_bar = 7;
        ss.clickable_bar = vec![0, 1, 2, 3];

        let config = collect_select_config(&skin, &mut ss).unwrap();
        assert!(config.bar.is_some());
        assert!(config.distribution_graph.is_none());
        assert_eq!(config.center_bar, 7);
        assert_eq!(config.clickable_bar.len(), 4);
    }

    #[test]
    fn test_collect_select_config_with_distribution_graph() {
        let (mut skin, mut state) = make_skin();
        let mut ss = make_select_state();

        let src: Vec<&str> = "#SRC_NOTECHART,0,0,0,0,200,100,1,1,0,0"
            .split(',')
            .collect();
        process_select_command("SRC_NOTECHART", &src, &mut skin, &mut state, &mut ss);

        let dst: Vec<&str> =
            "#DST_NOTECHART,0,0,100,50,200,100,0,255,255,255,255,0,0,0,0,0,0,0,0,0"
                .split(',')
                .collect();
        process_select_command("DST_NOTECHART", &dst, &mut skin, &mut state, &mut ss);

        let config = collect_select_config(&skin, &mut ss).unwrap();
        assert!(config.distribution_graph.is_some());
    }

    #[test]
    fn test_finalize_copies_images() {
        let mut ss = make_select_state();
        ss.barimageon[0] = Some(SkinImage::default());
        ss.barimageoff[0] = Some(SkinImage::default());
        ss.lamp_images[0] = Some(SkinImage::default());
        ss.my_lamp_images[0] = Some(SkinImage::default());
        ss.rival_lamp_images[0] = Some(SkinImage::default());

        finalize_select_state(&mut ss);

        assert!(ss.skinbar.bar_image_on[0].is_some());
        assert!(ss.skinbar.bar_image_off[0].is_some());
        assert!(ss.skinbar.lamp[0].is_some());
        assert!(ss.skinbar.my_lamp[0].is_some());
        assert!(ss.skinbar.rival_lamp[0].is_some());
    }
}
