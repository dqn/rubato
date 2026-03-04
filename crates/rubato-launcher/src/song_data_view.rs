// Translated from SongDataView.java

use std::collections::HashMap;

use egui;

/// Column definition for a SongData table.
/// Java: private TableColumn<SongData, T> with PropertyValueFactory
/// In Rust, we track column names and visibility.
#[derive(Clone, Debug)]
pub struct SongDataColumn {
    pub property_name: String,
    pub visible: bool,
}

/// SongDataView -- table view for displaying song data.
/// Java: SongDataView implements Initializable
/// Has @FXML TableColumn fields for title, artist, genre, mode, notes, level, sha256.
/// In Rust, this is a data struct tracking column definitions and visibility.
#[derive(Clone, Debug)]
pub struct SongDataView {
    // Java: private Map<String, TableColumn> columnMap = new HashMap<>();
    pub column_map: HashMap<String, SongDataColumn>,
    /// Column display order
    pub column_order: Vec<String>,
}

impl SongDataView {
    /// Creates and initializes a new SongDataView.
    /// Java: public void initialize(URL arg0, ResourceBundle arg1)
    /// Calls initColumn for each column: title, artist, genre, mode, notes, level, sha256.
    pub fn new() -> Self {
        let mut view = SongDataView {
            column_map: HashMap::new(),
            column_order: Vec::new(),
        };

        // Java: initColumn(title, "fullTitle");
        view.init_column("fullTitle");
        // Java: initColumn(artist, "fullArtist");
        view.init_column("fullArtist");
        // Java: initColumn(genre, "genre");
        view.init_column("genre");
        // Java: initColumn(mode, "mode");
        view.init_column("mode");
        // Java: initColumn(notes, "notes");
        view.init_column("notes");
        // Java: initColumn(level, "level");
        view.init_column("level");
        // Java: initColumn(sha256, "sha256");
        view.init_column("sha256");

        view
    }

    /// Initializes a column with a property name.
    /// Java: private void initColumn(TableColumn column, String value)
    /// - column.setCellValueFactory(new PropertyValueFactory(value));
    /// - columnMap.put(value, column);
    fn init_column(&mut self, value: &str) {
        let column = SongDataColumn {
            property_name: value.to_string(),
            visible: true,
        };
        self.column_order.push(value.to_string());
        self.column_map.insert(value.to_string(), column);
    }

    /// Sets visibility of columns.
    /// Java: public void setVisible(String... values)
    /// - First hides all columns
    /// - Then shows only the specified columns
    pub fn set_visible(&mut self, values: &[&str]) {
        // Java: for(TableColumn column : columnMap.values()) { column.setVisible(false); }
        for column in self.column_map.values_mut() {
            column.visible = false;
        }

        // Java: for(String value : values) { TableColumn column = columnMap.get(value); if(column != null) { column.setVisible(true); } }
        for value in values {
            if let Some(column) = self.column_map.get_mut(*value) {
                column.visible = true;
            }
        }
    }

    /// Gets the column map.
    pub fn get_column_map(&self) -> &HashMap<String, SongDataColumn> {
        &self.column_map
    }

    /// Checks if a column is visible.
    pub fn is_column_visible(&self, property_name: &str) -> bool {
        self.column_map
            .get(property_name)
            .map(|c| c.visible)
            .unwrap_or(false)
    }

    /// Render the SongData column visibility configuration UI.
    ///
    /// Shows a grid of checkboxes for toggling each column's visibility.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Song Data Columns");

        egui::Grid::new("song_data_columns_grid")
            .num_columns(2)
            .show(ui, |ui| {
                for name in &self.column_order.clone() {
                    if let Some(column) = self.column_map.get_mut(name) {
                        ui.label(name.as_str());
                        ui.checkbox(&mut column.visible, "");
                        ui.end_row();
                    }
                }
            });
    }
}

impl Default for SongDataView {
    fn default() -> Self {
        Self::new()
    }
}
