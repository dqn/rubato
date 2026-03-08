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
    /// Cached title (computed from song.full_title())
    title: String,
    /// Banner data
    pub banner: Option<Pixmap>,
    /// Stage file data
    pub stagefile: Option<Pixmap>,
}

impl SongBar {
    pub fn new(song: SongData) -> Self {
        let title = song.full_title();
        Self {
            selectable: SelectableBarData::default(),
            title,
            song,
            banner: None,
            stagefile: None,
        }
    }

    pub fn song_data(&self) -> &SongData {
        &self.song
    }

    pub fn exists_song(&self) -> bool {
        self.song.path().is_some()
    }

    pub fn banner(&self) -> Option<&Pixmap> {
        self.banner.as_ref()
    }

    pub fn stagefile(&self) -> Option<&Pixmap> {
        self.stagefile.as_ref()
    }

    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn lamp(&self, is_player: bool) -> i32 {
        let score = if is_player {
            self.selectable.bar_data.score()
        } else {
            self.selectable.bar_data.rival_score()
        };
        if let Some(score) = score {
            return score.clear;
        }
        0
    }

    /// Convert SongData slice to SongBar vec, removing duplicates by sha256
    pub fn to_song_bar_array(songs: &[SongData]) -> Vec<Bar> {
        // Remove duplicates by sha256, preserving insertion order
        let mut seen = HashSet::new();
        let mut filtered_songs = Vec::new();
        for song in songs {
            let key = song.file.sha256.clone();
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
                    && songs[i]
                        .as_ref()
                        .expect("song is Some after is_none guard")
                        .file
                        .sha256
                        == songs[j]
                            .as_ref()
                            .expect("song is Some after is_none guard")
                            .file
                            .sha256
                {
                    songs[j] = None;
                    count -= 1;
                }
            }
            for element in elements.iter_mut() {
                if element.path().is_none()
                    && ((!element.file.md5.is_empty()
                        && element.file.md5
                            == songs[i]
                                .as_ref()
                                .expect("song is Some after is_none guard")
                                .file
                                .md5)
                        || (!element.file.sha256.is_empty()
                            && element.file.sha256
                                == songs[i]
                                    .as_ref()
                                    .expect("song is Some after is_none guard")
                                    .file
                                    .sha256))
                {
                    let song_path = songs[i]
                        .as_ref()
                        .expect("song is Some after is_none guard")
                        .path()
                        .map(|s| s.to_string());
                    element.set_path_opt(song_path);
                    if let Some(ref _song) = songs[i] {
                        let elem_clone = element.clone();
                        songs[i]
                            .as_mut()
                            .expect("song is Some after is_none guard")
                            .merge(&elem_clone);
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
            if element.path().is_none() {
                result.push(Bar::Song(Box::new(SongBar::new(element.clone()))));
            }
        }

        result
    }
}
