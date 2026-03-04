use std::collections::HashSet;

use super::bar::Bar;
use super::selectable_bar::SelectableBarData;
use crate::select::stubs::*;

/// Song bar for individual songs
/// Translates: bms.player.beatoraja.select.bar.SongBar
#[derive(Clone)]
pub struct SongBar {
    pub selectable: SelectableBarData,
    /// Song data
    pub song: SongData,
    /// Banner data
    pub banner: Option<Pixmap>,
    /// Stage file data
    pub stagefile: Option<Pixmap>,
}

impl SongBar {
    pub fn new(song: SongData) -> Self {
        Self {
            selectable: SelectableBarData::default(),
            song,
            banner: None,
            stagefile: None,
        }
    }

    pub fn get_song_data(&self) -> &SongData {
        &self.song
    }

    pub fn exists_song(&self) -> bool {
        self.song.get_path().is_some()
    }

    pub fn get_banner(&self) -> Option<&Pixmap> {
        self.banner.as_ref()
    }

    pub fn set_banner(&mut self, banner: Option<Pixmap>) {
        self.banner = banner;
    }

    pub fn get_stagefile(&self) -> Option<&Pixmap> {
        self.stagefile.as_ref()
    }

    pub fn set_stagefile(&mut self, stagefile: Option<Pixmap>) {
        self.stagefile = stagefile;
    }

    pub fn get_title(&self) -> String {
        self.song.full_title()
    }

    pub fn get_lamp(&self, is_player: bool) -> i32 {
        let score = if is_player {
            self.selectable.bar_data.get_score()
        } else {
            self.selectable.bar_data.get_rival_score()
        };
        if let Some(score) = score {
            return score.get_clear();
        }
        0
    }

    /// Convert SongData slice to SongBar vec, removing duplicates by sha256
    pub fn to_song_bar_array(songs: &[SongData]) -> Vec<Bar> {
        // Remove duplicates by sha256, preserving insertion order
        let mut seen = HashSet::new();
        let mut filtered_songs = Vec::new();
        for song in songs {
            let key = song.get_sha256().to_string();
            if seen.insert(key) {
                filtered_songs.push(song.clone());
            }
        }

        let mut result = Vec::with_capacity(filtered_songs.len());
        for song in filtered_songs {
            result.push(Bar::Song(Box::new(SongBar::new(song))));
        }
        // Java fills backward (count-- index), reversing order
        result.reverse();
        result
    }

    /// Convert SongData slice to SongBar vec, matching against elements
    pub fn to_song_bar_array_with_elements(
        songs: &mut [Option<SongData>],
        elements: &mut [SongData],
    ) -> Vec<Bar> {
        // Remove duplicates
        let mut count = songs.len();
        let mut noexistscount = elements.len() as i32;

        for element in elements.iter_mut() {
            element.clear_path();
        }

        for i in 0..songs.len() {
            if songs[i].is_none() {
                continue;
            }
            for j in (i + 1)..songs.len() {
                if songs[j].is_some()
                    && songs[i].as_ref().unwrap().get_sha256()
                        == songs[j].as_ref().unwrap().get_sha256()
                {
                    songs[j] = None;
                    count -= 1;
                }
            }
            for j in 0..elements.len() {
                let element = &elements[j];
                if element.get_path().is_none()
                    && ((!element.get_md5().is_empty()
                        && element.get_md5() == songs[i].as_ref().unwrap().get_md5())
                        || (!element.get_sha256().is_empty()
                            && element.get_sha256() == songs[i].as_ref().unwrap().get_sha256()))
                {
                    let song_path = songs[i].as_ref().unwrap().get_path().map(|s| s.to_string());
                    elements[j].set_path_opt(song_path);
                    if let Some(ref _song) = songs[i] {
                        let elem_clone = elements[j].clone();
                        songs[i].as_mut().unwrap().merge(&elem_clone);
                    }
                    noexistscount -= 1;
                    break;
                }
            }
        }

        let total = count as i32 + noexistscount;
        let mut result: Vec<Bar> = Vec::with_capacity(total as usize);

        // Java fills backward: songs[0]→result[count-1], songs[last]→result[0]
        // Then elements[0]→result[count+noexists-1], elements[last]→result[count]
        // Layout: [songs reversed | elements reversed]
        for song in songs.iter().rev().flatten() {
            result.push(Bar::Song(Box::new(SongBar::new(song.clone()))));
        }
        for element in elements.iter().rev() {
            if element.get_path().is_none() {
                result.push(Bar::Song(Box::new(SongBar::new(element.clone()))));
            }
        }

        result
    }
}
