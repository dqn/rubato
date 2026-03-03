// Translated from TableEditorView.java

use std::path::{Path, PathBuf};

use regex::Regex;

use beatoraja_core::stubs::SongData;
use beatoraja_core::table_data::TableData;
use beatoraja_types::song_database_accessor::SongDatabaseAccessor;
use egui;

use crate::course_editor_view::CourseEditorView;
use crate::folder_editor_view::FolderEditorView;

/// Which sub-editor tab is active in the table editor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EditorTab {
    Course,
    Folder,
}

/// TableEditorView - table editor with course/folder sub-controllers
///
/// JavaFX UI widgets are translated to data structs.
/// Rendering via egui `render()` method.
pub struct TableEditorView {
    filepath: Option<PathBuf>,

    // JavaFX @FXML fields → egui widget state
    table_name: String,

    course_controller: CourseEditorView,
    folder_controller: FolderEditorView,

    /// Active sub-editor tab (Course or Folder).
    selected_tab: EditorTab,
}

impl TableEditorView {
    /// Hexadecimal pattern for md5/sha256 validation
    /// Java: private static final Pattern hexadecimalPattern = Pattern.compile("[0-9a-fA-F]*");
    fn hexadecimal_pattern() -> Regex {
        Regex::new(r"^[0-9a-fA-F]*$").unwrap()
    }

    /// Constructor
    pub fn new() -> Self {
        Self {
            filepath: None,
            table_name: String::new(),
            course_controller: CourseEditorView::new(),
            folder_controller: FolderEditorView::new(),
            selected_tab: EditorTab::Course,
        }
    }

    /// initialize - corresponds to Initializable.initialize(URL, ResourceBundle)
    pub fn initialize(&mut self) {
        // Empty in Java
    }

    /// init - sets the song database accessor on sub-controllers
    ///
    /// Takes two separate accessors because each sub-controller owns its accessor.
    /// In Java, a single reference was shared; in Rust, each controller gets its own Box.
    pub fn init(
        &mut self,
        course_songdb: Box<dyn SongDatabaseAccessor>,
        folder_songdb: Box<dyn SongDatabaseAccessor>,
    ) {
        self.course_controller
            .set_song_database_accessor(course_songdb);
        self.folder_controller.init(folder_songdb);
    }

    /// update - loads table data from file path
    pub fn update(&mut self, p: &Path) {
        let td = match TableData::read_from_path(p) {
            Some(td) => td,
            None => TableData {
                name: "New Table".to_string(),
                ..Default::default()
            },
        };

        self.course_controller.set_course_data(td.course.clone());
        self.folder_controller.set_table_folder(td.folder.clone());
        self.table_name = td.name.clone();
        self.filepath = Some(p.to_path_buf());
    }

    /// commit - saves table data to file
    pub fn commit(&mut self) {
        let td = TableData {
            name: self.table_name.clone(),
            course: self.course_controller.get_course_data(),
            folder: self.folder_controller.get_table_folder(),
            ..Default::default()
        };

        if let Some(ref filepath) = self.filepath
            && let Err(e) = TableData::write_to_path(filepath, &td)
        {
            log::warn!(
                "Failed to write table data to {}: {:#}",
                filepath.display(),
                e
            );
        }
    }

    /// isMd5OrSha256Hash - checks if text is a valid md5 or sha256 hash
    /// Java: public static boolean isMd5OrSha256Hash(String text)
    pub fn is_md5_or_sha256_hash(text: &str) -> bool {
        (text.len() == 32 || text.len() == 64) && Self::hexadecimal_pattern().is_match(text)
    }

    /// dialogAddCopiableRow - helper to add a copiable row to a grid dialog
    /// In Java, this creates a Label and read-only TextField in a GridPane.
    /// In Rust/egui, this is stubbed.
    fn dialog_add_copiable_row(
        _grid_data: &mut Vec<(String, String)>,
        _row: usize,
        label_text: &str,
        data_text: &str,
    ) {
        // Label label = new Label(labelText + ": ");
        // TextField textField = new TextField(dataText);
        // textField.setEditable(false);
        // gridPane.add(label, 0, row, 1, 1);
        // gridPane.add(textField, 1, row, 2, 1);
        _grid_data.push((format!("{}: ", label_text), data_text.to_string()));
    }

    /// displayChartDetailsDialog - displays a dialog with chart details
    /// Java: protected static void displayChartDetailsDialog(SongDatabaseAccessor songdb, SongData song, String... extraData)
    pub fn display_chart_details_dialog(
        _songdb: Option<&dyn SongDatabaseAccessor>,
        song: &SongData,
        extra_data: &[&str],
    ) {
        let mut grid_data: Vec<(String, String)> = Vec::new();

        // dialogAddCopiableRow(gridPane, 0, "Title", song.getFullTitle());
        Self::dialog_add_copiable_row(&mut grid_data, 0, "Title", &song.title);
        Self::dialog_add_copiable_row(&mut grid_data, 1, "Artist", &song.artist);
        Self::dialog_add_copiable_row(&mut grid_data, 2, "Genre", &song.genre);

        Self::dialog_add_copiable_row(&mut grid_data, 3, "MD5 Hash", &song.md5);
        Self::dialog_add_copiable_row(&mut grid_data, 4, "SHA256 Hash", &song.sha256);

        // if (song.getPath() == null && songdb != null) {
        //     // Try to find actual song in songdb
        //     SongData[] foundSongs = songdb.getSongDatas(new String[]{song.getSha256()});
        //     ...
        // }
        // Note: SongData stub does not have path, difficulty, judge, mode, level,
        // notes, minbpm, maxbpm, length fields. These would be populated once
        // the full SongData type is available.

        // Song detail display (difficulty, judge, BPM, etc.) is deferred
        // since the SongData stub doesn't have these fields.
        // Java code:
        //   String levelString = "UNKNOWN";
        //   switch(song.getDifficulty()) { ... }
        //   String judgeString;
        //   int judgeRank = song.getJudge();
        //   ...
        //   String bpmString;
        //   if (song.getMinbpm() == song.getMaxbpm()) { ... }
        //   String timeString = String.format("%d:%02d", song.getLength()/60000, (song.getLength()/1000)%60);
        //   Label detailsLabel = new Label(String.format("%dkeys / %s %d / %d notes / %s / %s / %s", ...));
        //   Button openFolderButton = new Button("Open Folder");
        //   openFolderButton.setOnAction((actionEvent) -> { Desktop.getDesktop().open(...); });

        // Extra data labels
        for extra in extra_data {
            grid_data.push(("".to_string(), extra.to_string()));
        }

        // Dialog dialog = new Dialog();
        // dialog.setTitle("Chart Details");
        // dialog.getDialogPane().setMinWidth(500);
        // dialog.getDialogPane().setMaxWidth(500);
        // dialog.getDialogPane().setContent(gridPane);
        // dialog.getDialogPane().getButtonTypes().add(new ButtonType("OK", ButtonData.CANCEL_CLOSE));
        // dialog.show();
        // Data prepared for egui::Window rendering via LauncherUi::show_chart_details()
        // When called outside of LauncherUi context, log as fallback
        for (label, value) in &grid_data {
            log::info!("{}{}", label, value);
        }
    }

    /// get_difficulty_string - converts difficulty int to display string
    /// Extracted from displayChartDetailsDialog for future use
    pub fn get_difficulty_string(difficulty: i32) -> &'static str {
        match difficulty {
            1 => "BEGINNER",
            2 => "NORMAL",
            3 => "HYPER",
            4 => "ANOTHER",
            5 => "INSANE",
            _ => "UNKNOWN",
        }
    }

    /// get_judge_string - converts judge rank to display string
    /// Extracted from displayChartDetailsDialog for future use
    pub fn get_judge_string(judge_rank: i32) -> &'static str {
        if judge_rank <= 25 {
            "VERY HARD"
        } else if judge_rank <= 50 {
            "HARD"
        } else if judge_rank <= 75 {
            "NORMAL"
        } else if judge_rank <= 100 {
            "EASY"
        } else {
            "VERY EASY"
        }
    }

    /// get_bpm_string - formats BPM display string
    /// Extracted from displayChartDetailsDialog for future use
    pub fn get_bpm_string(min_bpm: i32, max_bpm: i32) -> String {
        if min_bpm == max_bpm {
            format!("{}bpm", max_bpm)
        } else {
            format!("{}-{}bpm", min_bpm, max_bpm)
        }
    }

    /// get_time_string - formats length in milliseconds to mm:ss
    /// Extracted from displayChartDetailsDialog for future use
    pub fn get_time_string(length_ms: i32) -> String {
        format!("{}:{:02}", length_ms / 60000, (length_ms / 1000) % 60)
    }

    /// Render the table editor UI.
    ///
    /// Shows table name, save button, and tabbed sub-editors for courses and folders.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Table Editor");

        // File path display
        if let Some(ref path) = self.filepath {
            ui.label(format!("File: {}", path.display()));
        } else {
            ui.label("File: (none)");
        }

        ui.separator();

        // Table name
        ui.horizontal(|ui| {
            ui.label("Table Name:");
            ui.text_edit_singleline(&mut self.table_name);
        });

        // Save button
        if ui.button("Save").clicked() {
            self.commit();
        }

        ui.separator();

        // Sub-editor tabs
        ui.horizontal(|ui| {
            if ui
                .selectable_label(self.selected_tab == EditorTab::Course, "Courses")
                .clicked()
            {
                self.selected_tab = EditorTab::Course;
            }
            if ui
                .selectable_label(self.selected_tab == EditorTab::Folder, "Folders")
                .clicked()
            {
                self.selected_tab = EditorTab::Folder;
            }
        });

        ui.separator();

        match self.selected_tab {
            EditorTab::Course => self.course_controller.render(ui),
            EditorTab::Folder => self.folder_controller.render(ui),
        }
    }
}

impl Default for TableEditorView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    use beatoraja_core::course_data::{CourseData, CourseDataConstraint, TrophyData};
    use beatoraja_core::table_data::{TableData, TableFolder};
    use beatoraja_types::folder_data::FolderData;
    use beatoraja_types::song_data::SongData as TypesSongData;

    /// Mock SongDatabaseAccessor for testing
    struct MockSongDb;

    impl SongDatabaseAccessor for MockSongDb {
        fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn get_song_datas_by_sql(
            &self,
            _sql: &str,
            _score: &str,
            _scorelog: &str,
            _info: Option<&str>,
        ) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn set_song_datas(&self, _songs: &[TypesSongData]) {}
        fn get_song_datas_by_text(&self, _text: &str) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    fn make_song(title: &str, md5: &str, sha256: &str) -> SongData {
        let mut sd = SongData::new();
        sd.title = title.to_string();
        sd.md5 = md5.to_string();
        sd.sha256 = sha256.to_string();
        sd
    }

    // ---- Construction ----

    #[test]
    fn test_new_defaults() {
        let view = TableEditorView::new();
        assert!(view.filepath.is_none());
        assert!(view.table_name.is_empty());
    }

    #[test]
    fn test_default_trait() {
        let view = TableEditorView::default();
        assert!(view.filepath.is_none());
    }

    // ---- isMd5OrSha256Hash ----

    #[test]
    fn test_is_md5_hash() {
        // 32 hex chars
        assert!(TableEditorView::is_md5_or_sha256_hash(
            "abcdef1234567890abcdef1234567890"
        ));
    }

    #[test]
    fn test_is_sha256_hash() {
        // 64 hex chars
        assert!(TableEditorView::is_md5_or_sha256_hash(
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        ));
    }

    #[test]
    fn test_is_not_hash_wrong_length() {
        assert!(!TableEditorView::is_md5_or_sha256_hash("abcdef"));
        assert!(!TableEditorView::is_md5_or_sha256_hash(
            "abcdef1234567890abcdef123456789" // 31 chars
        ));
        assert!(!TableEditorView::is_md5_or_sha256_hash(
            "abcdef1234567890abcdef12345678901" // 33 chars
        ));
    }

    #[test]
    fn test_is_not_hash_non_hex() {
        assert!(!TableEditorView::is_md5_or_sha256_hash(
            "ghijkl1234567890ghijkl1234567890" // non-hex chars
        ));
    }

    #[test]
    fn test_is_hash_upper_case() {
        assert!(TableEditorView::is_md5_or_sha256_hash(
            "ABCDEF1234567890ABCDEF1234567890"
        ));
    }

    #[test]
    fn test_is_hash_mixed_case() {
        assert!(TableEditorView::is_md5_or_sha256_hash(
            "AbCdEf1234567890aBcDeF1234567890"
        ));
    }

    #[test]
    fn test_is_hash_empty_string() {
        assert!(!TableEditorView::is_md5_or_sha256_hash(""));
    }

    // ---- getDifficultyString ----

    #[test]
    fn test_get_difficulty_string() {
        assert_eq!(TableEditorView::get_difficulty_string(0), "UNKNOWN");
        assert_eq!(TableEditorView::get_difficulty_string(1), "BEGINNER");
        assert_eq!(TableEditorView::get_difficulty_string(2), "NORMAL");
        assert_eq!(TableEditorView::get_difficulty_string(3), "HYPER");
        assert_eq!(TableEditorView::get_difficulty_string(4), "ANOTHER");
        assert_eq!(TableEditorView::get_difficulty_string(5), "INSANE");
        assert_eq!(TableEditorView::get_difficulty_string(6), "UNKNOWN");
        assert_eq!(TableEditorView::get_difficulty_string(-1), "UNKNOWN");
    }

    // ---- getJudgeString ----

    #[test]
    fn test_get_judge_string() {
        assert_eq!(TableEditorView::get_judge_string(0), "VERY HARD");
        assert_eq!(TableEditorView::get_judge_string(25), "VERY HARD");
        assert_eq!(TableEditorView::get_judge_string(26), "HARD");
        assert_eq!(TableEditorView::get_judge_string(50), "HARD");
        assert_eq!(TableEditorView::get_judge_string(51), "NORMAL");
        assert_eq!(TableEditorView::get_judge_string(75), "NORMAL");
        assert_eq!(TableEditorView::get_judge_string(76), "EASY");
        assert_eq!(TableEditorView::get_judge_string(100), "EASY");
        assert_eq!(TableEditorView::get_judge_string(101), "VERY EASY");
        assert_eq!(TableEditorView::get_judge_string(200), "VERY EASY");
    }

    // ---- getBpmString ----

    #[test]
    fn test_get_bpm_string_same() {
        assert_eq!(TableEditorView::get_bpm_string(150, 150), "150bpm");
    }

    #[test]
    fn test_get_bpm_string_range() {
        assert_eq!(TableEditorView::get_bpm_string(120, 180), "120-180bpm");
    }

    // ---- getTimeString ----

    #[test]
    fn test_get_time_string() {
        assert_eq!(TableEditorView::get_time_string(0), "0:00");
        assert_eq!(TableEditorView::get_time_string(60000), "1:00");
        assert_eq!(TableEditorView::get_time_string(90000), "1:30");
        assert_eq!(TableEditorView::get_time_string(125000), "2:05");
        assert_eq!(TableEditorView::get_time_string(3600000), "60:00");
    }

    // ---- dialogAddCopiableRow ----

    #[test]
    fn test_dialog_add_copiable_row() {
        let mut grid = Vec::new();
        TableEditorView::dialog_add_copiable_row(&mut grid, 0, "Title", "My Song");
        assert_eq!(grid.len(), 1);
        assert_eq!(grid[0].0, "Title: ");
        assert_eq!(grid[0].1, "My Song");
    }

    // ---- displayChartDetailsDialog ----

    #[test]
    fn test_display_chart_details_dialog_does_not_panic() {
        let song = make_song("Test Song", "md5hash", "sha256hash");
        // Should not panic even with no songdb
        TableEditorView::display_chart_details_dialog(None, &song, &[]);
    }

    #[test]
    fn test_display_chart_details_dialog_with_extra_data() {
        let song = make_song("Test Song", "md5hash", "sha256hash");
        TableEditorView::display_chart_details_dialog(
            None,
            &song,
            &["Extra info 1", "Extra info 2"],
        );
        // Should not panic
    }

    // ---- init ----

    #[test]
    fn test_init_sets_songdb() {
        let mut view = TableEditorView::new();
        view.init(Box::new(MockSongDb), Box::new(MockSongDb));
        // No panic; sub-controllers have their songdb set
    }

    // ---- update / commit with temp file ----

    #[test]
    fn test_update_from_json_file() {
        let mut view = TableEditorView::new();

        // Create a valid TableData JSON with minimal data
        let song = make_song("Song A", "abcd1234abcd1234abcd1234abcd1234", "sha");
        let td = TableData {
            name: "My Table".to_string(),
            folder: vec![TableFolder {
                name: Some("Level 1".to_string()),
                songs: vec![song],
            }],
            course: vec![CourseData {
                name: Some("My Course".to_string()),
                hash: vec![make_song(
                    "Course Song",
                    "1234abcd1234abcd1234abcd1234abcd",
                    "sha2",
                )],
                constraint: vec![CourseDataConstraint::Class],
                trophy: vec![TrophyData::new("bronzemedal".to_string(), 5.0, 60.0)],
                release: false,
            }],
            ..Default::default()
        };
        let json = serde_json::to_string_pretty(&td).unwrap();

        let mut tmpfile = NamedTempFile::with_suffix(".json").unwrap();
        tmpfile.write_all(json.as_bytes()).unwrap();
        tmpfile.flush().unwrap();

        view.update(tmpfile.path());
        assert_eq!(view.table_name, "My Table");
        assert_eq!(view.filepath, Some(tmpfile.path().to_path_buf()));
    }

    #[test]
    fn test_update_nonexistent_file() {
        let mut view = TableEditorView::new();
        let path = Path::new("/tmp/nonexistent_table_42b.json");

        view.update(path);
        assert_eq!(view.table_name, "New Table");
        assert_eq!(view.filepath, Some(path.to_path_buf()));
    }

    #[test]
    fn test_commit_writes_json_file() {
        let mut view = TableEditorView::new();

        // Set up view with data
        view.table_name = "Saved Table".to_string();
        view.course_controller.set_course_data(vec![CourseData {
            name: Some("C1".to_string()),
            hash: vec![make_song("S1", "abcd1234abcd1234abcd1234abcd1234", "sha")],
            ..Default::default()
        }]);
        view.folder_controller.set_table_folder(vec![TableFolder {
            name: Some("F1".to_string()),
            songs: vec![make_song("S2", "1234abcd1234abcd1234abcd1234abcd", "sha2")],
        }]);

        let tmpfile = NamedTempFile::with_suffix(".json").unwrap();
        view.filepath = Some(tmpfile.path().to_path_buf());

        view.commit();

        // Read back and verify
        let contents = std::fs::read_to_string(tmpfile.path()).unwrap();
        let td: TableData = serde_json::from_str(&contents).unwrap();
        assert_eq!(td.name, "Saved Table");
    }

    #[test]
    fn test_commit_no_filepath() {
        let mut view = TableEditorView::new();
        view.table_name = "No Path".to_string();
        view.filepath = None;

        // Should not panic
        view.commit();
    }

    // ---- round trip: update → modify → commit → re-read ----

    #[test]
    fn test_round_trip_update_commit() {
        // Create initial file
        let song = make_song("Song A", "abcd1234abcd1234abcd1234abcd1234", "sha");
        let td = TableData {
            name: "Original".to_string(),
            folder: vec![TableFolder {
                name: Some("F1".to_string()),
                songs: vec![song],
            }],
            ..Default::default()
        };
        let json = serde_json::to_string_pretty(&td).unwrap();

        let mut tmpfile = NamedTempFile::with_suffix(".json").unwrap();
        tmpfile.write_all(json.as_bytes()).unwrap();
        tmpfile.flush().unwrap();

        // Load
        let mut view = TableEditorView::new();
        view.update(tmpfile.path());
        assert_eq!(view.table_name, "Original");

        // Modify
        view.table_name = "Modified".to_string();

        // Save
        view.commit();

        // Re-read
        let mut view2 = TableEditorView::new();
        view2.update(tmpfile.path());
        assert_eq!(view2.table_name, "Modified");
    }
}
