use super::bar::Bar;
use super::directory_bar::DirectoryBarData;
use super::song_bar::SongBar;
use crate::state::select::*;

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

    pub fn title(&self) -> &str {
        &self.title
    }

    /// Get children bars by executing SQL query against the song database.
    ///
    /// Translates: Java CommandBar.getChildren()
    pub fn children(&self, db: &dyn SongDatabaseAccessor, ctx: &CommandBarContext) -> Vec<Bar> {
        let songs = db.song_datas_by_sql(
            &self.sql,
            ctx.score_db_path,
            ctx.scorelog_db_path,
            ctx.info_db_path,
        );
        SongBar::to_song_bar_array(&songs)
    }

    pub fn update_folder_status(&mut self, db: &dyn SongDatabaseAccessor, ctx: &CommandBarContext) {
        let songs = db.song_datas_by_sql(
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
    use rubato_types::test_support::TestSongDb;

    #[test]
    fn command_bar_get_children_returns_sql_results() {
        let mut song = SongData::default();
        song.metadata.title = "SQL Result Song".to_string();
        song.file.sha256 = "sql_hash".to_string();

        let db = TestSongDb::new().with_songs_by_sql(vec![song]);
        let ctx = CommandBarContext {
            score_db_path: "player/score.db",
            scorelog_db_path: "player/scorelog.db",
            info_db_path: Some("songinfo.db"),
        };

        let bar = CommandBar::new("Recent".to_string(), "SELECT * FROM song".to_string());
        let children = bar.children(&db, &ctx);

        assert_eq!(children.len(), 1);
        assert!(children[0].as_song_bar().is_some());
    }

    #[test]
    fn command_bar_get_children_returns_empty_for_no_results() {
        let db = TestSongDb::new().with_songs_by_sql(vec![]);
        let ctx = CommandBarContext {
            score_db_path: "player/score.db",
            scorelog_db_path: "player/scorelog.db",
            info_db_path: None,
        };

        let bar = CommandBar::new("Empty".to_string(), "SELECT 1".to_string());
        let children = bar.children(&db, &ctx);

        assert!(children.is_empty());
    }
}
