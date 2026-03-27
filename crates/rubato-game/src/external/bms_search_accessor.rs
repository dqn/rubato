use std::io::Write;
use std::time::Duration;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::external::{SongData, TableAccessor, TableData, TableDataAccessor, TableFolder};

/// A `Write` adapter that caps buffered data at a fixed size.
/// Returns `WriteZero` when the accumulated bytes would exceed the limit,
/// causing `Response::copy_to()` to abort the transfer early.
struct LimitedWriter {
    buf: Vec<u8>,
    limit: usize,
}

impl Write for LimitedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        if self.buf.len() + data.len() > self.limit {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "response exceeded size limit",
            ));
        }
        self.buf.extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Read response body with streaming size enforcement.
///
/// Unlike `response.bytes()`, this rejects oversized responses during streaming,
/// preventing memory exhaustion from chunked responses that omit Content-Length.
fn read_response_bytes_limited(
    mut response: reqwest::blocking::Response,
    max_bytes: u64,
) -> Result<Vec<u8>> {
    if let Some(content_length) = response.content_length()
        && content_length > max_bytes
    {
        bail!("Response too large: {} bytes", content_length);
    }

    let mut writer = LimitedWriter {
        buf: Vec::new(),
        limit: max_bytes as usize,
    };

    match response.copy_to(&mut writer) {
        Ok(_) => Ok(writer.buf),
        Err(_) if writer.buf.len() >= writer.limit => {
            bail!("Response too large (>{} bytes)", max_bytes)
        }
        Err(e) => bail!("Failed to read response: {}", e),
    }
}

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
                    song.metadata.title = element.title.clone().unwrap_or_default();
                    song.metadata
                        .set_artist(element.artist.clone().unwrap_or_default());
                    song.metadata.genre = element.genre.clone().unwrap_or_default();
                    if let Some(ref downloads) = element.downloads
                        && !downloads.is_empty()
                        && let Some(ref url) = downloads[0].url
                    {
                        song.url = Some(url.clone());
                    }

                    // MD5 fetch
                    if let Some(ref id) = element.id {
                        match Self::fetch_patterns(id) {
                            Ok(patterns) => {
                                if !patterns.is_empty()
                                    && let Some(ref file) = patterns[0].file
                                    && let Some(ref hash_md5) = file.hash_md5
                                {
                                    song.file.md5 = hash_md5.clone();
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

    fn http_client() -> anyhow::Result<reqwest::blocking::Client> {
        Ok(reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?)
    }

    fn fetch_elements() -> anyhow::Result<Vec<BMSSearchElement>> {
        const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024; // 16 MB

        let response = Self::http_client()?.get(API_STRING).send()?;
        let bytes = read_response_bytes_limited(response, MAX_RESPONSE_BYTES)?;
        let elements: Vec<BMSSearchElement> = serde_json::from_slice(&bytes)?;
        Ok(elements)
    }

    fn fetch_patterns(id: &str) -> anyhow::Result<Vec<BMSPatterns>> {
        const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024; // 16 MB

        let url = format!("https://api.bmssearch.net/v1/bmses/{}/patterns?limit=1", id);
        let response = Self::http_client()?.get(&url).send()?;
        let bytes = read_response_bytes_limited(response, MAX_RESPONSE_BYTES)?;
        let patterns: Vec<BMSPatterns> = serde_json::from_slice(&bytes)?;
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
