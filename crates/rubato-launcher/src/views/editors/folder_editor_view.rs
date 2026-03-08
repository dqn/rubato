// Translated from FolderEditorView.java

use std::path::PathBuf;

use egui;
use rubato_core::course_data::CourseData;
use rubato_core::stubs::SongData;
use rubato_core::table_data::TableFolder;
use rubato_types::song_database_accessor::SongDatabaseAccessor;

use crate::views::editors::table_editor_view::TableEditorView;

/// SongDataView stub — corresponds to the SongDataView FXML sub-controller
#[derive(Clone, Debug, Default)]
pub struct SongDataView {
    visible_columns: Vec<String>,
}

impl SongDataView {
    pub fn set_visible(&mut self, columns: &[&str]) {
        self.visible_columns = columns.iter().map(|s| s.to_string()).collect();
        // In egui, columns are defined inline during render() -- no pre-init needed.
    }

    /// Returns the list of visible column names (for testing/inspection)
    pub fn visible_columns(&self) -> &[String] {
        &self.visible_columns
    }
}

/// FolderEditorView - folder editor with search, song data tables, folder list
///
/// JavaFX UI widgets are translated to data structs.
/// Rendering will be implemented via egui when the folder editor tab is wired.
pub struct FolderEditorView {
    // JavaFX @FXML fields → egui widget state
    search: String,
    search_songs: Vec<SongData>,
    search_songs_controller: SongDataView,
    search_songs_selected_items: Vec<SongData>,
    _search_songs_selected_index: Option<usize>,

    pub folders: Vec<TableFolder>,
    folders_selected_index: Option<usize>,
    folder_pane_visible: bool,
    folder_name: String,
    folder_songs: Vec<SongData>,
    folder_songs_controller: SongDataView,
    folder_songs_selected_index: Option<usize>,

    #[allow(dead_code)] // Used in tests; Java-ported field for future egui wiring
    filepath: Option<PathBuf>,

    selected_folder: Option<usize>, // index into folders

    songdb: Option<Box<dyn SongDatabaseAccessor>>,

    #[allow(dead_code)] // Used in tests; Java-ported field for future egui wiring
    courses: Vec<CourseData>,
}

impl FolderEditorView {
    /// Constructor
    pub fn new() -> Self {
        Self {
            search: String::new(),
            search_songs: Vec::new(),
            search_songs_controller: SongDataView::default(),
            search_songs_selected_items: Vec::new(),
            _search_songs_selected_index: None,

            folders: Vec::new(),
            folders_selected_index: None,
            folder_pane_visible: false,
            folder_name: String::new(),
            folder_songs: Vec::new(),
            folder_songs_controller: SongDataView::default(),
            folder_songs_selected_index: None,

            filepath: None,

            selected_folder: None,

            songdb: None,

            courses: Vec::new(),
        }
    }

    /// initialize - corresponds to Initializable.initialize(URL, ResourceBundle)
    pub fn initialize(&mut self) {
        // folders.getSelectionModel().selectedIndexProperty().addListener(...)
        // → handled by UI framework event system in egui

        // folders.setCellFactory(...)
        // → cell rendering handled by egui

        // folderSongsController.setVisible("fullTitle", "sha256");
        self.folder_songs_controller
            .set_visible(&["fullTitle", "sha256"]);
        // searchSongsController.setVisible("fullTitle", "fullArtist", "mode", "level", "notes", "sha256");
        self.search_songs_controller.set_visible(&[
            "fullTitle",
            "fullArtist",
            "mode",
            "level",
            "notes",
            "sha256",
        ]);

        // searchSongs.getSelectionModel().setSelectionMode(SelectionMode.MULTIPLE);
        // → multiple selection handled by egui

        // searchSongs.setOnMouseClicked(...)
        // folderSongs.setOnMouseClicked(...)
        // → double-click handlers handled by egui event system

        self.update_folder(None);
    }

    /// init - sets the song database accessor
    pub fn init(&mut self, songdb: Box<dyn SongDatabaseAccessor>) {
        self.songdb = Some(songdb);
    }

    /// searchSongs - searches for songs by hash or text
    pub fn search_songs(&mut self) {
        let Some(songdb) = self.songdb.as_ref() else {
            return;
        };
        if TableEditorView::is_md5_or_sha256_hash(&self.search) {
            self.search_songs = songdb.song_datas_by_hashes(std::slice::from_ref(&self.search));
        } else if self.search.len() > 1 {
            self.search_songs = songdb.song_datas_by_text(&self.search);
        }
    }

    /// updateTableFolder - commits current folder and updates to selected folder
    pub fn update_table_folder(&mut self) {
        self.commit_folder();
        let selected_idx = self.folders_selected_index;
        if let Some(idx) = selected_idx {
            self.update_folder(Some(idx));
        } else {
            self.update_folder(None);
        }
    }

    /// commitFolder - saves current folder state from UI
    fn commit_folder(&mut self) {
        let folder_idx = match self.selected_folder {
            Some(idx) => idx,
            None => return,
        };

        if folder_idx >= self.folders.len() {
            return;
        }

        // selectedFolder.setName(folderName.getText());
        self.folders[folder_idx].name = Some(self.folder_name.clone());
        // selectedFolder.setSong(folderSongs.getItems().toArray(...));
        self.folders[folder_idx].songs = self.folder_songs.clone();
    }

    /// updateFolder - updates UI to show the given folder
    fn update_folder(&mut self, folder_idx: Option<usize>) {
        self.selected_folder = folder_idx;
        match folder_idx {
            None => {
                // folderPane.setVisible(false);
                self.folder_pane_visible = false;
            }
            Some(idx) => {
                if idx >= self.folders.len() {
                    self.folder_pane_visible = false;
                    return;
                }
                // folderPane.setVisible(true);
                self.folder_pane_visible = true;

                // folderName.setText(selectedFolder.getName());
                self.folder_name = self.folders[idx].name.clone().unwrap_or_default();
                // folderSongs.getItems().setAll(course.getSong());
                self.folder_songs = self.folders[idx].songs.clone();
            }
        }
    }

    /// addTableFolder - adds a new empty folder
    pub fn add_table_folder(&mut self) {
        let folder = TableFolder {
            name: Some("New Folder".to_string()),
            ..Default::default()
        };
        self.folders.push(folder);
    }

    /// removeTableFolder - removes the currently selected folder
    pub fn remove_table_folder(&mut self) {
        if let Some(idx) = self.folders_selected_index
            && idx < self.folders.len()
        {
            self.folders.remove(idx);
        }
    }

    /// moveTableFolderUp - moves the selected folder up one position
    pub fn move_table_folder_up(&mut self) {
        if let Some(index) = self.folders_selected_index
            && index > 0
        {
            self.folders.swap(index, index - 1);
            self.folders_selected_index = Some(index - 1);
        }
    }

    /// moveTableFolderDown - moves the selected folder down one position
    pub fn move_table_folder_down(&mut self) {
        if let Some(index) = self.folders_selected_index
            && index < self.folders.len().saturating_sub(1)
        {
            self.folders.swap(index, index + 1);
            self.folders_selected_index = Some(index + 1);
        }
    }

    /// addSongData - adds selected search songs to the current folder
    pub fn add_song_data(&mut self) {
        // List<SongData> songs = searchSongs.getSelectionModel().getSelectedItems();
        for song in &self.search_songs_selected_items.clone() {
            self.folder_songs.push(song.clone());
        }
    }

    /// removeSongData - removes the selected song from the folder
    pub fn remove_song_data(&mut self) {
        if let Some(idx) = self.folder_songs_selected_index
            && idx < self.folder_songs.len()
        {
            self.folder_songs.remove(idx);
        }
    }

    /// moveSongDataUp - moves the selected song up one position
    pub fn move_song_data_up(&mut self) {
        if let Some(index) = self.folder_songs_selected_index
            && index > 0
        {
            self.folder_songs.swap(index, index - 1);
            self.folder_songs_selected_index = Some(index - 1);
        }
    }

    /// moveSongDataDown - moves the selected song down one position
    pub fn move_song_data_down(&mut self) {
        if let Some(index) = self.folder_songs_selected_index
            && index < self.folder_songs.len().saturating_sub(1)
        {
            self.folder_songs.swap(index, index + 1);
            self.folder_songs_selected_index = Some(index + 1);
        }
    }

    /// getTableFolder - commits and returns all folders
    pub fn table_folder(&mut self) -> Vec<TableFolder> {
        self.commit_folder();
        self.folders.clone()
    }
    /// getFoldersContainingSong - finds which folders contain a given song
    pub fn folders_containing_song(folders: &[TableFolder], song: &SongData) -> String {
        let mut sb = String::new();
        for folder in folders {
            let songs = &folder.songs;
            for ts in songs {
                let ts_md5 = ts.file.md5.as_str();
                let song_md5 = song.file.md5.as_str();
                let ts_sha256 = ts.file.sha256.as_str();
                let song_sha256 = song.file.sha256.as_str();

                if (!ts_md5.is_empty() && !song_md5.is_empty() && ts_md5 == song_md5)
                    || (!ts_sha256.is_empty()
                        && !song_sha256.is_empty()
                        && ts_sha256 == song_sha256)
                {
                    if !sb.is_empty() {
                        sb.push_str(", ");
                    }
                    sb.push_str(folder.name.as_deref().unwrap_or(""));
                    // continue (to next folder)
                    break;
                }
            }
        }
        if sb.is_empty() {
            "None".to_string()
        } else {
            sb
        }
    }

    /// displayChartDetailsDialog - shows chart details dialog for a song
    fn _display_chart_details_dialog(&self, song: &SongData) {
        let extra = format!(
            "In custom folder(s):\n{}",
            Self::folders_containing_song(&self.folders, song)
        );
        TableEditorView::display_chart_details_dialog(self.songdb.as_deref(), song, &[&extra]);
    }

    /// Render the folder editor UI.
    ///
    /// Layout: folder list on the left, folder detail pane on the right.
    /// Song search panel at the bottom.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // --- Folder list panel ---
        ui.horizontal(|ui| {
            ui.label("Folders:");
            if ui.button("Add").clicked() {
                self.add_table_folder();
            }
            if ui.button("Remove").clicked() {
                self.remove_table_folder();
                self.update_folder(None);
            }
            if ui.button("Up").clicked() {
                self.move_table_folder_up();
            }
            if ui.button("Down").clicked() {
                self.move_table_folder_down();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("folder_list_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                let mut new_selection = self.folders_selected_index;
                for (i, folder) in self.folders.iter().enumerate() {
                    let name = folder.name.as_deref().unwrap_or("(unnamed)");
                    let selected = self.folders_selected_index == Some(i);
                    if ui.selectable_label(selected, name).clicked() {
                        new_selection = Some(i);
                    }
                }
                if new_selection != self.folders_selected_index {
                    self.commit_folder();
                    self.folders_selected_index = new_selection;
                    self.update_folder(new_selection);
                }
            });

        ui.separator();

        // --- Folder detail pane ---
        if self.folder_pane_visible {
            ui.horizontal(|ui| {
                ui.label("Folder Name:");
                ui.text_edit_singleline(&mut self.folder_name);
            });

            ui.separator();

            // --- Folder songs ---
            ui.horizontal(|ui| {
                ui.label("Folder Songs:");
                if ui.button("Remove Song").clicked() {
                    self.remove_song_data();
                }
                if ui.button("Move Up").clicked() {
                    self.move_song_data_up();
                }
                if ui.button("Move Down").clicked() {
                    self.move_song_data_down();
                }
            });

            egui::ScrollArea::vertical()
                .id_salt("folder_songs_scroll")
                .max_height(100.0)
                .show(ui, |ui| {
                    for (i, song) in self.folder_songs.iter().enumerate() {
                        let selected = self.folder_songs_selected_index == Some(i);
                        let label = format!("{} [{}]", song.metadata.full_title(), &song.file.sha256);
                        if ui.selectable_label(selected, &label).clicked() {
                            self.folder_songs_selected_index = Some(i);
                        }
                    }
                });
        }

        ui.separator();

        // --- Song search ---
        ui.horizontal(|ui| {
            ui.label("Search:");
            let response = ui.text_edit_singleline(&mut self.search);
            if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || ui.button("Search").clicked()
            {
                self.search_songs();
            }
        });

        if ui.button("Add Selected to Folder").clicked() {
            self.add_song_data();
        }

        egui::ScrollArea::vertical()
            .id_salt("search_songs_folder_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                // Multi-select: toggle with click
                for (i, song) in self.search_songs.iter().enumerate() {
                    let is_selected = self
                        .search_songs_selected_items
                        .iter()
                        .any(|s| s.file.sha256 == song.file.sha256 && s.file.md5 == song.file.md5);
                    let label = format!(
                        "{} - {} [{}]",
                        song.metadata.full_title(),
                        song.metadata.artist,
                        &song.file.sha256,
                    );
                    if ui.selectable_label(is_selected, &label).clicked() {
                        if is_selected {
                            self.search_songs_selected_items.retain(|s| {
                                s.file.sha256 != song.file.sha256 || s.file.md5 != song.file.md5
                            });
                        } else {
                            self.search_songs_selected_items
                                .push(self.search_songs[i].clone());
                        }
                    }
                }
            });
    }
}

impl Default for FolderEditorView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_core::table_data::TableFolder;
    use rubato_types::folder_data::FolderData;
    use rubato_types::song_data::SongData as TypesSongData;

    /// Mock SongDatabaseAccessor for testing
    struct MockSongDb;

    impl SongDatabaseAccessor for MockSongDb {
        fn song_datas(&self, _key: &str, _value: &str) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn song_datas_by_sql(
            &self,
            _sql: &str,
            _score: &str,
            _scorelog: &str,
            _info: Option<&str>,
        ) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn set_song_datas(&self, _songs: &[TypesSongData]) {}
        fn song_datas_by_text(&self, _text: &str) -> Vec<TypesSongData> {
            Vec::new()
        }
        fn folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    fn make_song(title: &str, md5: &str, sha256: &str) -> SongData {
        let mut sd = SongData::new();
        sd.metadata.title = title.to_string();
        sd.file.md5 = md5.to_string();
        sd.file.sha256 = sha256.to_string();
        sd
    }

    fn make_folder(name: &str, songs: Vec<SongData>) -> TableFolder {
        TableFolder {
            name: Some(name.to_string()),
            songs,
        }
    }

    // ---- Construction ----

    #[test]
    fn test_new_defaults() {
        let view = FolderEditorView::new();
        assert!(view.search.is_empty());
        assert!(view.search_songs.is_empty());
        assert!(view.folders.is_empty());
        assert!(view.folders_selected_index.is_none());
        assert!(!view.folder_pane_visible);
        assert!(view.folder_name.is_empty());
        assert!(view.folder_songs.is_empty());
        assert!(view.filepath.is_none());
        assert!(view.selected_folder.is_none());
        assert!(view.songdb.is_none());
        assert!(view.courses.is_empty());
    }

    #[test]
    fn test_default_trait() {
        let view = FolderEditorView::default();
        assert!(view.folders.is_empty());
    }

    // ---- initialize() ----

    #[test]
    fn test_initialize_sets_visible_columns() {
        let mut view = FolderEditorView::new();
        view.initialize();
        assert_eq!(
            view.folder_songs_controller.visible_columns(),
            &["fullTitle", "sha256"]
        );
        assert_eq!(
            view.search_songs_controller.visible_columns(),
            &[
                "fullTitle",
                "fullArtist",
                "mode",
                "level",
                "notes",
                "sha256"
            ]
        );
    }

    #[test]
    fn test_initialize_hides_folder_pane() {
        let mut view = FolderEditorView::new();
        view.initialize();
        assert!(!view.folder_pane_visible);
        assert!(view.selected_folder.is_none());
    }

    // ---- set/get table folder ----

    #[test]
    fn test_set_and_get_table_folder() {
        let mut view = FolderEditorView::new();
        let folders = vec![
            make_folder("Folder A", vec![]),
            make_folder("Folder B", vec![]),
        ];
        view.folders = folders;
        let result = view.table_folder();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name(), "Folder A");
        assert_eq!(result[1].name(), "Folder B");
    }

    // ---- addTableFolder ----

    #[test]
    fn test_add_table_folder() {
        let mut view = FolderEditorView::new();
        assert!(view.folders.is_empty());

        view.add_table_folder();
        assert_eq!(view.folders.len(), 1);
        assert_eq!(view.folders[0].name(), "New Folder");
        assert!(view.folders[0].songs.is_empty());
    }

    // ---- removeTableFolder ----

    #[test]
    fn test_remove_table_folder() {
        let mut view = FolderEditorView::new();
        view.folders = vec![
            make_folder("A", vec![]),
            make_folder("B", vec![]),
            make_folder("C", vec![]),
        ];
        view.folders_selected_index = Some(1);

        view.remove_table_folder();
        assert_eq!(view.folders.len(), 2);
        assert_eq!(view.folders[0].name(), "A");
        assert_eq!(view.folders[1].name(), "C");
    }

    #[test]
    fn test_remove_table_folder_no_selection() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("A", vec![])];
        view.folders_selected_index = None;

        view.remove_table_folder();
        assert_eq!(view.folders.len(), 1);
    }

    // ---- moveTableFolderUp/Down ----

    #[test]
    fn test_move_table_folder_up() {
        let mut view = FolderEditorView::new();
        view.folders = vec![
            make_folder("A", vec![]),
            make_folder("B", vec![]),
            make_folder("C", vec![]),
        ];
        view.folders_selected_index = Some(1);

        view.move_table_folder_up();
        assert_eq!(view.folders[0].name(), "B");
        assert_eq!(view.folders[1].name(), "A");
        assert_eq!(view.folders_selected_index, Some(0));
    }

    #[test]
    fn test_move_table_folder_up_at_top() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("A", vec![]), make_folder("B", vec![])];
        view.folders_selected_index = Some(0);

        view.move_table_folder_up();
        assert_eq!(view.folders[0].name(), "A");
        assert_eq!(view.folders_selected_index, Some(0));
    }

    #[test]
    fn test_move_table_folder_down() {
        let mut view = FolderEditorView::new();
        view.folders = vec![
            make_folder("A", vec![]),
            make_folder("B", vec![]),
            make_folder("C", vec![]),
        ];
        view.folders_selected_index = Some(0);

        view.move_table_folder_down();
        assert_eq!(view.folders[0].name(), "B");
        assert_eq!(view.folders[1].name(), "A");
        assert_eq!(view.folders_selected_index, Some(1));
    }

    #[test]
    fn test_move_table_folder_down_at_bottom() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("A", vec![]), make_folder("B", vec![])];
        view.folders_selected_index = Some(1);

        view.move_table_folder_down();
        assert_eq!(view.folders[1].name(), "B");
        assert_eq!(view.folders_selected_index, Some(1));
    }

    // ---- addSongData / removeSongData ----

    #[test]
    fn test_add_song_data() {
        let mut view = FolderEditorView::new();
        view.search_songs_selected_items = vec![
            make_song("Song 1", "md5a", "sha1"),
            make_song("Song 2", "md5b", "sha2"),
        ];

        view.add_song_data();
        assert_eq!(view.folder_songs.len(), 2);
        assert_eq!(view.folder_songs[0].metadata.title, "Song 1");
        assert_eq!(view.folder_songs[1].metadata.title, "Song 2");
    }

    #[test]
    fn test_remove_song_data() {
        let mut view = FolderEditorView::new();
        view.folder_songs = vec![
            make_song("S1", "a", "x"),
            make_song("S2", "b", "y"),
            make_song("S3", "c", "z"),
        ];
        view.folder_songs_selected_index = Some(1);

        view.remove_song_data();
        assert_eq!(view.folder_songs.len(), 2);
        assert_eq!(view.folder_songs[0].metadata.title, "S1");
        assert_eq!(view.folder_songs[1].metadata.title, "S3");
    }

    #[test]
    fn test_remove_song_data_no_selection() {
        let mut view = FolderEditorView::new();
        view.folder_songs = vec![make_song("S1", "a", "x")];
        view.folder_songs_selected_index = None;

        view.remove_song_data();
        assert_eq!(view.folder_songs.len(), 1);
    }

    // ---- moveSongDataUp/Down ----

    #[test]
    fn test_move_song_data_up() {
        let mut view = FolderEditorView::new();
        view.folder_songs = vec![
            make_song("A", "1", "x1"),
            make_song("B", "2", "x2"),
            make_song("C", "3", "x3"),
        ];
        view.folder_songs_selected_index = Some(2);

        view.move_song_data_up();
        assert_eq!(view.folder_songs[1].metadata.title, "C");
        assert_eq!(view.folder_songs[2].metadata.title, "B");
        assert_eq!(view.folder_songs_selected_index, Some(1));
    }

    #[test]
    fn test_move_song_data_up_at_top() {
        let mut view = FolderEditorView::new();
        view.folder_songs = vec![make_song("A", "1", "x1"), make_song("B", "2", "x2")];
        view.folder_songs_selected_index = Some(0);

        view.move_song_data_up();
        assert_eq!(view.folder_songs[0].metadata.title, "A");
        assert_eq!(view.folder_songs_selected_index, Some(0));
    }

    #[test]
    fn test_move_song_data_down() {
        let mut view = FolderEditorView::new();
        view.folder_songs = vec![
            make_song("A", "1", "x1"),
            make_song("B", "2", "x2"),
            make_song("C", "3", "x3"),
        ];
        view.folder_songs_selected_index = Some(0);

        view.move_song_data_down();
        assert_eq!(view.folder_songs[0].metadata.title, "B");
        assert_eq!(view.folder_songs[1].metadata.title, "A");
        assert_eq!(view.folder_songs_selected_index, Some(1));
    }

    #[test]
    fn test_move_song_data_down_at_bottom() {
        let mut view = FolderEditorView::new();
        view.folder_songs = vec![make_song("A", "1", "x1"), make_song("B", "2", "x2")];
        view.folder_songs_selected_index = Some(1);

        view.move_song_data_down();
        assert_eq!(view.folder_songs[1].metadata.title, "B");
        assert_eq!(view.folder_songs_selected_index, Some(1));
    }

    // ---- commitFolder ----

    #[test]
    fn test_commit_folder_saves_name_and_songs() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("Original", vec![])];
        view.selected_folder = Some(0);
        view.folder_name = "Renamed".to_string();
        view.folder_songs = vec![make_song("New Song", "md5", "sha")];

        view.commit_folder();
        assert_eq!(view.folders[0].name(), "Renamed");
        assert_eq!(view.folders[0].songs.len(), 1);
        assert_eq!(view.folders[0].songs[0].metadata.title, "New Song");
    }

    #[test]
    fn test_commit_folder_no_selection() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("Original", vec![])];
        view.selected_folder = None;
        view.folder_name = "Changed".to_string();

        view.commit_folder();
        assert_eq!(view.folders[0].name(), "Original");
    }

    #[test]
    fn test_commit_folder_out_of_bounds() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("Original", vec![])];
        view.selected_folder = Some(5);
        view.folder_name = "Changed".to_string();

        view.commit_folder();
        assert_eq!(view.folders[0].name(), "Original");
    }

    // ---- updateFolder ----

    #[test]
    fn test_update_folder_none_hides_pane() {
        let mut view = FolderEditorView::new();
        view.folder_pane_visible = true;

        view.update_folder(None);
        assert!(!view.folder_pane_visible);
        assert!(view.selected_folder.is_none());
    }

    #[test]
    fn test_update_folder_shows_folder_data() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder(
            "Test Folder",
            vec![make_song("S1", "m1", "s1")],
        )];

        view.update_folder(Some(0));
        assert!(view.folder_pane_visible);
        assert_eq!(view.selected_folder, Some(0));
        assert_eq!(view.folder_name, "Test Folder");
        assert_eq!(view.folder_songs.len(), 1);
        assert_eq!(view.folder_songs[0].metadata.title, "S1");
    }

    #[test]
    fn test_update_folder_out_of_bounds_hides_pane() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("A", vec![])];

        view.update_folder(Some(5));
        assert!(!view.folder_pane_visible);
    }

    // ---- updateTableFolder ----

    #[test]
    fn test_update_table_folder_commits_and_updates() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("Before", vec![])];
        view.selected_folder = Some(0);
        view.folder_name = "After".to_string();
        view.folders_selected_index = Some(0);

        view.update_table_folder();
        assert_eq!(view.folders[0].name(), "After");
        assert_eq!(view.folder_name, "After");
    }

    #[test]
    fn test_update_table_folder_no_selection() {
        let mut view = FolderEditorView::new();
        view.folders = vec![make_folder("A", vec![])];
        view.folders_selected_index = None;
        view.selected_folder = None;

        view.update_table_folder();
        assert!(!view.folder_pane_visible);
    }

    // ---- searchSongs ----

    #[test]
    fn test_search_songs_no_songdb() {
        let mut view = FolderEditorView::new();
        view.search = "test".to_string();
        view.songdb = None;

        view.search_songs();
        assert!(view.search_songs.is_empty());
    }

    #[test]
    fn test_search_songs_with_hash() {
        let mut view = FolderEditorView::new();
        view.songdb = Some(Box::new(MockSongDb));
        view.search = "abcdef1234567890abcdef1234567890".to_string();

        view.search_songs();
        assert!(view.search_songs.is_empty());
    }

    #[test]
    fn test_search_songs_with_text() {
        let mut view = FolderEditorView::new();
        view.songdb = Some(Box::new(MockSongDb));
        view.search = "test query".to_string();

        view.search_songs();
        assert!(view.search_songs.is_empty());
    }

    #[test]
    fn test_search_songs_short_text_skipped() {
        let mut view = FolderEditorView::new();
        view.songdb = Some(Box::new(MockSongDb));
        view.search = "a".to_string();

        view.search_songs();
        assert!(view.search_songs.is_empty());
    }

    // ---- getFoldersContainingSong ----

    #[test]
    fn test_get_folders_containing_song_by_md5() {
        let folders = vec![
            make_folder("Folder A", vec![make_song("S1", "hash1", "sha1")]),
            make_folder("Folder B", vec![make_song("S2", "hash2", "sha2")]),
        ];
        let song = make_song("Query", "hash1", "");

        let result = FolderEditorView::folders_containing_song(&folders, &song);
        assert_eq!(result, "Folder A");
    }

    #[test]
    fn test_get_folders_containing_song_by_sha256() {
        let folders = vec![
            make_folder("Folder A", vec![make_song("S1", "m1", "sha_match")]),
            make_folder("Folder B", vec![make_song("S2", "m2", "sha_other")]),
        ];
        let song = make_song("Query", "", "sha_match");

        let result = FolderEditorView::folders_containing_song(&folders, &song);
        assert_eq!(result, "Folder A");
    }

    #[test]
    fn test_get_folders_containing_song_multiple_folders() {
        let folders = vec![
            make_folder("Folder A", vec![make_song("S1", "hash1", "sha1")]),
            make_folder("Folder B", vec![make_song("S2", "hash1", "sha2")]),
            make_folder("Folder C", vec![make_song("S3", "other", "other")]),
        ];
        let song = make_song("Query", "hash1", "");

        let result = FolderEditorView::folders_containing_song(&folders, &song);
        assert_eq!(result, "Folder A, Folder B");
    }

    #[test]
    fn test_get_folders_containing_song_none() {
        let folders = vec![make_folder(
            "Folder A",
            vec![make_song("S1", "hash1", "sha1")],
        )];
        let song = make_song("Query", "nomatch", "nomatch");

        let result = FolderEditorView::folders_containing_song(&folders, &song);
        assert_eq!(result, "None");
    }

    #[test]
    fn test_get_folders_containing_song_empty_hashes_no_match() {
        let folders = vec![make_folder("Folder A", vec![make_song("S1", "", "")])];
        let song = make_song("Query", "", "");

        let result = FolderEditorView::folders_containing_song(&folders, &song);
        assert_eq!(result, "None");
    }

    #[test]
    fn test_get_folders_containing_song_empty_folders() {
        let folders: Vec<TableFolder> = vec![];
        let song = make_song("Query", "hash", "sha");

        let result = FolderEditorView::folders_containing_song(&folders, &song);
        assert_eq!(result, "None");
    }

    // ---- round-trip: add → select → commit → get ----

    #[test]
    fn test_round_trip_add_edit_get() {
        let mut view = FolderEditorView::new();
        view.initialize();

        // Add a folder
        view.add_table_folder();
        assert_eq!(view.folders.len(), 1);

        // Select and edit
        view.folders_selected_index = Some(0);
        view.update_folder(Some(0));
        assert!(view.folder_pane_visible);
        assert_eq!(view.folder_name, "New Folder");

        // Modify
        view.folder_name = "Edited Folder".to_string();

        // Add a song
        view.search_songs_selected_items = vec![make_song("Test Song", "md5val", "shaval")];
        view.add_song_data();
        assert_eq!(view.folder_songs.len(), 1);

        // Get folder data (triggers commit)
        let folders = view.table_folder();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].name(), "Edited Folder");
        assert_eq!(folders[0].songs.len(), 1);
        assert_eq!(folders[0].songs[0].metadata.title, "Test Song");
    }
}
