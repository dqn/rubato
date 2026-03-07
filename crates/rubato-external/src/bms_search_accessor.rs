use serde::Deserialize;

use crate::stubs::{SongData, TableAccessor, TableData, TableDataAccessor, TableFolder};

/// BMS Search accessor class.
/// Translated from Java: BMSSearchAccessor extends TableDataAccessor.TableAccessor
///
/// @see <a href="https://bmssearch.stoplight.io/docs/bmssearch-api/YXBpOjg0MzMw-bms-search-api">bmssearch-api</a>
pub struct BMSSearchAccessor {
    tabledir: String,
}

static API_STRING: &str =
    "https://api.bmssearch.net/v1/bmses/search?orderBy=PUBLISHED&orderDirection=DESC&limit=20";

impl BMSSearchAccessor {
    pub fn new(tabledir: &str) -> Self {
        Self {
            tabledir: tabledir.to_string(),
        }
    }

    fn read_impl(&self) -> Option<TableData> {
        let mut td: Option<TableData> = None;
        match Self::fetch_elements() {
            Ok(elements) => {
                let mut tde_new = TableFolder {
                    name: Some("New".to_string()),
                    ..Default::default()
                };
                let mut songs: Vec<SongData> = Vec::new();
                for element in &elements {
                    let mut song = SongData::default();
                    song.title = element.title.clone().unwrap_or_default();
                    song.set_artist(element.artist.clone().unwrap_or_default());
                    song.genre = element.genre.clone().unwrap_or_default();
                    if let Some(ref downloads) = element.downloads
                        && !downloads.is_empty()
                        && let Some(ref url) = downloads[0].url
                    {
                        song.set_url(url.clone());
                    }

                    // MD5 fetch
                    if let Some(ref id) = element.id {
                        match Self::fetch_patterns(id) {
                            Ok(patterns) => {
                                if !patterns.is_empty()
                                    && let Some(ref file) = patterns[0].file
                                    && let Some(ref hash_md5) = file.hash_md5
                                {
                                    song.md5 = hash_md5.clone();
                                }
                            }
                            Err(e) => {
                                log::error!("BMS Search pattern fetch error: {}", e);
                            }
                        }
                    }

                    songs.push(song);
                }
                tde_new.songs = songs;
                let table_data = TableData {
                    name: "BMS Search".to_string(),
                    folder: vec![tde_new],
                    ..Default::default()
                };
                log::info!("BMS Search fetch complete");
                td = Some(table_data);
            }
            Err(e) => {
                log::error!("BMS Search update exception: {}", e);
            }
        }
        td
    }

    fn fetch_elements() -> anyhow::Result<Vec<BMSSearchElement>> {
        let body = reqwest::blocking::get(API_STRING)?.text()?;
        let elements: Vec<BMSSearchElement> = serde_json::from_str(&body)?;
        Ok(elements)
    }

    fn fetch_patterns(id: &str) -> anyhow::Result<Vec<BMSPatterns>> {
        let url = format!("https://api.bmssearch.net/v1/bmses/{}/patterns?limit=1", id);
        let body = reqwest::blocking::get(&url)?.text()?;
        let patterns: Vec<BMSPatterns> = serde_json::from_str(&body)?;
        Ok(patterns)
    }
}

impl TableAccessor for BMSSearchAccessor {
    fn name(&self) -> &str {
        "BMS Search"
    }

    fn read(&self) -> Option<TableData> {
        self.read_impl()
    }

    fn write(&self, td: &mut TableData) {
        TableDataAccessor::new(&self.tabledir).write(td);
    }
}

/// BMS Search API element
#[derive(Clone, Debug, Default, Deserialize)]
pub struct BMSSearchElement {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub genre: Option<String>,
    #[serde(default)]
    pub artist: Option<String>,
    #[serde(default)]
    pub downloads: Option<Vec<Downloads>>,
}

/// Download URL info
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Downloads {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

/// BMS patterns API response
#[derive(Clone, Debug, Default, Deserialize)]
pub struct BMSPatterns {
    #[serde(default)]
    pub file: Option<BMSPatternsFile>,
}

/// BMS patterns file with hash
#[derive(Clone, Debug, Default, Deserialize)]
pub struct BMSPatternsFile {
    #[serde(default, rename = "hashMd5")]
    pub hash_md5: Option<String>,
}
