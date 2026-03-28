use super::{ScoreData, SongData, SongSelectionAccess};
use rubato_skin::last_played_sort;
use rubato_skin::sync_utils::lock_or_recover;

use std::sync::Mutex;

static SELECTOR: Mutex<Option<Box<dyn SongSelectionAccess>>> = Mutex::new(None);

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
                    .map(|sd| sd.metadata.title.clone())
                    .unwrap_or_default();

                let last_play_record_time = current_score_data
                    .as_ref()
                    .map(|sd| format!("{}", sd.date))
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
        *lock_or_recover(&SELECTOR) = Some(selector);
    }

    pub fn is_last_played_sort_enabled() -> bool {
        last_played_sort::is_enabled()
    }

    pub fn force_disable_last_played_sort() {
        last_played_sort::force_disable();
    }
}

fn get_current_song_data() -> Option<SongData> {
    let selector = lock_or_recover(&SELECTOR);
    if let Some(ref sel) = *selector {
        return sel.selected_song_data();
    }
    None
}

fn get_current_score_data() -> Option<ScoreData> {
    let selector = lock_or_recover(&SELECTOR);
    if let Some(ref sel) = *selector {
        return sel.selected_score_data();
    }
    None
}
