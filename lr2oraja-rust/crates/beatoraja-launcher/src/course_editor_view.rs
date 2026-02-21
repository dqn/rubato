// Translated from CourseEditorView.java

use beatoraja_core::course_data::{CourseData, CourseDataConstraint, TrophyData};
use beatoraja_core::main_controller::SongDatabaseAccessor;
use beatoraja_core::stubs::SongData;

use crate::folder_editor_view::SongDataView;
use crate::table_editor_view::TableEditorView;

/// CourseEditorView - course editor with constraints, trophies, song search
///
/// JavaFX UI widgets are translated to data structs.
/// All rendering/UI operations use todo!("egui integration").
#[allow(dead_code)]
pub struct CourseEditorView {
    // JavaFX @FXML fields → egui widget state
    search: String,
    search_songs: Vec<SongData>,
    search_songs_controller: SongDataView,
    search_songs_selected_items: Vec<SongData>,

    courses: Vec<CourseData>,
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

    filename: String,

    selected_course: Option<usize>, // index into courses

    songdb: Option<SongDatabaseAccessor>,
}

#[allow(dead_code)]
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

            filename: String::new(),

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
    pub fn set_song_database_accessor(&mut self, songdb: SongDatabaseAccessor) {
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
            self.search_songs = Vec::new(); // stub — SongDatabaseAccessor methods not yet implemented
        } else if self.search.len() > 1 {
            // searchSongs.getItems().setAll(songdb.getSongDatasByText(search.getText()));
            let _songdb = self.songdb.as_ref().unwrap();
            self.search_songs = Vec::new(); // stub
        }
    }

    /// getCourseData - commits and returns all courses
    pub fn get_course_data(&mut self) -> Vec<CourseData> {
        self.commit_course();
        self.courses.clone()
    }

    /// setCourseData - sets the course list
    pub fn set_course_data(&mut self, course: Vec<CourseData>) {
        self.courses = course;
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
    fn get_value(value: f64) -> f64 {
        // In Java:
        // spinner.getValueFactory().setValue(
        //     spinner.getValueFactory().getConverter().fromString(spinner.getEditor().getText())
        // );
        // return spinner.getValue();
        value
    }

    /// addCourseData - adds a new course with default values
    pub fn add_course_data(&mut self) {
        let mut course = CourseData::default();
        course.name = Some("New Course".to_string());
        course.release = false;
        // CourseData.TrophyData[] trophy = new CourseData.TrophyData[3];
        // trophy[0] = new CourseData.TrophyData("bronzemedal", 7.5f, 55.0f);
        // trophy[1] = new CourseData.TrophyData("silvermedal", 5.0f, 70.0f);
        // trophy[2] = new CourseData.TrophyData("goldmedal", 2.5f, 85.0f);
        course.trophy = vec![
            TrophyData::new("bronzemedal".to_string(), 7.5, 55.0),
            TrophyData::new("silvermedal".to_string(), 5.0, 70.0),
            TrophyData::new("goldmedal".to_string(), 2.5, 85.0),
        ];
        self.courses.push(course);
    }

    /// removeCourseData - removes the currently selected course
    pub fn remove_course_data(&mut self) {
        if let Some(idx) = self.courses_selected_index {
            if idx < self.courses.len() {
                self.courses.remove(idx);
            }
        }
    }

    /// moveCourseDataUp - moves the selected course up one position
    pub fn move_course_data_up(&mut self) {
        if let Some(index) = self.courses_selected_index {
            if index > 0 {
                self.courses.swap(index, index - 1);
                self.courses_selected_index = Some(index - 1);
            }
        }
    }

    /// moveCourseDataDown - moves the selected course down one position
    pub fn move_course_data_down(&mut self) {
        if let Some(index) = self.courses_selected_index {
            if index < self.courses.len().saturating_sub(1) {
                self.courses.swap(index, index + 1);
                self.courses_selected_index = Some(index + 1);
            }
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
        if let Some(idx) = self.course_songs_selected_index {
            if idx < self.course_songs.len() {
                self.course_songs.remove(idx);
            }
        }
    }

    /// moveSongDataUp - moves the selected song up one position
    pub fn move_song_data_up(&mut self) {
        if let Some(index) = self.course_songs_selected_index {
            if index > 0 {
                self.course_songs.swap(index, index - 1);
                self.course_songs_selected_index = Some(index - 1);
            }
        }
    }

    /// moveSongDataDown - moves the selected song down one position
    pub fn move_song_data_down(&mut self) {
        if let Some(index) = self.course_songs_selected_index {
            if index < self.course_songs.len().saturating_sub(1) {
                self.course_songs.swap(index, index + 1);
                self.course_songs_selected_index = Some(index + 1);
            }
        }
    }
}

impl Default for CourseEditorView {
    fn default() -> Self {
        Self::new()
    }
}
