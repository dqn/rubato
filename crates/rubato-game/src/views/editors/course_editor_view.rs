// Translated from CourseEditorView.java

use crate::core::course_data::{CourseData, CourseDataConstraint, TrophyData};
use egui;
use rubato_types::song_data::SongData;
use rubato_types::song_database_accessor::SongDatabaseAccessor;

use crate::views::editors::folder_editor_view::SongDataView;
use crate::views::editors::table_editor_view::TableEditorView;

/// CourseEditorView - course editor with constraints, trophies, song search
///
/// JavaFX UI widgets are translated to data structs.
/// Rendering will be implemented via egui when the course editor tab is wired.
pub struct CourseEditorView {
    // JavaFX @FXML fields → egui widget state
    search: String,
    search_songs: Vec<SongData>,
    search_songs_controller: SongDataView,
    search_songs_selected_items: Vec<SongData>,

    pub courses: Vec<CourseData>,
    courses_selected_index: Option<usize>,
    course_pane_visible: bool,
    course_name: String,
    release: bool,
    grade_type: Option<CourseDataConstraint>,
    hispeed_type: Option<CourseDataConstraint>,
    judge_type: Option<CourseDataConstraint>,
    gauge_type: Option<CourseDataConstraint>,
    ln_type: Option<CourseDataConstraint>,
    bronzemiss: f64,
    bronzescore: f64,
    silvermiss: f64,
    silverscore: f64,
    goldmiss: f64,
    goldscore: f64,
    course_songs: Vec<SongData>,
    course_songs_controller: SongDataView,
    course_songs_selected_index: Option<usize>,

    // ComboBox items for each constraint type
    grade_type_items: Vec<Option<CourseDataConstraint>>,
    hispeed_type_items: Vec<Option<CourseDataConstraint>>,
    judge_type_items: Vec<Option<CourseDataConstraint>>,
    gauge_type_items: Vec<Option<CourseDataConstraint>>,
    ln_type_items: Vec<Option<CourseDataConstraint>>,

    _filename: String,

    selected_course: Option<usize>, // index into courses

    songdb: Option<Box<dyn SongDatabaseAccessor>>,
}

impl CourseEditorView {
    /// Constructor
    pub fn new() -> Self {
        Self {
            search: String::new(),
            search_songs: Vec::new(),
            search_songs_controller: SongDataView::default(),
            search_songs_selected_items: Vec::new(),

            courses: Vec::new(),
            courses_selected_index: None,
            course_pane_visible: false,
            course_name: String::new(),
            release: false,
            grade_type: None,
            hispeed_type: None,
            judge_type: None,
            gauge_type: None,
            ln_type: None,
            bronzemiss: 0.0,
            bronzescore: 0.0,
            silvermiss: 0.0,
            silverscore: 0.0,
            goldmiss: 0.0,
            goldscore: 0.0,
            course_songs: Vec::new(),
            course_songs_controller: SongDataView::default(),
            course_songs_selected_index: None,

            // gradeType.getItems().setAll(null, CLASS, MIRROR, RANDOM);
            grade_type_items: vec![
                None,
                Some(CourseDataConstraint::Class),
                Some(CourseDataConstraint::Mirror),
                Some(CourseDataConstraint::Random),
            ],
            // hispeedType.getItems().setAll(null, NO_SPEED);
            hispeed_type_items: vec![None, Some(CourseDataConstraint::NoSpeed)],
            // judgeType.getItems().setAll(null, NO_GOOD, NO_GREAT);
            judge_type_items: vec![
                None,
                Some(CourseDataConstraint::NoGood),
                Some(CourseDataConstraint::NoGreat),
            ],
            // gaugeType.getItems().setAll(null, GAUGE_LR2, GAUGE_5KEYS, GAUGE_7KEYS, GAUGE_9KEYS, GAUGE_24KEYS);
            gauge_type_items: vec![
                None,
                Some(CourseDataConstraint::GaugeLr2),
                Some(CourseDataConstraint::Gauge5Keys),
                Some(CourseDataConstraint::Gauge7Keys),
                Some(CourseDataConstraint::Gauge9Keys),
                Some(CourseDataConstraint::Gauge24Keys),
            ],
            // lnType.getItems().setAll(null, LN, CN, HCN);
            ln_type_items: vec![
                None,
                Some(CourseDataConstraint::Ln),
                Some(CourseDataConstraint::Cn),
                Some(CourseDataConstraint::Hcn),
            ],

            _filename: String::new(),

            selected_course: None,

            songdb: None,
        }
    }

    /// initialize - corresponds to Initializable.initialize(URL, ResourceBundle)
    pub fn initialize(&mut self) {
        // Constraint combo box items are set in the constructor

        // courses.getSelectionModel().selectedIndexProperty().addListener(...)
        // → handled by UI framework event system in egui

        // courses.setCellFactory(...)
        // → cell rendering handled by egui

        // courseSongsController.setVisible("fullTitle", "sha256");
        self.course_songs_controller
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
        // courseSongs.setOnMouseClicked(...)
        // → double-click handlers handled by egui event system

        self.update_course(None);
    }

    /// setSongDatabaseAccessor - sets the song database accessor
    pub fn set_song_database_accessor(&mut self, songdb: Box<dyn SongDatabaseAccessor>) {
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

    /// getCourseData - commits and returns all courses
    pub fn course_data(&mut self) -> Vec<CourseData> {
        self.commit_course();
        self.courses.clone()
    }
    /// updateCourseData - commits current course and updates to selected course
    pub fn update_course_data(&mut self) {
        self.commit_course();
        let selected_idx = self.courses_selected_index;
        if let Some(idx) = selected_idx {
            self.update_course(Some(idx));
        } else {
            self.update_course(None);
        }
    }

    /// commitCourse - saves current course state from UI
    fn commit_course(&mut self) {
        let course_idx = match self.selected_course {
            Some(idx) => idx,
            None => return,
        };

        if course_idx >= self.courses.len() {
            return;
        }

        // selectedCourse.setName(courseName.getText());
        self.courses[course_idx].name = Some(self.course_name.clone());
        // selectedCourse.setRelease(release.isSelected());
        self.courses[course_idx].release = self.release;

        // Build constraint list from combo boxes
        let mut constraint = Vec::new();
        // if(gradeType.getValue() != null) { constraint.add(gradeType.getValue()); }
        if let Some(c) = self.grade_type {
            constraint.push(c);
        }
        // if(hispeedType.getValue() != null) { constraint.add(hispeedType.getValue()); }
        if let Some(c) = self.hispeed_type {
            constraint.push(c);
        }
        // if(judgeType.getValue() != null) { constraint.add(judgeType.getValue()); }
        if let Some(c) = self.judge_type {
            constraint.push(c);
        }
        // if(gaugeType.getValue() != null) { constraint.add(gaugeType.getValue()); }
        if let Some(c) = self.gauge_type {
            constraint.push(c);
        }
        // if(lnType.getValue() != null) { constraint.add(lnType.getValue()); }
        if let Some(c) = self.ln_type {
            constraint.push(c);
        }
        self.courses[course_idx].constraint = constraint;

        // Build trophy array
        // trophy[0] = new CourseData.TrophyData("bronzemedal", bronzemiss, bronzescore);
        // trophy[1] = new CourseData.TrophyData("silvermedal", silvermiss, silverscore);
        // trophy[2] = new CourseData.TrophyData("goldmedal", goldmiss, goldscore);
        let trophy = vec![
            TrophyData::new(
                "bronzemedal".to_string(),
                self.bronzemiss as f32,
                self.bronzescore as f32,
            ),
            TrophyData::new(
                "silvermedal".to_string(),
                self.silvermiss as f32,
                self.silverscore as f32,
            ),
            TrophyData::new(
                "goldmedal".to_string(),
                self.goldmiss as f32,
                self.goldscore as f32,
            ),
        ];
        self.courses[course_idx].trophy = trophy;

        // selectedCourse.setSong(courseSongs.getItems().toArray(...));
        self.courses[course_idx].hash = self.course_songs.clone();
    }

    /// updateCourse - updates UI to show the given course
    fn update_course(&mut self, course_idx: Option<usize>) {
        self.selected_course = course_idx;
        match course_idx {
            None => {
                // coursePane.setVisible(false);
                self.course_pane_visible = false;
            }
            Some(idx) => {
                if idx >= self.courses.len() {
                    self.course_pane_visible = false;
                    return;
                }
                // coursePane.setVisible(true);
                self.course_pane_visible = true;

                let course = self.courses[idx].clone();

                // courseName.setText(selectedCourse.getName());
                self.course_name = course.name.clone().unwrap_or_default();
                // release.setSelected(selectedCourse.isRelease());
                self.release = course.release;

                // Reset all constraint combo boxes to null
                self.grade_type = None;
                self.judge_type = None;
                self.hispeed_type = None;
                self.gauge_type = None;
                self.ln_type = None;

                // for(CourseData.CourseDataConstraint constraint : course.getConstraint()) { switch(constraint) { ... } }
                for constraint in &course.constraint {
                    match constraint {
                        CourseDataConstraint::Class
                        | CourseDataConstraint::Mirror
                        | CourseDataConstraint::Random => {
                            self.grade_type = Some(*constraint);
                        }
                        CourseDataConstraint::NoGreat | CourseDataConstraint::NoGood => {
                            self.judge_type = Some(*constraint);
                        }
                        CourseDataConstraint::NoSpeed => {
                            self.hispeed_type = Some(*constraint);
                        }
                        CourseDataConstraint::Gauge24Keys
                        | CourseDataConstraint::Gauge5Keys
                        | CourseDataConstraint::Gauge7Keys
                        | CourseDataConstraint::Gauge9Keys
                        | CourseDataConstraint::GaugeLr2 => {
                            self.gauge_type = Some(*constraint);
                        }
                        CourseDataConstraint::Ln
                        | CourseDataConstraint::Cn
                        | CourseDataConstraint::Hcn => {
                            self.ln_type = Some(*constraint);
                        }
                    }
                }

                // for(CourseData.TrophyData trophy : course.getTrophy()) { ... }
                for trophy in &course.trophy {
                    let trophy_name = trophy.name.as_deref().unwrap_or("");
                    if trophy_name == "bronzemedal" {
                        // bronzemiss.getValueFactory().setValue(Double.valueOf(trophy.getMissrate()));
                        self.bronzemiss = trophy.missrate as f64;
                        // bronzescore.getValueFactory().setValue(Double.valueOf(trophy.getScorerate()));
                        self.bronzescore = trophy.scorerate as f64;
                    }
                    if trophy_name == "silvermedal" {
                        // silvermiss.getValueFactory().setValue(Double.valueOf(trophy.getMissrate()));
                        self.silvermiss = trophy.missrate as f64;
                        // silverscore.getValueFactory().setValue(Double.valueOf(trophy.getScorerate()));
                        self.silverscore = trophy.scorerate as f64;
                    }
                    if trophy_name == "goldmedal" {
                        // goldmiss.getValueFactory().setValue(Double.valueOf(trophy.getMissrate()));
                        self.goldmiss = trophy.missrate as f64;
                        // goldscore.getValueFactory().setValue(Double.valueOf(trophy.getScorerate()));
                        self.goldscore = trophy.scorerate as f64;
                    }
                }

                // courseSongs.getItems().setAll(course.getSong());
                self.course_songs = course.hash.clone();
            }
        }
    }

    /// getValue - helper to get spinner value (in Java, forces text → value conversion)
    /// In Rust, f64 fields are used directly.
    #[cfg(test)]
    fn value(value: f64) -> f64 {
        // In Java:
        // spinner.getValueFactory().setValue(
        //     spinner.getValueFactory().getConverter().fromString(spinner.getEditor().getText())
        // );
        // return spinner.getValue();
        value
    }

    /// addCourseData - adds a new course with default values
    pub fn add_course_data(&mut self) {
        // CourseData.TrophyData[] trophy = new CourseData.TrophyData[3];
        // trophy[0] = new CourseData.TrophyData("bronzemedal", 7.5f, 55.0f);
        // trophy[1] = new CourseData.TrophyData("silvermedal", 5.0f, 70.0f);
        // trophy[2] = new CourseData.TrophyData("goldmedal", 2.5f, 85.0f);
        let course = CourseData {
            name: Some("New Course".to_string()),
            release: false,
            trophy: vec![
                TrophyData::new("bronzemedal".to_string(), 7.5, 55.0),
                TrophyData::new("silvermedal".to_string(), 5.0, 70.0),
                TrophyData::new("goldmedal".to_string(), 2.5, 85.0),
            ],
            ..Default::default()
        };
        self.courses.push(course);
    }

    /// removeCourseData - removes the currently selected course
    pub fn remove_course_data(&mut self) {
        if let Some(idx) = self.courses_selected_index
            && idx < self.courses.len()
        {
            self.courses.remove(idx);
        }
    }

    /// moveCourseDataUp - moves the selected course up one position
    pub fn move_course_data_up(&mut self) {
        if let Some(index) = self.courses_selected_index
            && index > 0
        {
            self.courses.swap(index, index - 1);
            self.courses_selected_index = Some(index - 1);
        }
    }

    /// moveCourseDataDown - moves the selected course down one position
    pub fn move_course_data_down(&mut self) {
        if let Some(index) = self.courses_selected_index
            && index < self.courses.len().saturating_sub(1)
        {
            self.courses.swap(index, index + 1);
            self.courses_selected_index = Some(index + 1);
        }
    }

    /// addSongData - adds selected search songs to the current course
    pub fn add_song_data(&mut self) {
        // List<SongData> songs = searchSongs.getSelectionModel().getSelectedItems();
        for song in &self.search_songs_selected_items.clone() {
            self.course_songs.push(song.clone());
        }
    }

    /// removeSongData - removes the selected song from the course
    pub fn remove_song_data(&mut self) {
        if let Some(idx) = self.course_songs_selected_index
            && idx < self.course_songs.len()
        {
            self.course_songs.remove(idx);
        }
    }

    /// moveSongDataUp - moves the selected song up one position
    pub fn move_song_data_up(&mut self) {
        if let Some(index) = self.course_songs_selected_index
            && index > 0
        {
            self.course_songs.swap(index, index - 1);
            self.course_songs_selected_index = Some(index - 1);
        }
    }

    /// moveSongDataDown - moves the selected song down one position
    pub fn move_song_data_down(&mut self) {
        if let Some(index) = self.course_songs_selected_index
            && index < self.course_songs.len().saturating_sub(1)
        {
            self.course_songs.swap(index, index + 1);
            self.course_songs_selected_index = Some(index + 1);
        }
    }

    /// Render the course editor UI.
    ///
    /// Layout: course list on the left, course detail pane on the right.
    /// Song search panel at the bottom.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        // --- Course list panel ---
        ui.horizontal(|ui| {
            ui.label("Courses:");
            if ui.button("Add").clicked() {
                self.add_course_data();
            }
            if ui.button("Remove").clicked() {
                self.remove_course_data();
                self.update_course(None);
            }
            if ui.button("Up").clicked() {
                self.move_course_data_up();
            }
            if ui.button("Down").clicked() {
                self.move_course_data_down();
            }
        });

        egui::ScrollArea::vertical()
            .id_salt("course_list_scroll")
            .max_height(120.0)
            .show(ui, |ui| {
                let mut new_selection = self.courses_selected_index;
                for (i, course) in self.courses.iter().enumerate() {
                    let name = course.name.as_deref().unwrap_or("(unnamed)");
                    let selected = self.courses_selected_index == Some(i);
                    if ui.selectable_label(selected, name).clicked() {
                        new_selection = Some(i);
                    }
                }
                if new_selection != self.courses_selected_index {
                    self.commit_course();
                    self.courses_selected_index = new_selection;
                    self.update_course(new_selection);
                }
            });

        ui.separator();

        // --- Course detail pane ---
        if self.course_pane_visible {
            egui::Grid::new("course_detail_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Course Name:");
                    ui.text_edit_singleline(&mut self.course_name);
                    ui.end_row();

                    ui.label("Release:");
                    ui.checkbox(&mut self.release, "");
                    ui.end_row();
                });

            // Constraint combo boxes
            ui.collapsing("Constraints", |ui| {
                Self::render_constraint_combo(
                    ui,
                    "Grade",
                    "course_grade_type",
                    &self.grade_type_items,
                    &mut self.grade_type,
                );
                Self::render_constraint_combo(
                    ui,
                    "Hi-Speed",
                    "course_hispeed_type",
                    &self.hispeed_type_items,
                    &mut self.hispeed_type,
                );
                Self::render_constraint_combo(
                    ui,
                    "Judge",
                    "course_judge_type",
                    &self.judge_type_items,
                    &mut self.judge_type,
                );
                Self::render_constraint_combo(
                    ui,
                    "Gauge",
                    "course_gauge_type",
                    &self.gauge_type_items,
                    &mut self.gauge_type,
                );
                Self::render_constraint_combo(
                    ui,
                    "LN Type",
                    "course_ln_type",
                    &self.ln_type_items,
                    &mut self.ln_type,
                );
            });

            // Trophy settings
            ui.collapsing("Trophies", |ui| {
                egui::Grid::new("trophy_grid")
                    .num_columns(3)
                    .show(ui, |ui| {
                        ui.label("");
                        ui.label("Miss Rate");
                        ui.label("Score Rate");
                        ui.end_row();

                        ui.label("Bronze:");
                        ui.add(egui::DragValue::new(&mut self.bronzemiss).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.bronzescore).speed(0.1));
                        ui.end_row();

                        ui.label("Silver:");
                        ui.add(egui::DragValue::new(&mut self.silvermiss).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.silverscore).speed(0.1));
                        ui.end_row();

                        ui.label("Gold:");
                        ui.add(egui::DragValue::new(&mut self.goldmiss).speed(0.1));
                        ui.add(egui::DragValue::new(&mut self.goldscore).speed(0.1));
                        ui.end_row();
                    });
            });

            ui.separator();

            // --- Course songs ---
            ui.horizontal(|ui| {
                ui.label("Course Songs:");
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
                .id_salt("course_songs_scroll")
                .max_height(100.0)
                .show(ui, |ui| {
                    for (i, song) in self.course_songs.iter().enumerate() {
                        let selected = self.course_songs_selected_index == Some(i);
                        let label =
                            format!("{} [{}]", song.metadata.full_title(), &song.file.sha256);
                        if ui.selectable_label(selected, &label).clicked() {
                            self.course_songs_selected_index = Some(i);
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

        if ui.button("Add Selected to Course").clicked() {
            self.add_song_data();
        }

        egui::ScrollArea::vertical()
            .id_salt("search_songs_scroll")
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

    /// Helper to render a constraint ComboBox.
    fn render_constraint_combo(
        ui: &mut egui::Ui,
        label: &str,
        id_salt: &str,
        items: &[Option<CourseDataConstraint>],
        current: &mut Option<CourseDataConstraint>,
    ) {
        ui.horizontal(|ui| {
            ui.label(format!("{}:", label));
            let selected_text = current.map_or("(None)".to_string(), |c| format!("{:?}", c));
            egui::ComboBox::from_id_salt(id_salt)
                .selected_text(&selected_text)
                .show_ui(ui, |ui| {
                    for item in items {
                        let display = item.map_or("(None)".to_string(), |c| format!("{:?}", c));
                        ui.selectable_value(current, *item, &display);
                    }
                });
        });
    }
}

impl Default for CourseEditorView {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::test_support::TestSongDb;

    fn make_song(title: &str, sha256: &str) -> SongData {
        let mut sd = SongData::new();
        sd.metadata.title = title.to_string();
        sd.file.sha256 = sha256.to_string();
        sd
    }

    fn make_course(name: &str) -> CourseData {
        CourseData {
            name: Some(name.to_string()),
            release: false,
            trophy: vec![
                TrophyData::new("bronzemedal".to_string(), 7.5, 55.0),
                TrophyData::new("silvermedal".to_string(), 5.0, 70.0),
                TrophyData::new("goldmedal".to_string(), 2.5, 85.0),
            ],
            ..Default::default()
        }
    }

    // ---- Construction ----

    #[test]
    fn test_new_defaults() {
        let view = CourseEditorView::new();
        assert!(view.search.is_empty());
        assert!(view.search_songs.is_empty());
        assert!(view.courses.is_empty());
        assert!(view.courses_selected_index.is_none());
        assert!(!view.course_pane_visible);
        assert!(view.course_name.is_empty());
        assert!(!view.release);
        assert!(view.grade_type.is_none());
        assert!(view.hispeed_type.is_none());
        assert!(view.judge_type.is_none());
        assert!(view.gauge_type.is_none());
        assert!(view.ln_type.is_none());
        assert_eq!(view.bronzemiss, 0.0);
        assert_eq!(view.bronzescore, 0.0);
        assert_eq!(view.silvermiss, 0.0);
        assert_eq!(view.silverscore, 0.0);
        assert_eq!(view.goldmiss, 0.0);
        assert_eq!(view.goldscore, 0.0);
        assert!(view.course_songs.is_empty());
        assert!(view.selected_course.is_none());
        assert!(view.songdb.is_none());
    }

    #[test]
    fn test_default_trait() {
        let view = CourseEditorView::default();
        assert!(view.courses.is_empty());
    }

    // ---- Constraint combo box items ----

    #[test]
    fn test_grade_type_items() {
        let view = CourseEditorView::new();
        assert_eq!(view.grade_type_items.len(), 4);
        assert_eq!(view.grade_type_items[0], None);
        assert_eq!(view.grade_type_items[1], Some(CourseDataConstraint::Class));
        assert_eq!(view.grade_type_items[2], Some(CourseDataConstraint::Mirror));
        assert_eq!(view.grade_type_items[3], Some(CourseDataConstraint::Random));
    }

    #[test]
    fn test_hispeed_type_items() {
        let view = CourseEditorView::new();
        assert_eq!(view.hispeed_type_items.len(), 2);
        assert_eq!(view.hispeed_type_items[0], None);
        assert_eq!(
            view.hispeed_type_items[1],
            Some(CourseDataConstraint::NoSpeed)
        );
    }

    #[test]
    fn test_judge_type_items() {
        let view = CourseEditorView::new();
        assert_eq!(view.judge_type_items.len(), 3);
        assert_eq!(view.judge_type_items[0], None);
        assert_eq!(view.judge_type_items[1], Some(CourseDataConstraint::NoGood));
        assert_eq!(
            view.judge_type_items[2],
            Some(CourseDataConstraint::NoGreat)
        );
    }

    #[test]
    fn test_gauge_type_items() {
        let view = CourseEditorView::new();
        assert_eq!(view.gauge_type_items.len(), 6);
        assert_eq!(view.gauge_type_items[0], None);
        assert_eq!(
            view.gauge_type_items[1],
            Some(CourseDataConstraint::GaugeLr2)
        );
        assert_eq!(
            view.gauge_type_items[5],
            Some(CourseDataConstraint::Gauge24Keys)
        );
    }

    #[test]
    fn test_ln_type_items() {
        let view = CourseEditorView::new();
        assert_eq!(view.ln_type_items.len(), 4);
        assert_eq!(view.ln_type_items[0], None);
        assert_eq!(view.ln_type_items[1], Some(CourseDataConstraint::Ln));
        assert_eq!(view.ln_type_items[2], Some(CourseDataConstraint::Cn));
        assert_eq!(view.ln_type_items[3], Some(CourseDataConstraint::Hcn));
    }

    // ---- initialize() ----

    #[test]
    fn test_initialize_sets_visible_columns() {
        let mut view = CourseEditorView::new();
        view.initialize();
        assert_eq!(
            view.course_songs_controller.visible_columns(),
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
    fn test_initialize_hides_course_pane() {
        let mut view = CourseEditorView::new();
        view.initialize();
        assert!(!view.course_pane_visible);
        assert!(view.selected_course.is_none());
    }

    // ---- set/get course data ----

    #[test]
    fn test_set_and_get_course_data() {
        let mut view = CourseEditorView::new();
        let courses = vec![make_course("Course A"), make_course("Course B")];
        view.courses = courses;
        let result = view.course_data();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name(), "Course A");
        assert_eq!(result[1].name(), "Course B");
    }

    // ---- addCourseData ----

    #[test]
    fn test_add_course_data() {
        let mut view = CourseEditorView::new();
        assert!(view.courses.is_empty());

        view.add_course_data();
        assert_eq!(view.courses.len(), 1);
        assert_eq!(view.courses[0].name(), "New Course");
        assert!(!view.courses[0].release);
        assert_eq!(view.courses[0].trophy.len(), 3);
        assert_eq!(view.courses[0].trophy[0].name(), "bronzemedal");
        assert_eq!(view.courses[0].trophy[0].missrate, 7.5);
        assert_eq!(view.courses[0].trophy[0].scorerate, 55.0);
        assert_eq!(view.courses[0].trophy[1].name(), "silvermedal");
        assert_eq!(view.courses[0].trophy[1].missrate, 5.0);
        assert_eq!(view.courses[0].trophy[1].scorerate, 70.0);
        assert_eq!(view.courses[0].trophy[2].name(), "goldmedal");
        assert_eq!(view.courses[0].trophy[2].missrate, 2.5);
        assert_eq!(view.courses[0].trophy[2].scorerate, 85.0);
    }

    // ---- removeCourseData ----

    #[test]
    fn test_remove_course_data() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A"), make_course("B"), make_course("C")];
        view.courses_selected_index = Some(1);

        view.remove_course_data();
        assert_eq!(view.courses.len(), 2);
        assert_eq!(view.courses[0].name(), "A");
        assert_eq!(view.courses[1].name(), "C");
    }

    #[test]
    fn test_remove_course_data_no_selection() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A")];
        view.courses_selected_index = None;

        view.remove_course_data();
        assert_eq!(view.courses.len(), 1);
    }

    // ---- moveCourseDataUp/Down ----

    #[test]
    fn test_move_course_data_up() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A"), make_course("B"), make_course("C")];
        view.courses_selected_index = Some(1);

        view.move_course_data_up();
        assert_eq!(view.courses[0].name(), "B");
        assert_eq!(view.courses[1].name(), "A");
        assert_eq!(view.courses_selected_index, Some(0));
    }

    #[test]
    fn test_move_course_data_up_at_top() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A"), make_course("B")];
        view.courses_selected_index = Some(0);

        view.move_course_data_up();
        assert_eq!(view.courses[0].name(), "A");
        assert_eq!(view.courses_selected_index, Some(0));
    }

    #[test]
    fn test_move_course_data_down() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A"), make_course("B"), make_course("C")];
        view.courses_selected_index = Some(1);

        view.move_course_data_down();
        assert_eq!(view.courses[1].name(), "C");
        assert_eq!(view.courses[2].name(), "B");
        assert_eq!(view.courses_selected_index, Some(2));
    }

    #[test]
    fn test_move_course_data_down_at_bottom() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A"), make_course("B")];
        view.courses_selected_index = Some(1);

        view.move_course_data_down();
        assert_eq!(view.courses[1].name(), "B");
        assert_eq!(view.courses_selected_index, Some(1));
    }

    // ---- addSongData / removeSongData ----

    #[test]
    fn test_add_song_data() {
        let mut view = CourseEditorView::new();
        view.search_songs_selected_items =
            vec![make_song("Song 1", "aaa"), make_song("Song 2", "bbb")];

        view.add_song_data();
        assert_eq!(view.course_songs.len(), 2);
        assert_eq!(view.course_songs[0].metadata.title, "Song 1");
        assert_eq!(view.course_songs[1].metadata.title, "Song 2");
    }

    #[test]
    fn test_remove_song_data() {
        let mut view = CourseEditorView::new();
        view.course_songs = vec![
            make_song("S1", "a"),
            make_song("S2", "b"),
            make_song("S3", "c"),
        ];
        view.course_songs_selected_index = Some(1);

        view.remove_song_data();
        assert_eq!(view.course_songs.len(), 2);
        assert_eq!(view.course_songs[0].metadata.title, "S1");
        assert_eq!(view.course_songs[1].metadata.title, "S3");
    }

    #[test]
    fn test_remove_song_data_no_selection() {
        let mut view = CourseEditorView::new();
        view.course_songs = vec![make_song("S1", "a")];
        view.course_songs_selected_index = None;

        view.remove_song_data();
        assert_eq!(view.course_songs.len(), 1);
    }

    // ---- moveSongDataUp/Down ----

    #[test]
    fn test_move_song_data_up() {
        let mut view = CourseEditorView::new();
        view.course_songs = vec![
            make_song("A", "1"),
            make_song("B", "2"),
            make_song("C", "3"),
        ];
        view.course_songs_selected_index = Some(2);

        view.move_song_data_up();
        assert_eq!(view.course_songs[1].metadata.title, "C");
        assert_eq!(view.course_songs[2].metadata.title, "B");
        assert_eq!(view.course_songs_selected_index, Some(1));
    }

    #[test]
    fn test_move_song_data_up_at_top() {
        let mut view = CourseEditorView::new();
        view.course_songs = vec![make_song("A", "1"), make_song("B", "2")];
        view.course_songs_selected_index = Some(0);

        view.move_song_data_up();
        assert_eq!(view.course_songs[0].metadata.title, "A");
        assert_eq!(view.course_songs_selected_index, Some(0));
    }

    #[test]
    fn test_move_song_data_down() {
        let mut view = CourseEditorView::new();
        view.course_songs = vec![
            make_song("A", "1"),
            make_song("B", "2"),
            make_song("C", "3"),
        ];
        view.course_songs_selected_index = Some(0);

        view.move_song_data_down();
        assert_eq!(view.course_songs[0].metadata.title, "B");
        assert_eq!(view.course_songs[1].metadata.title, "A");
        assert_eq!(view.course_songs_selected_index, Some(1));
    }

    #[test]
    fn test_move_song_data_down_at_bottom() {
        let mut view = CourseEditorView::new();
        view.course_songs = vec![make_song("A", "1"), make_song("B", "2")];
        view.course_songs_selected_index = Some(1);

        view.move_song_data_down();
        assert_eq!(view.course_songs[1].metadata.title, "B");
        assert_eq!(view.course_songs_selected_index, Some(1));
    }

    // ---- commitCourse ----

    #[test]
    fn test_commit_course_saves_name_and_release() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Original")];
        view.selected_course = Some(0);
        view.course_name = "Renamed".to_string();
        view.release = true;

        view.commit_course();
        assert_eq!(view.courses[0].name(), "Renamed");
        assert!(view.courses[0].release);
    }

    #[test]
    fn test_commit_course_saves_constraints() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Test")];
        view.selected_course = Some(0);
        view.course_name = "Test".to_string();
        view.grade_type = Some(CourseDataConstraint::Class);
        view.hispeed_type = Some(CourseDataConstraint::NoSpeed);
        view.judge_type = None;
        view.gauge_type = Some(CourseDataConstraint::GaugeLr2);
        view.ln_type = Some(CourseDataConstraint::Hcn);

        view.commit_course();
        assert_eq!(view.courses[0].constraint.len(), 4);
        assert!(
            view.courses[0]
                .constraint
                .contains(&CourseDataConstraint::Class)
        );
        assert!(
            view.courses[0]
                .constraint
                .contains(&CourseDataConstraint::NoSpeed)
        );
        assert!(
            view.courses[0]
                .constraint
                .contains(&CourseDataConstraint::GaugeLr2)
        );
        assert!(
            view.courses[0]
                .constraint
                .contains(&CourseDataConstraint::Hcn)
        );
    }

    #[test]
    fn test_commit_course_saves_trophies() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Test")];
        view.selected_course = Some(0);
        view.course_name = "Test".to_string();
        view.bronzemiss = 10.0;
        view.bronzescore = 50.0;
        view.silvermiss = 6.0;
        view.silverscore = 65.0;
        view.goldmiss = 3.0;
        view.goldscore = 80.0;

        view.commit_course();
        assert_eq!(view.courses[0].trophy.len(), 3);
        assert_eq!(view.courses[0].trophy[0].missrate, 10.0);
        assert_eq!(view.courses[0].trophy[0].scorerate, 50.0);
        assert_eq!(view.courses[0].trophy[1].missrate, 6.0);
        assert_eq!(view.courses[0].trophy[1].scorerate, 65.0);
        assert_eq!(view.courses[0].trophy[2].missrate, 3.0);
        assert_eq!(view.courses[0].trophy[2].scorerate, 80.0);
    }

    #[test]
    fn test_commit_course_saves_songs() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Test")];
        view.selected_course = Some(0);
        view.course_name = "Test".to_string();
        view.course_songs = vec![make_song("Song A", "hash1"), make_song("Song B", "hash2")];

        view.commit_course();
        assert_eq!(view.courses[0].hash.len(), 2);
        assert_eq!(view.courses[0].hash[0].metadata.title, "Song A");
        assert_eq!(view.courses[0].hash[1].metadata.title, "Song B");
    }

    #[test]
    fn test_commit_course_no_selection() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Original")];
        view.selected_course = None;
        view.course_name = "Changed".to_string();

        view.commit_course();
        // Should not modify anything
        assert_eq!(view.courses[0].name(), "Original");
    }

    #[test]
    fn test_commit_course_out_of_bounds() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Original")];
        view.selected_course = Some(5); // out of bounds
        view.course_name = "Changed".to_string();

        view.commit_course();
        assert_eq!(view.courses[0].name(), "Original");
    }

    // ---- updateCourse ----

    #[test]
    fn test_update_course_none_hides_pane() {
        let mut view = CourseEditorView::new();
        view.course_pane_visible = true;

        view.update_course(None);
        assert!(!view.course_pane_visible);
        assert!(view.selected_course.is_none());
    }

    #[test]
    fn test_update_course_shows_course_data() {
        let mut view = CourseEditorView::new();
        let mut course = make_course("Test Course");
        course.release = true;
        course.constraint = vec![
            CourseDataConstraint::Mirror,
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::NoGood,
            CourseDataConstraint::Gauge7Keys,
            CourseDataConstraint::Cn,
        ];
        course.hash = vec![make_song("S1", "h1")];
        view.courses = vec![course];

        view.update_course(Some(0));
        assert!(view.course_pane_visible);
        assert_eq!(view.selected_course, Some(0));
        assert_eq!(view.course_name, "Test Course");
        assert!(view.release);
        assert_eq!(view.grade_type, Some(CourseDataConstraint::Mirror));
        assert_eq!(view.hispeed_type, Some(CourseDataConstraint::NoSpeed));
        assert_eq!(view.judge_type, Some(CourseDataConstraint::NoGood));
        assert_eq!(view.gauge_type, Some(CourseDataConstraint::Gauge7Keys));
        assert_eq!(view.ln_type, Some(CourseDataConstraint::Cn));
        assert_eq!(view.course_songs.len(), 1);
        assert_eq!(view.course_songs[0].metadata.title, "S1");
    }

    #[test]
    fn test_update_course_loads_trophies() {
        let mut view = CourseEditorView::new();
        let mut course = make_course("Trophy Test");
        course.trophy = vec![
            TrophyData::new("bronzemedal".to_string(), 8.0, 50.0),
            TrophyData::new("silvermedal".to_string(), 4.0, 75.0),
            TrophyData::new("goldmedal".to_string(), 1.0, 90.0),
        ];
        view.courses = vec![course];

        view.update_course(Some(0));
        assert_eq!(view.bronzemiss, 8.0);
        assert_eq!(view.bronzescore, 50.0);
        assert_eq!(view.silvermiss, 4.0);
        assert_eq!(view.silverscore, 75.0);
        assert_eq!(view.goldmiss, 1.0);
        assert_eq!(view.goldscore, 90.0);
    }

    #[test]
    fn test_update_course_out_of_bounds_hides_pane() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A")];

        view.update_course(Some(5));
        assert!(!view.course_pane_visible);
    }

    #[test]
    fn test_update_course_resets_constraints() {
        let mut view = CourseEditorView::new();
        // Set some constraints first
        view.grade_type = Some(CourseDataConstraint::Class);
        view.hispeed_type = Some(CourseDataConstraint::NoSpeed);

        // Create course with no constraints
        let course = CourseData {
            name: Some("No Constraints".to_string()),
            ..Default::default()
        };
        view.courses = vec![course];

        view.update_course(Some(0));
        // All constraints should be reset to None
        assert!(view.grade_type.is_none());
        assert!(view.hispeed_type.is_none());
        assert!(view.judge_type.is_none());
        assert!(view.gauge_type.is_none());
        assert!(view.ln_type.is_none());
    }

    // ---- updateCourseData ----

    #[test]
    fn test_update_course_data_commits_and_updates() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("Before")];
        view.selected_course = Some(0);
        view.course_name = "After".to_string();
        view.courses_selected_index = Some(0);

        view.update_course_data();
        // commit_course should have saved "After"
        assert_eq!(view.courses[0].name(), "After");
        // Then update_course should have loaded it back
        assert_eq!(view.course_name, "After");
    }

    #[test]
    fn test_update_course_data_no_selection() {
        let mut view = CourseEditorView::new();
        view.courses = vec![make_course("A")];
        view.courses_selected_index = None;
        view.selected_course = None;

        view.update_course_data();
        assert!(!view.course_pane_visible);
    }

    // ---- searchSongs ----

    #[test]
    fn test_search_songs_no_songdb() {
        let mut view = CourseEditorView::new();
        view.search = "test".to_string();
        view.songdb = None;

        view.search_songs();
        // Should early return without panic
        assert!(view.search_songs.is_empty());
    }

    #[test]
    fn test_search_songs_with_hash() {
        let mut view = CourseEditorView::new();
        view.songdb = Some(Box::new(TestSongDb::new()));
        // Valid md5 hash (32 hex chars)
        view.search = "abcdef1234567890abcdef1234567890".to_string();

        view.search_songs();
        // Stub returns empty, but should not panic
        assert!(view.search_songs.is_empty());
    }

    #[test]
    fn test_search_songs_with_text() {
        let mut view = CourseEditorView::new();
        view.songdb = Some(Box::new(TestSongDb::new()));
        view.search = "test query".to_string();

        view.search_songs();
        // Stub returns empty, but should not panic
        assert!(view.search_songs.is_empty());
    }

    #[test]
    fn test_search_songs_short_text_skipped() {
        let mut view = CourseEditorView::new();
        view.songdb = Some(Box::new(TestSongDb::new()));
        view.search = "a".to_string(); // length <= 1, not a hash

        view.search_songs();
        assert!(view.search_songs.is_empty());
    }

    // ---- getValue ----

    #[test]
    fn test_get_value_passthrough() {
        assert_eq!(CourseEditorView::value(42.0), 42.0);
        assert_eq!(CourseEditorView::value(0.0), 0.0);
        assert_eq!(CourseEditorView::value(-1.5), -1.5);
    }

    // ---- round-trip: add → select → commit → get ----

    #[test]
    fn test_round_trip_add_edit_get() {
        let mut view = CourseEditorView::new();
        view.initialize();

        // Add a course
        view.add_course_data();
        assert_eq!(view.courses.len(), 1);

        // Select and edit
        view.courses_selected_index = Some(0);
        view.update_course(Some(0));
        assert!(view.course_pane_visible);
        assert_eq!(view.course_name, "New Course");

        // Modify
        view.course_name = "Edited Course".to_string();
        view.release = true;
        view.grade_type = Some(CourseDataConstraint::Class);

        // Add a song
        view.search_songs_selected_items = vec![make_song("Test Song", "hashvalue")];
        view.add_song_data();
        assert_eq!(view.course_songs.len(), 1);

        // Get course data (triggers commit)
        let courses = view.course_data();
        assert_eq!(courses.len(), 1);
        assert_eq!(courses[0].name(), "Edited Course");
        assert!(courses[0].release);
        assert!(courses[0].constraint.contains(&CourseDataConstraint::Class));
        assert_eq!(courses[0].hash.len(), 1);
        assert_eq!(courses[0].hash[0].metadata.title, "Test Song");
    }
}
