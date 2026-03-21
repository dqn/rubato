use serde::Deserialize;

/// Root structure for database GM test fixtures.
#[derive(Debug, Deserialize)]
pub struct DatabaseFixture {
    pub test_cases: Vec<SongDataFixture>,
}

/// A single SongData test case exported from Java.
#[derive(Debug, Deserialize)]
pub struct SongDataFixture {
    pub filename: String,
    pub md5: String,
    pub sha256: String,
    pub title: String,
    pub subtitle: String,
    pub genre: String,
    pub artist: String,
    pub subartist: String,
    pub tag: String,
    pub path: String,
    pub folder: String,
    pub banner: String,
    pub stagefile: String,
    pub backbmp: String,
    pub preview: String,
    pub parent: String,
    pub level: i32,
    pub mode: i32,
    pub difficulty: i32,
    pub judge: i32,
    pub minbpm: i32,
    pub maxbpm: i32,
    pub length: i32,
    pub notes: i32,
    pub feature: i32,
    pub content: i32,
    pub date: i64,
    pub favorite: i32,
    pub adddate: i64,
    pub charthash: String,
}
