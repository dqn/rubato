use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::select::stubs::*;

/// Context for CommandBar SQL queries.
/// Provides the database paths needed for SQL-based song queries.
pub struct CommandBarContext<'a> {
    pub score_db_path: &'a str,
    pub scorelog_db_path: &'a str,
    pub info_db_path: Option<&'a str>,
}

/// SQL command-based directory bar
/// Translates: bms.player.beatoraja.select.bar.CommandBar
#[derive(Clone)]
pub struct CommandBar {
    pub directory: DirectoryBarData,
    /// Bar title
    pub title: String,
    /// SQL query
    pub sql: String,
}

impl CommandBar {
    pub fn new(title: String, sql: String) -> Self {
        Self::new_with_visibility(title, sql, false)
    }

    pub fn new_with_visibility(title: String, sql: String, show_invisible_chart: bool) -> Self {
        Self {
            directory: DirectoryBarData::new(show_invisible_chart),
            title,
            sql,
        }
    }

    pub fn get_title(&self) -> String {
        self.title.clone()
    }

    /// Get children bars by executing SQL query against the song database.
    ///
    /// Translates: Java CommandBar.getChildren()
    pub fn get_children(&self, db: &dyn SongDatabaseAccessor, ctx: &CommandBarContext) -> Vec<Bar> {
        let songs = db.get_song_datas_by_sql(
            &self.sql,
            ctx.score_db_path,
            ctx.scorelog_db_path,
            ctx.info_db_path,
        );
        SongBar::to_song_bar_array(&songs)
    }

    pub fn update_folder_status(&mut self, db: &dyn SongDatabaseAccessor, ctx: &CommandBarContext) {
        let songs = db.get_song_datas_by_sql(
            &self.sql,
            ctx.score_db_path,
            ctx.scorelog_db_path,
            ctx.info_db_path,
        );
        self.directory
            .update_folder_status_with_songs(&songs, None, |_| None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::folder_data::FolderData;

    struct MockSongDb {
        sql_songs: Vec<SongData>,
    }

    impl MockSongDb {
        fn new(sql_songs: Vec<SongData>) -> Self {
            Self { sql_songs }
        }
    }

    impl SongDatabaseAccessor for MockSongDb {
        fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
            Vec::new()
        }
        fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
            Vec::new()
        }
        fn get_song_datas_by_sql(
            &self,
            _sql: &str,
            _score: &str,
            _scorelog: &str,
            _info: Option<&str>,
        ) -> Vec<SongData> {
            self.sql_songs.clone()
        }
        fn set_song_datas(&self, _songs: &[SongData]) {}
        fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
            Vec::new()
        }
        fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    #[test]
    fn command_bar_get_children_returns_sql_results() {
        let mut song = SongData::default();
        song.set_title("SQL Result Song".to_string());
        song.set_sha256("sql_hash".to_string());

        let db = MockSongDb::new(vec![song]);
        let ctx = CommandBarContext {
            score_db_path: "player/score.db",
            scorelog_db_path: "player/scorelog.db",
            info_db_path: Some("songinfo.db"),
        };

        let bar = CommandBar::new("Recent".to_string(), "SELECT * FROM song".to_string());
        let children = bar.get_children(&db, &ctx);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
    }

    #[test]
    fn command_bar_get_children_returns_empty_for_no_results() {
        let db = MockSongDb::new(vec![]);
        let ctx = CommandBarContext {
            score_db_path: "player/score.db",
            scorelog_db_path: "player/scorelog.db",
            info_db_path: None,
        };

        let bar = CommandBar::new("Empty".to_string(), "SELECT 1".to_string());
        let children = bar.get_children(&db, &ctx);

        assert!(children.is_empty());
    }
}
