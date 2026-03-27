use super::*;

impl PlayConfigurationView {
    pub fn new() -> Self {
        PlayConfigurationView {
            newversion_text: String::new(),
            newversion_url: None,
            pending_version_check: None,
            players: Vec::new(),
            players_selected: None,
            playername: String::new(),
            playconfig: None,
            hispeed: 1.0,
            lr2configuration_hgap: 25.0,
            lr2configuration_vgap: 4.0,
            lr2configurationassist_hgap: 25.0,
            lr2configurationassist_vgap: 4.0,
            fixhispeed: None,
            gvalue: 500,
            enable_constant: false,
            const_fadein_time: 100,
            hispeedmargin: 0.25,
            hispeedautoadjust: false,
            scoreop: None,
            scoreop2: None,
            doubleop: None,
            gaugeop: None,
            lntype: None,
            enable_lanecover: true,
            lanecover: 200,
            lanecovermarginlow: 1,
            lanecovermarginhigh: 10,
            lanecoverswitchduration: 500,
            enable_lift: false,
            lift: 100,
            enable_hidden: false,
            hidden: 100,
            bgmpath: String::new(),
            soundpath: String::new(),
            notesdisplaytiming: 0,
            notesdisplaytimingautoadjust: false,
            bpmguide: false,
            gaugeautoshift: None,
            bottomshiftablegauge: None,
            customjudge: false,
            njudgepg: 400,
            njudgegr: 400,
            njudgegd: 100,
            sjudgepg: 400,
            sjudgegr: 400,
            sjudgegd: 100,
            minemode: None,
            scrollmode: None,
            longnotemode: None,
            forcedcnendings: false,
            longnoterate: 1.0,
            hranthresholdbpm: 120,
            seventoninepattern: None,
            seventoninetype: None,
            exitpressduration: 1000,
            chartpreview: true,
            guidese: false,
            windowhold: false,
            extranotedepth: 0,
            judgeregion: false,
            markprocessednote: false,
            showhiddennote: false,
            showpastnote: false,
            target: Vec::new(),
            target_selected: None,
            judgealgorithm: None,
            autosavereplay1: None,
            autosavereplay2: None,
            autosavereplay3: None,
            autosavereplay4: None,
            usecim: false,
            enable_ipfs: false,
            ipfsurl: String::new(),
            enable_http: false,
            http_download_source: Vec::new(),
            http_download_source_selected: None,
            default_download_url: String::new(),
            override_download_url: String::new(),
            clipboard_screenshot: false,
            score_options_labels: Vec::new(),
            double_options_labels: Vec::new(),
            seven_to_nine_pattern_labels: Vec::new(),
            seven_to_nine_type_labels: Vec::new(),
            gauge_options_labels: Vec::new(),
            fixhispeed_labels: Vec::new(),
            lntype_labels: Vec::new(),
            gaugeautoshift_labels: Vec::new(),
            bottomshiftablegauge_labels: Vec::new(),
            minemode_labels: Vec::new(),
            scrollmode_labels: Vec::new(),
            longnotemode_labels: Vec::new(),
            judgealgorithm_labels: Vec::new(),
            autosave_labels: Vec::new(),
            video_controller: VideoConfigurationView::default(),
            audio_controller: AudioConfigurationView::default(),
            input_controller: InputConfigurationView::default(),
            resource_controller: ResourceConfigurationView::new(),
            music_select_controller: MusicSelectConfigurationView::default(),
            skin_controller: SkinConfigurationView::new(),
            ir_controller: IRConfigurationView::default(),
            table_controller: TableEditorView::new(),
            stream_controller: StreamEditorView::default(),
            discord_controller: DiscordConfigurationView::default(),
            obs_controller: ObsConfigurationView::new(),
            trainer_controller: TrainerView::default(),
            config: None,
            player: None,
            loader: None,
            song_updated: false,
            pc: None,
            exit_requested: false,
            bms_loading_handle: None,
            bms_loading_result: None,
            lr2_import_handle: None,
            player_panel_disabled: false,
            video_tab_disabled: false,
            audio_tab_disabled: false,
            input_tab_disabled: false,
            resource_tab_disabled: false,
            option_tab_disabled: false,
            other_tab_disabled: false,
            ir_tab_disabled: false,
            stream_tab_disabled: false,
            discord_tab_disabled: false,
            obs_tab_disabled: false,
            control_panel_disabled: false,
        }
    }

    /// Initialize combo box labels (static helper)
    /// Translates: static void initComboBox(ComboBox<Integer> combo, final String[] values)
    pub fn init_combo_box_labels(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    /// Initialize the view
    /// Translates: public void initialize(URL arg0, ResourceBundle arg1)
    pub fn initialize(&mut self) {
        let t = std::time::Instant::now();
        self.lr2configuration_hgap = 25.0;
        self.lr2configuration_vgap = 4.0;
        self.lr2configurationassist_hgap = 25.0;
        self.lr2configurationassist_vgap = 4.0;

        let score_options = vec![
            "OFF",
            "MIRROR",
            "RANDOM",
            "R-RANDOM",
            "S-RANDOM",
            "SPIRAL",
            "H-RANDOM",
            "ALL-SCR",
            "RANDOM-EX",
            "S-RANDOM-EX",
        ];
        self.score_options_labels = Self::init_combo_box_labels(&score_options);

        self.double_options_labels =
            Self::init_combo_box_labels(&["OFF", "FLIP", "BATTLE", "BATTLE AS"]);

        self.seven_to_nine_pattern_labels = Self::init_combo_box_labels(&[
            "OFF",
            "SC1KEY2~8",
            "SC1KEY3~9",
            "SC2KEY3~9",
            "SC8KEY1~7",
            "SC9KEY1~7",
            "SC9KEY2~8",
        ]);

        // These would normally come from resource bundle, using defaults
        self.seven_to_nine_type_labels =
            Self::init_combo_box_labels(&["OFF", "NO MASHING", "ALTERNATION"]);

        self.gauge_options_labels = Self::init_combo_box_labels(&[
            "ASSIST EASY",
            "EASY",
            "NORMAL",
            "HARD",
            "EX-HARD",
            "HAZARD",
        ]);

        self.fixhispeed_labels =
            Self::init_combo_box_labels(&["OFF", "START BPM", "MAX BPM", "MAIN BPM", "MIN BPM"]);

        self.lntype_labels =
            Self::init_combo_box_labels(&["LONG NOTE", "CHARGE NOTE", "HELL CHARGE NOTE"]);

        self.gaugeautoshift_labels = Self::init_combo_box_labels(&[
            "NONE",
            "CONTINUE",
            "SURVIVAL TO GROOVE",
            "BEST CLEAR",
            "SELECT TO UNDER",
        ]);

        self.bottomshiftablegauge_labels =
            Self::init_combo_box_labels(&["ASSIST EASY", "EASY", "NORMAL"]);

        self.minemode_labels =
            Self::init_combo_box_labels(&["OFF", "REMOVE", "ADD RANDOM", "ADD NEAR", "ADD ALL"]);

        self.scrollmode_labels = Self::init_combo_box_labels(&["OFF", "REMOVE", "ADD"]);

        self.longnotemode_labels = Self::init_combo_box_labels(&[
            "OFF", "REMOVE", "ADD LN", "ADD CN", "ADD HCN", "ADD ALL",
        ]);

        // These would normally come from resource bundle
        self.judgealgorithm_labels = Self::init_combo_box_labels(&["LR2", "AC", "BOTTOM PRIORITY"]);

        self.autosave_labels = Self::init_combo_box_labels(&[
            "NONE",
            "BETTER SCORE",
            "BETTER OR SAME SCORE",
            "BETTER MISSCOUNT",
            "BETTER OR SAME MISSCOUNT",
            "BETTER COMBO",
            "BETTER OR SAME COMBO",
            "BETTER LAMP",
            "BETTER OR SAME LAMP",
            "BETTER ALL",
            "ALWAYS",
        ]);

        self.http_download_source = DOWNLOAD_SOURCES.keys().cloned().collect();

        // Sub-controller init calls: these methods set up internal state
        // (table columns, combo box items, etc.) — actual rendering is egui.
        // We pass `self` as a dummy since the parameter is unused in Rust.
        // NOTE: We cannot pass `&self` here because `self` is `&mut`.
        // The init methods use `_main` (unused), so we pass a default-constructed instance.
        let dummy = PlayConfigurationView::new();
        self.resource_controller.init(&dummy);
        self.discord_controller.init();
        self.obs_controller.init(&dummy);

        self.check_new_version();
        let elapsed = t.elapsed().as_millis();
        info!("Initialization time (ms): {}", elapsed);
    }
}
