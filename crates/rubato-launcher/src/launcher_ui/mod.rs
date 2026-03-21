// LauncherUi -- egui-based launcher configuration window
// Java equivalent: PlayConfigurationView (JavaFX Application)

pub(crate) mod tabs;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests;

use bms_model::mode::Mode;
use rubato_core::config::Config;
use rubato_core::player_config::PlayerConfig;

use crate::views::config::obs_configuration_view::ObsConfigurationView;
use crate::views::play_configuration_view::{PlayConfigurationView, PlayMode};
use crate::views::skin_configuration_view::SkinConfigurationView;

/// Tab selection for the launcher UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
enum Tab {
    Video,
    Audio,
    Input,
    MusicSelect,
    Skin,
    Option,
    Other,
    IR,
    Stream,
    Discord,
    OBS,
}

impl Tab {
    fn label(&self) -> &'static str {
        match self {
            Tab::Video => "Video",
            Tab::Audio => "Audio",
            Tab::Input => "Input",
            Tab::MusicSelect => "Music Select",
            Tab::Skin => "Skin",
            Tab::Option => "Option",
            Tab::Other => "Other",
            Tab::IR => "IR",
            Tab::Stream => "Stream",
            Tab::Discord => "Discord",
            Tab::OBS => "OBS",
        }
    }

    fn all() -> &'static [Tab] {
        &[
            Tab::Video,
            Tab::Audio,
            Tab::Input,
            Tab::MusicSelect,
            Tab::Skin,
            Tab::Option,
            Tab::Other,
            Tab::IR,
            Tab::Stream,
            Tab::Discord,
            Tab::OBS,
        ]
    }
}

const IR_SEND_LABELS: [&str; 3] = ["ALWAYS", "COMPLETE SONG", "UPDATE SCORE"];

/// Main launcher UI state.
///
/// Java equivalent: PlayConfigurationView -- manages all configuration sub-views
/// and provides the top-level player selector + action buttons.
pub struct LauncherUi {
    config: Config,
    player: PlayerConfig,
    selected_tab: Tab,
    player_name: String,
    selected_play_mode: usize,
    bms_paths: Vec<String>,
    selected_ir_index: usize,
    /// Decrypted IR userid buffer for egui text editing.
    ir_userid_buf: String,
    /// Decrypted IR password buffer for egui text editing.
    ir_password_buf: String,
    /// Previous IR index to detect slot switches.
    ir_prev_index: Option<usize>,
    /// Skin configuration sub-view (skin type/header selection + custom options).
    skin_view: SkinConfigurationView,
    /// Discord webhook URL list for editing.
    webhook_urls: Vec<String>,
    /// New webhook URL input buffer.
    webhook_url_input: String,
    /// OBS configuration sub-view (connection, scene/action selectors).
    obs_view: ObsConfigurationView,
    /// Whether the "What's New" popup is open.
    show_whats_new: bool,
    /// What's New message text.
    whats_new_text: String,
    /// Chart details dialog state.
    chart_details_open: bool,
    /// Chart details dialog data (label, value) pairs.
    chart_details_data: Vec<(String, String)>,
    /// Set to true when the user clicks "Start" -- signals the caller to launch play.
    /// Java: PlayConfigurationView.start() calls MainLoader.play()
    play_requested: bool,
    /// Set to true when the user clicks "Exit".
    /// Java: PlayConfigurationView.exit() calls commit() + System.exit(0)
    exit_requested: bool,
    /// Set to true when the user clicks "Load All BMS".
    load_all_bms_requested: bool,
    /// Set to true when the user clicks "Load Diff BMS".
    load_diff_bms_requested: bool,
    /// Set to true when the user clicks "Import Score".
    import_score_requested: bool,
    /// Shared flag for play_requested, survives after eframe drops the App.
    /// Used by run_launcher() to detect whether play should be launched.
    shared_play_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
    shared_load_all_bms: std::sync::Arc<std::sync::atomic::AtomicBool>,
    shared_load_diff_bms: std::sync::Arc<std::sync::atomic::AtomicBool>,
    shared_import_score: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl LauncherUi {
    pub fn new(config: Config, player: PlayerConfig) -> Self {
        let player_name = config
            .playername
            .clone()
            .unwrap_or_else(|| "default".to_string());
        // Initialize skin configuration view: scan filesystem + load player config
        let mut skin_view = SkinConfigurationView::new();
        skin_view.initialize();
        skin_view.update_config(&config);
        skin_view.update_player(&player);
        let webhook_urls = config.integration.webhook_url.clone();
        let bms_paths = config.paths.bmsroot.clone();

        // Initialize OBS configuration view
        let mut obs_view = ObsConfigurationView::new();
        obs_view.initialize();
        let dummy_main = PlayConfigurationView::new();
        obs_view.init(&dummy_main);
        obs_view.update(config.clone());

        let has_ir = !player.irconfig.is_empty();
        let mut ui = Self {
            config,
            player,
            selected_tab: Tab::Option,
            player_name,
            selected_play_mode: 1, // BEAT_7K
            bms_paths,
            selected_ir_index: 0,
            ir_userid_buf: String::new(),
            ir_password_buf: String::new(),
            ir_prev_index: None,
            skin_view,
            webhook_urls,
            webhook_url_input: String::new(),
            obs_view,
            show_whats_new: false,
            whats_new_text: String::new(),
            chart_details_open: false,
            chart_details_data: Vec::new(),
            play_requested: false,
            exit_requested: false,
            load_all_bms_requested: false,
            load_diff_bms_requested: false,
            import_score_requested: false,
            shared_play_requested: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            shared_load_all_bms: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            shared_load_diff_bms: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            shared_import_score: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        // Pre-load IR slot 0 buffers so flush_ir_buffers() can save edits
        // even if the user never switches slots (ir_prev_index must be Some).
        if has_ir {
            ui.load_ir_buffers(0);
        }
        ui
    }

    /// Create a LauncherUi with shared flags.
    /// Used by run_launcher() to detect requests after eframe drops the App.
    fn new_with_shared_flags(
        config: Config,
        player: PlayerConfig,
        shared_play_requested: std::sync::Arc<std::sync::atomic::AtomicBool>,
        shared_load_all_bms: std::sync::Arc<std::sync::atomic::AtomicBool>,
        shared_load_diff_bms: std::sync::Arc<std::sync::atomic::AtomicBool>,
        shared_import_score: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        let mut ui = Self::new(config, player);
        ui.shared_play_requested = shared_play_requested;
        ui.shared_load_all_bms = shared_load_all_bms;
        ui.shared_load_diff_bms = shared_load_diff_bms;
        ui.shared_import_score = shared_import_score;
        ui
    }

    /// Returns true if the user has clicked "Start" and play should be launched.
    /// Java: PlayConfigurationView.start() triggers MainLoader.play()
    pub fn is_play_requested(&self) -> bool {
        self.play_requested
    }

    /// Returns true if the user has clicked "Load All BMS".
    pub fn is_load_all_bms_requested(&self) -> bool {
        self.load_all_bms_requested
    }

    /// Returns true if the user has clicked "Load Diff BMS".
    pub fn is_load_diff_bms_requested(&self) -> bool {
        self.load_diff_bms_requested
    }

    /// Returns true if the user has clicked "Import Score".
    pub fn is_import_score_requested(&self) -> bool {
        self.import_score_requested
    }

    /// Returns a clone of the current Config.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a clone of the current PlayerConfig.
    pub fn player(&self) -> &PlayerConfig {
        &self.player
    }

    fn current_mode(&self) -> Mode {
        PlayMode::values()
            .get(self.selected_play_mode)
            .map(|m| m.to_mode())
            .unwrap_or(Mode::BEAT_7K)
    }

    /// Render the launcher configuration UI.
    ///
    /// Java equivalent: PlayConfigurationView.start(Stage primaryStage) builds
    /// the JavaFX scene graph with tabs, combo boxes, and action buttons.
    pub fn render_ui(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Header: player name + play mode selector
            ui.horizontal(|ui| {
                ui.label("Player:");
                ui.text_edit_singleline(&mut self.player_name);

                ui.separator();

                let play_modes = PlayMode::values();
                let selected_text = play_modes
                    .get(self.selected_play_mode)
                    .map(|m| m.display_name())
                    .unwrap_or("7KEYS");
                egui::ComboBox::from_label("Mode")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (i, mode) in play_modes.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.selected_play_mode,
                                i,
                                mode.display_name(),
                            );
                        }
                    });
            });

            ui.separator();

            // Tab bar
            ui.horizontal(|ui| {
                for tab in Tab::all() {
                    if ui
                        .selectable_label(self.selected_tab == *tab, tab.label())
                        .clicked()
                    {
                        self.selected_tab = *tab;
                    }
                }
            });

            ui.separator();

            // Tab content
            egui::ScrollArea::vertical().show(ui, |ui| match self.selected_tab {
                Tab::Video => self.render_video_tab(ui),
                Tab::Audio => self.render_audio_tab(ui),
                Tab::Input => self.render_input_tab(ui),
                Tab::MusicSelect => self.render_music_select_tab(ui),
                Tab::Skin => self.render_skin_tab(ui),
                Tab::Option => self.render_option_tab(ui),
                Tab::Other => self.render_other_tab(ui),
                Tab::IR => self.render_ir_tab(ui),
                Tab::Stream => self.render_stream_tab(ui),
                Tab::Discord => self.render_discord_tab(ui),
                Tab::OBS => self.render_obs_tab(ui),
            });

            ui.separator();

            // Popups
            self.render_popups(ui.ctx());

            // Action buttons at the bottom
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    self.play_requested = true;
                    log::info!("Start requested");
                }
                if ui.button("Load All BMS").clicked() {
                    self.load_all_bms_requested = true;
                    log::info!("Load All BMS requested");
                }
                if ui.button("Load Diff BMS").clicked() {
                    self.load_diff_bms_requested = true;
                    log::info!("Load Diff BMS requested");
                }
                if ui.button("Import Score").clicked() {
                    self.import_score_requested = true;
                    log::info!("Import Score requested");
                }
                if ui.button("Exit").clicked() {
                    self.exit_requested = true;
                }
            });
        });
    }

    /// Render popup windows (What's New, Chart Details).
    fn render_popups(&mut self, ctx: &egui::Context) {
        if self.show_whats_new {
            let mut open = self.show_whats_new;
            egui::Window::new("What's New")
                .open(&mut open)
                .resizable(true)
                .default_width(400.0)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.label(&self.whats_new_text);
                    });
                    if ui.button("OK").clicked() {
                        self.show_whats_new = false;
                    }
                });
            self.show_whats_new = open;
        }

        if self.chart_details_open {
            let mut open = self.chart_details_open;
            egui::Window::new("Chart Details")
                .open(&mut open)
                .resizable(true)
                .default_width(500.0)
                .show(ctx, |ui| {
                    egui::Grid::new("chart_details_grid").show(ui, |ui| {
                        for (label, value) in &self.chart_details_data {
                            ui.label(label);
                            let mut val = value.clone();
                            ui.add(egui::TextEdit::singleline(&mut val));
                            ui.end_row();
                        }
                    });
                    if ui.button("OK").clicked() {
                        self.chart_details_open = false;
                    }
                });
            self.chart_details_open = open;
        }
    }

    /// Show the What's New popup with the given text.
    pub fn show_whats_new_popup(&mut self, text: String) {
        self.whats_new_text = text;
        self.show_whats_new = true;
    }

    /// Show the chart details dialog with the given data.
    pub fn show_chart_details(&mut self, data: Vec<(String, String)>) {
        self.chart_details_data = data;
        self.chart_details_open = true;
    }

    fn commit_config(&mut self) {
        self.config.playername = Some(self.player_name.clone());
        // Sync player.id so PlayerConfig::write() saves to the correct profile directory
        self.player.id = Some(self.player_name.clone());
        // Commit BMS root paths
        self.config.paths.bmsroot = self.bms_paths.clone();
        // Commit webhook URLs
        self.config.integration.webhook_url = self.webhook_urls.clone();
        // Commit OBS configuration (scene/action selections, connection settings)
        self.obs_view.commit();
        if let Some(obs_config) = self.obs_view.config() {
            self.config.obs = obs_config.obs.clone();
        }
        // Flush IR userid/password buffers (triggers AES encryption)
        self.flush_ir_buffers();
        // Commit skin configuration (saves to player.skin + skin_history)
        self.skin_view.commit();
        if let Some(updated_player) = self.skin_view.player() {
            self.player.skin = updated_player.skin.clone();
            self.player.skin_history = updated_player.skin_history.clone();
        }
        if let Err(e) = Config::write(&self.config) {
            log::error!("Failed to save config: {}", e);
        }
        if let Err(e) = PlayerConfig::write(&self.config.paths.playerpath, &self.player) {
            log::error!("Failed to save player config: {}", e);
        }
    }
}

/// eframe::App implementation for LauncherUi.
///
/// Java equivalent: JavaFX Application.start(Stage) -> PlayConfigurationView scene rendering.
/// In Java, the JavaFX framework calls into the scene graph each frame.
/// In Rust, eframe calls update() each frame, which delegates to render_ui().
impl eframe::App for LauncherUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_ui(ctx);

        // Java: PlayConfigurationView.exit() calls commit() + System.exit(0)
        // In eframe, we close the viewport instead.
        if self.exit_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // Java: PlayConfigurationView.start() triggers MainLoader.play()
        // The action flags are checked by the caller after run_native() returns.
        // When using eframe, we close the launcher window so the action can begin.
        if self.play_requested
            || self.load_all_bms_requested
            || self.load_diff_bms_requested
            || self.import_score_requested
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Java: PlayConfigurationView.exit() calls commit() before closing.
    /// eframe calls on_exit() when the window is being closed.
    fn on_exit(&mut self) {
        self.commit_config();
        // Persist flags to shared atomics so run_launcher() can read them.
        self.shared_play_requested
            .store(self.play_requested, std::sync::atomic::Ordering::Release);
        self.shared_load_all_bms.store(
            self.load_all_bms_requested,
            std::sync::atomic::Ordering::Release,
        );
        self.shared_load_diff_bms.store(
            self.load_diff_bms_requested,
            std::sync::atomic::Ordering::Release,
        );
        self.shared_import_score.store(
            self.import_score_requested,
            std::sync::atomic::Ordering::Release,
        );
    }
}

/// Result of running the launcher UI.
///
/// After the eframe window closes, this struct holds the final Config/PlayerConfig
/// (re-read from disk after commit_config saved them) and whether "Start" was clicked.
pub struct LauncherResult {
    pub config: Config,
    pub player: PlayerConfig,
    pub play_requested: bool,
    pub load_all_bms_requested: bool,
    pub load_diff_bms_requested: bool,
    pub import_score_requested: bool,
}

/// Launch the egui configuration window using eframe.
///
/// Java equivalent: MainLoader.start(Stage) -> creates JavaFX Stage with PlayConfigurationView.
/// In Rust, this creates an eframe window with LauncherUi.
///
/// Returns LauncherResult after the window is closed, so the caller
/// can check play_requested and retrieve config/player for play().
pub fn run_launcher(
    config: Config,
    player: PlayerConfig,
    title: &str,
) -> anyhow::Result<LauncherResult> {
    // Shared atomic flags: survive after eframe drops the App.
    let shared_play_requested = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shared_load_all_bms = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shared_load_diff_bms = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let shared_import_score = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    let launcher = LauncherUi::new_with_shared_flags(
        config,
        player,
        shared_play_requested.clone(),
        shared_load_all_bms.clone(),
        shared_load_diff_bms.clone(),
        shared_import_score.clone(),
    );

    // Java: primaryStage.setScene(scene); primaryStage.show();
    // eframe::run_native() blocks until the window is closed.
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([1000.0, 700.0]),
        ..Default::default()
    };

    eframe::run_native(
        title,
        native_options,
        Box::new(move |_cc| Ok(Box::new(launcher))),
    )
    .map_err(|e| anyhow::anyhow!("eframe::run_native failed: {}", e))?;

    // After run_native returns, the App has been dropped (on_exit saved state).
    let play_requested = shared_play_requested.load(std::sync::atomic::Ordering::Acquire);

    // Re-read config/player from disk (commit_config saved them in on_exit).
    let config = Config::read().unwrap_or_default();
    let playerpath = &config.paths.playerpath;
    let playername = config.playername.as_deref().unwrap_or("default");
    let player = PlayerConfig::read_player_config(playerpath, playername)
        .unwrap_or_else(|_| PlayerConfig::default());

    let load_all_bms_requested = shared_load_all_bms.load(std::sync::atomic::Ordering::Acquire);
    let load_diff_bms_requested = shared_load_diff_bms.load(std::sync::atomic::Ordering::Acquire);
    let import_score_requested = shared_import_score.load(std::sync::atomic::Ordering::Acquire);

    Ok(LauncherResult {
        config,
        player,
        play_requested,
        load_all_bms_requested,
        load_diff_bms_requested,
        import_score_requested,
    })
}
