use super::stubs::{ScoreData, SongData, SongSelectionAccess};
use beatoraja_types::last_played_sort;

use std::sync::Mutex;

static SELECTOR: Mutex<Option<Box<dyn SongSelectionAccess>>> = Mutex::new(None);
static CURRENT_REVERSE_LOOKUP_LIST: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub struct SongManagerMenu;

impl SongManagerMenu {
    /// Render the song manager window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let current_song_data = get_current_song_data();
        let current_score_data = get_current_score_data();

        let mut open = true;
        egui::Window::new("Song Manager")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                let song_name = current_song_data
                    .as_ref()
                    .map(|sd| sd.get_title().to_string())
                    .unwrap_or_default();

                let last_play_record_time = current_score_data
                    .as_ref()
                    .map(|sd| format!("{}", sd.get_date()))
                    .unwrap_or_else(|| "N/A".to_string());

                if song_name.is_empty() {
                    ui.label("Not a selectable song");
                } else {
                    ui.label(format!("Song: {}", song_name));
                    ui.label(format!("Last played: {}", last_play_record_time));

                    let mut sort = last_played_sort::is_enabled();
                    ui.checkbox(&mut sort, "Sort by last played");
                    last_played_sort::set(sort);
                }
            });
    }

    pub fn inject_music_selector(selector: Box<dyn SongSelectionAccess>) {
        *SELECTOR.lock().unwrap() = Some(selector);
    }

    pub fn is_last_played_sort_enabled() -> bool {
        last_played_sort::is_enabled()
    }

    pub fn force_disable_last_played_sort() {
        last_played_sort::force_disable();
    }
}

fn update_reverse_lookup_data(current_song_data: &Option<SongData>) {
    if current_song_data.is_none() {
        CURRENT_REVERSE_LOOKUP_LIST.lock().unwrap().clear();
        return;
    }

    // Current song data is not used in this call, consider deleting upstream of this function
    // getReverseLookupData uses the selectors resource object to get data for what song is currently selected
    *CURRENT_REVERSE_LOOKUP_LIST.lock().unwrap() = get_reverse_lookup_data();
}

fn get_current_song_data() -> Option<SongData> {
    let selector = SELECTOR.lock().unwrap();
    if let Some(ref sel) = *selector {
        return sel.get_selected_song_data();
    }
    None
}

fn get_current_score_data() -> Option<ScoreData> {
    let selector = SELECTOR.lock().unwrap();
    if let Some(ref sel) = *selector {
        return sel.get_selected_score_data();
    }
    None
}

fn get_reverse_lookup_data() -> Vec<String> {
    let selector = SELECTOR.lock().unwrap();
    if let Some(ref sel) = *selector {
        return sel.get_reverse_lookup_data();
    }
    Vec::new()
}
