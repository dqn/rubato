// Translates: bms.player.beatoraja.launcher.MusicSelectConfigurationView

use rubato_core::config::{Config, SongPreview};
use rubato_core::player_config::PlayerConfig;
use rubato_state::select::music_selector::ChartReplicationMode;

/// Translates: MusicSelectConfigurationView (JavaFX → egui)
///
/// Song select configuration UI: scroll durations, analog scroll,
/// folder lamp, song info, preview, random select, chart replication.
#[derive(Default)]
pub struct MusicSelectConfigurationView {
    // @FXML private NumericSpinner<Integer> scrolldurationlow;
    scrolldurationlow: i32,
    // @FXML private NumericSpinner<Integer> scrolldurationhigh;
    scrolldurationhigh: i32,

    // @FXML private CheckBox analogScroll;
    analog_scroll: bool,
    // @FXML private NumericSpinner<Integer> analogTicksPerScroll;
    analog_ticks_per_scroll: i32,

    // @FXML private CheckBox folderlamp;
    folderlamp: bool,
    // @FXML private CheckBox useSongInfo;
    use_song_info: bool,
    // @FXML private CheckBox shownoexistingbar;
    shownoexistingbar: bool,
    // @FXML private ComboBox<SongPreview> songPreview;
    song_preview: Option<SongPreview>,
    // @FXML private CheckBox randomselect;
    randomselect: bool,
    // @FXML private NumericSpinner<Integer> maxsearchbar;
    maxsearchbar: i32,

    // @FXML private ComboBox<String> chartReplicationMode;
    chart_replication_mode: Option<String>,
    chart_replication_mode_items: Vec<String>,

    // @FXML private CheckBox skipDecideScreen;
    skip_decide_screen: bool,

    // private Config config;
    config: Option<Config>,
    // private PlayerConfig player;
    player: Option<PlayerConfig>,
}

impl MusicSelectConfigurationView {
    // public void initialize(URL arg0, ResourceBundle arg1)
    pub fn initialize(&mut self) {
        // songPreview.getItems().setAll(SongPreview.values());
        // (SongPreview variants: NONE, ONCE, LOOP)

        // chartReplicationMode.getItems().setAll(Stream.of(ChartReplicationMode.allMode).map(ChartReplicationMode::name).toList());
        self.chart_replication_mode_items = ChartReplicationMode::ALL_MODE
            .iter()
            .map(|m| m.name().to_string())
            .collect();
    }

    // public void update(Config config)
    pub fn update(&mut self, config: &Config) {
        self.config = Some(config.clone());

        // scrolldurationlow.getValueFactory().setValue(config.getScrollDurationLow());
        self.scrolldurationlow = config.scrolldurationlow;
        // scrolldurationhigh.getValueFactory().setValue(config.getScrollDurationHigh());
        self.scrolldurationhigh = config.scrolldurationhigh;

        // analogScroll.setSelected(config.isAnalogScroll());
        self.analog_scroll = config.analog_scroll;
        // analogTicksPerScroll.getValueFactory().setValue(config.getAnalogTicksPerScroll());
        self.analog_ticks_per_scroll = config.analog_ticks_per_scroll;

        // useSongInfo.setSelected(config.isUseSongInfo());
        self.use_song_info = config.use_song_info;
        // folderlamp.setSelected(config.isFolderlamp());
        self.folderlamp = config.folderlamp;
        // shownoexistingbar.setSelected(config.isShowNoSongExistingBar());
        self.shownoexistingbar = config.show_no_song_existing_bar;
        // songPreview.setValue(config.getSongPreview());
        self.song_preview = Some(config.song_preview.clone());

        // maxsearchbar.getValueFactory().setValue(config.getMaxSearchBarCount());
        self.maxsearchbar = config.max_search_bar_count;
        // skipDecideScreen.setSelected(config.isSkipDecideScreen());
        self.skip_decide_screen = config.skip_decide_screen;
    }

    // public void commit()
    pub fn commit(&mut self) {
        if let Some(ref mut config) = self.config {
            // config.setScrollDutationLow(scrolldurationlow.getValue());
            config.scrolldurationlow = self.scrolldurationlow;
            // config.setScrollDutationHigh(scrolldurationhigh.getValue());
            config.scrolldurationhigh = self.scrolldurationhigh;

            // config.setAnalogScroll(analogScroll.isSelected());
            config.analog_scroll = self.analog_scroll;
            // config.setAnalogTicksPerScroll(analogTicksPerScroll.getValue());
            config.analog_ticks_per_scroll = self.analog_ticks_per_scroll;

            // config.setUseSongInfo(useSongInfo.isSelected());
            config.use_song_info = self.use_song_info;
            // config.setFolderlamp(folderlamp.isSelected());
            config.folderlamp = self.folderlamp;
            // config.setShowNoSongExistingBar(shownoexistingbar.isSelected());
            config.show_no_song_existing_bar = self.shownoexistingbar;
            // config.setSongPreview(songPreview.getValue());
            if let Some(ref sp) = self.song_preview {
                config.song_preview = sp.clone();
            }

            // config.setMaxSearchBarCount(maxsearchbar.getValue());
            config.max_search_bar_count = self.maxsearchbar;
            // config.setSkipDecideScreen(skipDecideScreen.isSelected());
            config.skip_decide_screen = self.skip_decide_screen;
        }
    }

    // public void updatePlayer(PlayerConfig player)
    pub fn update_player(&mut self, player: &PlayerConfig) {
        // this.player = player;
        self.player = Some(player.clone());
        // if(player == null) { return; }
        // (In Rust, Option is used)

        // randomselect.setSelected(player.isRandomSelect());
        self.randomselect = player.is_random_select;

        // chartReplicationMode.setValue(player.getChartReplicationMode());
        self.chart_replication_mode = Some(player.chart_replication_mode.clone());
    }

    // public void commitPlayer()
    pub fn commit_player(&mut self) {
        // if(player == null) { return; }
        if let Some(ref mut player) = self.player {
            // player.setRandomSelect(randomselect.isSelected());
            player.is_random_select = self.randomselect;

            // player.setChartReplicationMode(chartReplicationMode.getValue());
            if let Some(ref mode) = self.chart_replication_mode {
                player.chart_replication_mode = mode.clone();
            }
        }
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scroll");
        egui::Grid::new("music_select_scroll_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Scroll Duration Low:");
                ui.add(egui::DragValue::new(&mut self.scrolldurationlow).range(0..=10000));
                ui.end_row();

                ui.label("Scroll Duration High:");
                ui.add(egui::DragValue::new(&mut self.scrolldurationhigh).range(0..=10000));
                ui.end_row();

                ui.label("Analog Scroll:");
                ui.checkbox(&mut self.analog_scroll, "");
                ui.end_row();

                if self.analog_scroll {
                    ui.label("Analog Ticks Per Scroll:");
                    ui.add(egui::DragValue::new(&mut self.analog_ticks_per_scroll).range(1..=100));
                    ui.end_row();
                }
            });

        ui.separator();
        ui.heading("Display");
        egui::Grid::new("music_select_display_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Folder Lamp:");
                ui.checkbox(&mut self.folderlamp, "");
                ui.end_row();

                ui.label("Use Song Info:");
                ui.checkbox(&mut self.use_song_info, "");
                ui.end_row();

                ui.label("Show Non-Existing Bar:");
                ui.checkbox(&mut self.shownoexistingbar, "");
                ui.end_row();

                ui.label("Song Preview:");
                let sp_label = self
                    .song_preview
                    .as_ref()
                    .map(|sp| format!("{:?}", sp))
                    .unwrap_or_default();
                egui::ComboBox::from_id_salt("music_select_song_preview")
                    .selected_text(&sp_label)
                    .show_ui(ui, |ui| {
                        let previews = [SongPreview::NONE, SongPreview::ONCE, SongPreview::LOOP];
                        for preview in &previews {
                            let label = format!("{:?}", preview);
                            let selected = sp_label == label;
                            if ui.selectable_label(selected, &label).clicked() {
                                self.song_preview = Some(preview.clone());
                            }
                        }
                    });
                ui.end_row();

                ui.label("Skip Decide Screen:");
                ui.checkbox(&mut self.skip_decide_screen, "");
                ui.end_row();
            });

        ui.separator();
        ui.heading("Search / Misc");
        egui::Grid::new("music_select_misc_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Max Search Bar Count:");
                ui.add(egui::DragValue::new(&mut self.maxsearchbar).range(0..=1000));
                ui.end_row();

                ui.label("Random Select:");
                ui.checkbox(&mut self.randomselect, "");
                ui.end_row();

                ui.label("Chart Replication Mode:");
                let crm_label = self.chart_replication_mode.clone().unwrap_or_default();
                egui::ComboBox::from_id_salt("music_select_chart_replication")
                    .selected_text(&crm_label)
                    .show_ui(ui, |ui| {
                        for mode in &self.chart_replication_mode_items.clone() {
                            ui.selectable_value(
                                &mut self.chart_replication_mode,
                                Some(mode.clone()),
                                mode,
                            );
                        }
                    });
                ui.end_row();
            });
    }
}
