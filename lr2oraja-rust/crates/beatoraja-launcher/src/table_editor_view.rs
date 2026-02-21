// Translated from TableEditorView.java

use std::path::{Path, PathBuf};

use regex::Regex;

use beatoraja_core::main_controller::SongDatabaseAccessor;
use beatoraja_core::stubs::SongData;
use beatoraja_core::table_data::TableData;

use crate::course_editor_view::CourseEditorView;
use crate::folder_editor_view::FolderEditorView;

/// TableEditorView - table editor with course/folder sub-controllers
///
/// JavaFX UI widgets are translated to data structs.
/// All rendering/UI operations use todo!("egui integration").
#[allow(dead_code)]
pub struct TableEditorView {
    filepath: Option<PathBuf>,

    // JavaFX @FXML fields → egui widget state
    table_name: String,

    course_controller: CourseEditorView,
    folder_controller: FolderEditorView,
}

#[allow(dead_code)]
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
        }
    }

    /// initialize - corresponds to Initializable.initialize(URL, ResourceBundle)
    pub fn initialize(&mut self) {
        // Empty in Java
    }

    /// init - sets the song database accessor on sub-controllers
    pub fn init(&mut self, songdb: SongDatabaseAccessor) {
        self.course_controller.set_song_database_accessor(songdb);
        // FolderEditorView.init expects owned SongDatabaseAccessor
        // but SongDatabaseAccessor is a unit struct, so we create another
        let songdb2 = SongDatabaseAccessor;
        self.folder_controller.init(songdb2);
    }

    /// update - loads table data from file path
    pub fn update(&mut self, p: &Path) {
        let td = match TableData::read_from_path(p) {
            Some(td) => td,
            None => {
                let mut td = TableData::default();
                td.name = "New Table".to_string();
                td
            }
        };

        self.course_controller
            .set_course_data(td.course.clone());
        self.folder_controller
            .set_table_folder(td.folder.clone());
        self.table_name = td.name.clone();
        self.filepath = Some(p.to_path_buf());
    }

    /// commit - saves table data to file
    pub fn commit(&mut self) {
        let mut td = TableData::default();
        td.name = self.table_name.clone();
        td.course = self.course_controller.get_course_data();
        td.folder = self.folder_controller.get_table_folder();

        if let Some(ref filepath) = self.filepath {
            TableData::write_to_path(filepath, &td);
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
        _songdb: Option<&SongDatabaseAccessor>,
        song: &SongData,
        extra_data: &[&str],
    ) {
        let mut grid_data: Vec<(String, String)> = Vec::new();

        // dialogAddCopiableRow(gridPane, 0, "Title", song.getFullTitle());
        Self::dialog_add_copiable_row(
            &mut grid_data,
            0,
            "Title",
            song.title.as_deref().unwrap_or(""),
        );
        // dialogAddCopiableRow(gridPane, 1, "Artist", song.getFullArtist());
        Self::dialog_add_copiable_row(&mut grid_data, 1, "Artist", ""); // SongData stub has no artist field
        // dialogAddCopiableRow(gridPane, 2, "Genre", song.getGenre());
        Self::dialog_add_copiable_row(&mut grid_data, 2, "Genre", ""); // SongData stub has no genre field

        // dialogAddCopiableRow(gridPane, 3, "MD5 Hash", song.getMd5());
        Self::dialog_add_copiable_row(
            &mut grid_data,
            3,
            "MD5 Hash",
            song.md5.as_deref().unwrap_or(""),
        );
        // dialogAddCopiableRow(gridPane, 4, "SHA256 Hash", song.getSha256());
        Self::dialog_add_copiable_row(
            &mut grid_data,
            4,
            "SHA256 Hash",
            song.sha256.as_deref().unwrap_or(""),
        );

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
        let _ = grid_data; // suppress unused warning
        todo!("egui integration")
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
}

impl Default for TableEditorView {
    fn default() -> Self {
        Self::new()
    }
}
