// Translated from FolderEditorView.java

use std::path::PathBuf;

use beatoraja_core::course_data::CourseData;
use beatoraja_core::main_controller::SongDatabaseAccessor;
use beatoraja_core::stubs::SongData;
use beatoraja_core::table_data::TableFolder;

use crate::table_editor_view::TableEditorView;

/// SongDataView stub — corresponds to the SongDataView FXML sub-controller
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct SongDataView {
    visible_columns: Vec<String>,
}

#[allow(dead_code)]
impl SongDataView {
    pub fn set_visible(&mut self, columns: &[&str]) {
        self.visible_columns = columns.iter().map(|s| s.to_string()).collect();
        // todo!("egui integration")
    }
}

/// FolderEditorView - folder editor with search, song data tables, folder list
///
/// JavaFX UI widgets are translated to data structs.
/// All rendering/UI operations use todo!("egui integration").
#[allow(dead_code)]
pub struct FolderEditorView {
    // JavaFX @FXML fields → egui widget state
    search: String,
    search_songs: Vec<SongData>,
    search_songs_controller: SongDataView,
    search_songs_selected_items: Vec<SongData>,
    search_songs_selected_index: Option<usize>,

    folders: Vec<TableFolder>,
    folders_selected_index: Option<usize>,
    folder_pane_visible: bool,
    folder_name: String,
    folder_songs: Vec<SongData>,
    folder_songs_controller: SongDataView,
    folder_songs_selected_index: Option<usize>,

    filepath: Option<PathBuf>,

    selected_folder: Option<usize>, // index into folders

    songdb: Option<SongDatabaseAccessor>,

    courses: Vec<CourseData>,
}

#[allow(dead_code)]
impl FolderEditorView {
    /// Constructor
    pub fn new() -> Self {
        Self {
            search: String::new(),
            search_songs: Vec::new(),
            search_songs_controller: SongDataView::default(),
            search_songs_selected_items: Vec::new(),
            search_songs_selected_index: None,

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
    pub fn init(&mut self, songdb: SongDatabaseAccessor) {
        self.songdb = Some(songdb);
    }

    /// searchSongs - searches for songs by hash or text
    pub fn search_songs(&mut self) {
        if self.songdb.is_none() {
            return;
        }
        if TableEditorView::is_md5_or_sha256_hash(&self.search) {
            // searchSongs.getItems().setAll(songdb.getSongDatas(new String[]{search.getText()}));
            let _songdb = self.songdb.as_ref().unwrap();
            // SongDatabaseAccessor.get_song_datas not yet implemented
            self.search_songs = Vec::new(); // stub
        } else if self.search.len() > 1 {
            // searchSongs.getItems().setAll(songdb.getSongDatasByText(search.getText()));
            let _songdb = self.songdb.as_ref().unwrap();
            // SongDatabaseAccessor.get_song_datas_by_text not yet implemented
            self.search_songs = Vec::new(); // stub
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
                self.folder_name = self.folders[idx]
                    .name
                    .clone()
                    .unwrap_or_default();
                // folderSongs.getItems().setAll(course.getSong());
                self.folder_songs = self.folders[idx].songs.clone();
            }
        }
    }

    /// addTableFolder - adds a new empty folder
    pub fn add_table_folder(&mut self) {
        let mut folder = TableFolder::default();
        folder.name = Some("New Folder".to_string());
        self.folders.push(folder);
    }

    /// removeTableFolder - removes the currently selected folder
    pub fn remove_table_folder(&mut self) {
        if let Some(idx) = self.folders_selected_index {
            if idx < self.folders.len() {
                self.folders.remove(idx);
            }
        }
    }

    /// moveTableFolderUp - moves the selected folder up one position
    pub fn move_table_folder_up(&mut self) {
        if let Some(index) = self.folders_selected_index {
            if index > 0 {
                self.folders.swap(index, index - 1);
                self.folders_selected_index = Some(index - 1);
            }
        }
    }

    /// moveTableFolderDown - moves the selected folder down one position
    pub fn move_table_folder_down(&mut self) {
        if let Some(index) = self.folders_selected_index {
            if index < self.folders.len().saturating_sub(1) {
                self.folders.swap(index, index + 1);
                self.folders_selected_index = Some(index + 1);
            }
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
        if let Some(idx) = self.folder_songs_selected_index {
            if idx < self.folder_songs.len() {
                self.folder_songs.remove(idx);
            }
        }
    }

    /// moveSongDataUp - moves the selected song up one position
    pub fn move_song_data_up(&mut self) {
        if let Some(index) = self.folder_songs_selected_index {
            if index > 0 {
                self.folder_songs.swap(index, index - 1);
                self.folder_songs_selected_index = Some(index - 1);
            }
        }
    }

    /// moveSongDataDown - moves the selected song down one position
    pub fn move_song_data_down(&mut self) {
        if let Some(index) = self.folder_songs_selected_index {
            if index < self.folder_songs.len().saturating_sub(1) {
                self.folder_songs.swap(index, index + 1);
                self.folder_songs_selected_index = Some(index + 1);
            }
        }
    }

    /// getTableFolder - commits and returns all folders
    pub fn get_table_folder(&mut self) -> Vec<TableFolder> {
        self.commit_folder();
        self.folders.clone()
    }

    /// setTableFolder - sets the folder list
    pub fn set_table_folder(&mut self, folder: Vec<TableFolder>) {
        self.folders = folder;
    }

    /// getFoldersContainingSong - finds which folders contain a given song
    pub fn get_folders_containing_song(folders: &[TableFolder], song: &SongData) -> String {
        let mut sb = String::new();
        for i in 0..folders.len() {
            let songs = &folders[i].songs;
            for j in 0..songs.len() {
                let ts = &songs[j];
                let ts_md5 = ts.md5.as_deref().unwrap_or("");
                let song_md5 = song.md5.as_deref().unwrap_or("");
                let ts_sha256 = ts.sha256.as_deref().unwrap_or("");
                let song_sha256 = song.sha256.as_deref().unwrap_or("");

                if (!ts_md5.is_empty()
                    && !song_md5.is_empty()
                    && ts_md5 == song_md5)
                    || (!ts_sha256.is_empty()
                        && !song_sha256.is_empty()
                        && ts_sha256 == song_sha256)
                {
                    if !sb.is_empty() {
                        sb.push_str(", ");
                    }
                    sb.push_str(folders[i].name.as_deref().unwrap_or(""));
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
    fn display_chart_details_dialog(&self, song: &SongData) {
        let extra = format!(
            "In custom folder(s):\n{}",
            Self::get_folders_containing_song(&self.folders, song)
        );
        TableEditorView::display_chart_details_dialog(self.songdb.as_ref(), song, &[&extra]);
    }
}

impl Default for FolderEditorView {
    fn default() -> Self {
        Self::new()
    }
}
