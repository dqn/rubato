// Translates: bms.player.beatoraja.launcher.MusicSelectConfigurationView

use beatoraja_core::config::{Config, SongPreview};
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_select::music_selector::ChartReplicationMode;

/// Translates: MusicSelectConfigurationView (JavaFX → egui)
///
/// Song select configuration UI: scroll durations, analog scroll,
/// folder lamp, song info, preview, random select, chart replication.
#[allow(dead_code)]
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

impl Default for MusicSelectConfigurationView {
    fn default() -> Self {
        MusicSelectConfigurationView {
            scrolldurationlow: 0,
            scrolldurationhigh: 0,
            analog_scroll: false,
            analog_ticks_per_scroll: 0,
            folderlamp: false,
            use_song_info: false,
            shownoexistingbar: false,
            song_preview: None,
            randomselect: false,
            maxsearchbar: 0,
            chart_replication_mode: None,
            chart_replication_mode_items: Vec::new(),
            skip_decide_screen: false,
            config: None,
            player: None,
        }
    }
}

#[allow(dead_code)]
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
}
