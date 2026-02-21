use crate::stubs::{Bar, ImBoolean, MusicSelector, ScoreData, SongBar, SongData};

use std::sync::Mutex;

static SELECTOR: Mutex<Option<MusicSelector>> = Mutex::new(None);
static CURRENT_REVERSE_LOOKUP_LIST: Mutex<Vec<String>> = Mutex::new(Vec::new());
static LAST_PLAYED_SORT: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });

pub struct SongManagerMenu;

impl SongManagerMenu {
    pub fn show(_show_song_manager: &mut ImBoolean) {
        let current_song_data = get_current_song_data();
        let current_score_data = get_current_score_data();

        // if (ImGui.begin("Song Manager", showSongManager, ImGuiWindowFlags.AlwaysAutoResize))
        {
            let song_name = current_song_data
                .as_ref()
                .map(|sd| sd.get_title().to_string())
                .unwrap_or_default();

            let last_play_record_time = current_score_data
                .as_ref()
                .map(|sd| {
                    // Date date = new Date(scoreData.getDate() * 1000L);
                    // return simpleDateFormat.format(date);
                    let timestamp = sd.get_date();
                    // Simple formatting stub
                    format!("{}", timestamp)
                })
                .unwrap_or_else(|| "Not played".to_string());

            // ImGui.text("current picking: " + songName);
            // ImGui.text("Last played: " + lastPlayRecordTime);
            let _ = last_play_record_time;

            // if (ImGui.checkbox("Sort by last played", LAST_PLAYED_SORT))
            {
                // selector.getBarManager().updateBar();
            }

            if song_name.is_empty() {
                // ImGui.text("Not a selectable song");
            } else {
                // if (ImGui.button("Show Reverse Lookup"))
                {
                    update_reverse_lookup_data(&current_song_data);
                    // ImGui.openPopup("Reverse Lookup");
                }
                // if (ImGui.beginPopup("Reverse Lookup", ...))
                {
                    let list = CURRENT_REVERSE_LOOKUP_LIST.lock().unwrap();
                    for (i, item) in list.iter().enumerate() {
                        let _ = (i, item);
                        // ImGui.pushID(i);
                        // ImGui.bulletText(item);
                        // ImGui.popID();
                    }
                    // ImGui.endPopup();
                }
            }
        }
        // ImGui.end();
        todo!("SongManagerMenu::show - egui integration")
    }

    pub fn inject_music_selector(music_selector: MusicSelector) {
        *SELECTOR.lock().unwrap() = Some(music_selector);
    }

    pub fn is_last_played_sort_enabled() -> bool {
        LAST_PLAYED_SORT.lock().unwrap().get()
    }

    pub fn force_disable_last_played_sort() {
        LAST_PLAYED_SORT.lock().unwrap().set(false);
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
    if let Some(ref _sel) = *selector {
        // if (selector.getSelectedBar() instanceof SongBar)
        // { final SongData sd = ((SongBar) selector.getSelectedBar()).getSongData(); ... }
        // Stubbed - would need dynamic dispatch
    }
    None
}

fn get_current_score_data() -> Option<ScoreData> {
    let selector = SELECTOR.lock().unwrap();
    if let Some(ref _sel) = *selector {
        // if (selector.getSelectedBar() instanceof SongBar)
        // { final ScoreData sd = ((SongBar) selector.getSelectedBar()).getScore(); ... }
        // Stubbed - would need dynamic dispatch
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
