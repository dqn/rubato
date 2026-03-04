use std::collections::VecDeque;
use std::sync::Mutex;

use super::selectable_bar::SelectableBarData;
use crate::select::stubs::*;

/// Queue length for random index generation
const QUEUE_LENGTH: usize = 1000;

/// Bar to resolve when selecting a song (random select)
/// Translates: bms.player.beatoraja.select.bar.ExecutableBar
pub struct ExecutableBar {
    pub selectable: SelectableBarData,
    /// Bar title
    pub title: String,
    /// Source songs
    pub songs: Vec<SongData>,
    /// Queue for random index
    pub queue: Mutex<VecDeque<usize>>,
    /// Current song
    pub current_song: Mutex<Option<SongData>>,
}

impl Clone for ExecutableBar {
    fn clone(&self) -> Self {
        let queue = self.queue.lock().unwrap().clone();
        let current_song = self.current_song.lock().unwrap().clone();
        Self {
            selectable: self.selectable.clone(),
            title: self.title.clone(),
            songs: self.songs.clone(),
            queue: Mutex::new(queue),
            current_song: Mutex::new(current_song),
        }
    }
}

impl ExecutableBar {
    pub fn new(songs: Vec<SongData>, title: String) -> Self {
        let bar = Self {
            selectable: SelectableBarData::default(),
            title,
            songs,
            queue: Mutex::new(VecDeque::new()),
            current_song: Mutex::new(None),
        };
        bar.create_index_queue();
        bar
    }

    pub fn get_song_data(&self) -> SongData {
        self._get_song_data()
    }

    fn _get_song_data(&self) -> SongData {
        let mut queue = self.queue.lock().unwrap();
        if queue.is_empty() {
            drop(queue);
            self.create_index_queue();
            queue = self.queue.lock().unwrap();
        }

        // In Java: if (state instanceof MusicSelector || currentSong == null)
        // Simplified: always get next random song
        let mut current = self.current_song.lock().unwrap();
        let index = queue.pop_front().unwrap();
        *current = Some(self.songs[index].clone());
        current.as_ref().unwrap().clone()
    }

    fn create_index_queue(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
        for _ in 0..(QUEUE_LENGTH - 1) {
            let index = (rand::random::<f64>() * self.songs.len() as f64) as usize;
            queue.push_back(index);
        }
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    pub fn get_lamp(&self, _is_player: bool) -> i32 {
        0
    }
}
