use super::download_task_menu::DownloadTaskMenu;
use super::freq_trainer_menu::FreqTrainerMenu;
use super::imgui_notify::ImGuiNotify;
use super::judge_trainer_menu::JudgeTrainerMenu;
use super::misc_setting_menu::MiscSettingMenu;
use super::performance_monitor::PerformanceMonitor;
use super::random_trainer_menu::RandomTrainerMenu;
use super::skin_menu::SkinMenu;
use super::skin_widget_manager::SkinWidgetManager;
use super::{Version, version};

use rubato_types::sync_utils::lock_or_recover;
use std::sync::Mutex;

static WINDOW_WIDTH: Mutex<i32> = Mutex::new(0);
static WINDOW_HEIGHT: Mutex<i32> = Mutex::new(0);

static SHOW_MOD_MENU: Mutex<bool> = Mutex::new(false);
static SHOW_RANDOM_TRAINER: Mutex<bool> = Mutex::new(false);
static SHOW_FREQ_PLUS: Mutex<bool> = Mutex::new(false);
static SHOW_JUDGE_TRAINER: Mutex<bool> = Mutex::new(false);
static SHOW_SONG_MANAGER: Mutex<bool> = Mutex::new(false);
static SHOW_DOWNLOAD_MENU: Mutex<bool> = Mutex::new(false);
static SHOW_SKIN_WIDGET_MANAGER: Mutex<bool> = Mutex::new(false);
static SHOW_PERFORMANCE_MONITOR: Mutex<bool> = Mutex::new(false);
static SHOW_SKIN_MENU: Mutex<bool> = Mutex::new(false);
static SHOW_MISC_SETTING: Mutex<bool> = Mutex::new(false);

pub fn window_width() -> i32 {
    *lock_or_recover(&WINDOW_WIDTH)
}

pub fn window_height() -> i32 {
    *lock_or_recover(&WINDOW_HEIGHT)
}

pub struct ImGuiRenderer;

impl ImGuiRenderer {
    pub fn init(width: i32, height: i32) {
        *lock_or_recover(&WINDOW_WIDTH) = width;
        *lock_or_recover(&WINDOW_HEIGHT) = height;
        // egui context is initialized in beatoraja-bin; nothing to do here.
    }

    pub fn start() {
        // egui frame is managed by beatoraja-bin via egui_winit::State::take_egui_input()
    }

    /// Render mod menu overlay using egui.
    ///
    /// Java equivalent: ImGuiRenderer.render() — called between ImGui.newFrame() and ImGui.render().
    /// Called from beatoraja-bin's event loop within egui::Context::run().
    pub fn render_ui(ctx: &egui::Context) {
        let show_mod_menu = *lock_or_recover(&SHOW_MOD_MENU);
        if show_mod_menu {
            // Window positioning: 44% from left, 2% from top
            // Java: ImGui.setNextWindowPos(windowWidth * 0.44f, windowHeight * 0.02f, ImGuiCond.Once)
            let rel_x = window_width() as f32 * 0.44;
            let rel_y = window_height() as f32 * 0.02;

            let mut show = true;
            egui::Window::new("Endless Dream")
                .open(&mut show)
                .default_pos(egui::pos2(rel_x, rel_y))
                .auto_sized()
                .show(ctx, |ui| {
                    // Sub-window toggle checkboxes
                    let mut freq = lock_or_recover(&SHOW_FREQ_PLUS);
                    ui.checkbox(&mut freq, "Show Rate Modifier Window");
                    drop(freq);

                    let mut random = lock_or_recover(&SHOW_RANDOM_TRAINER);
                    ui.checkbox(&mut random, "Show Random Trainer Window");
                    drop(random);

                    let mut judge = lock_or_recover(&SHOW_JUDGE_TRAINER);
                    ui.checkbox(&mut judge, "Show Judge Trainer Window");
                    drop(judge);

                    {
                        let mut skin = lock_or_recover(&SHOW_SKIN_MENU);
                        let old = *skin;
                        ui.checkbox(&mut skin, "Show Skin Configuration Window");
                        if *skin && !old {
                            SkinMenu::invalidate();
                        }
                    }

                    let mut swm = lock_or_recover(&SHOW_SKIN_WIDGET_MANAGER);
                    ui.checkbox(&mut swm, "Show Skin Widget Manager Window");
                    drop(swm);

                    let mut song = lock_or_recover(&SHOW_SONG_MANAGER);
                    ui.checkbox(&mut song, "Show Song Manager Window");
                    drop(song);

                    let mut dl = lock_or_recover(&SHOW_DOWNLOAD_MENU);
                    ui.checkbox(&mut dl, "Show Download Tasks Window");
                    drop(dl);

                    {
                        let mut perf = lock_or_recover(&SHOW_PERFORMANCE_MONITOR);
                        let old = *perf;
                        ui.checkbox(&mut perf, "Show Performance Monitor Window");
                        if *perf && !old {
                            PerformanceMonitor::reload_event_tree();
                        }
                    }

                    let mut misc = lock_or_recover(&SHOW_MISC_SETTING);
                    ui.checkbox(&mut misc, "Show Misc Setting Window");
                    drop(misc);

                    // Debug information
                    ui.collapsing("Endless Dream Debug Information", |ui| {
                        let commit_hash = Version::git_commit_hash().unwrap_or("unknown");
                        let build_time = version::build_date().unwrap_or("unknown");
                        ui.label(format!("Commit hash: {}", commit_hash));
                        ui.label(format!("Build time: {}", build_time));
                    });
                });
            if !show {
                *lock_or_recover(&SHOW_MOD_MENU) = false;
            }

            // Render sub-windows
            if *lock_or_recover(&SHOW_FREQ_PLUS) {
                FreqTrainerMenu::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_RANDOM_TRAINER) {
                RandomTrainerMenu::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_JUDGE_TRAINER) {
                JudgeTrainerMenu::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_SONG_MANAGER) {
                crate::modmenu::song_manager_menu::SongManagerMenu::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_DOWNLOAD_MENU) {
                DownloadTaskMenu::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_SKIN_WIDGET_MANAGER) {
                SkinWidgetManager::set_focus(true);
                SkinWidgetManager::show_ui(ctx);
            } else {
                SkinWidgetManager::set_focus(false);
            }
            if *lock_or_recover(&SHOW_PERFORMANCE_MONITOR) {
                PerformanceMonitor::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_SKIN_MENU) {
                SkinMenu::show_ui(ctx);
            }
            if *lock_or_recover(&SHOW_MISC_SETTING) {
                MiscSettingMenu::show_ui(ctx);
            }
        }

        // Render toast notifications overlay
        ImGuiNotify::render_notifications_ui(ctx);
    }

    /// Legacy render method — retained for backward compatibility with MainController stub calls.
    /// Actual rendering is now done via render_ui() called from beatoraja-bin.
    pub fn render() {}

    pub fn end() {
        // egui rendering is handled by beatoraja-bin via EguiIntegration::render()
    }

    pub fn dispose() {
        // egui context cleanup is handled by beatoraja-bin
    }

    pub fn get_show_mod_menu() -> bool {
        *lock_or_recover(&SHOW_MOD_MENU)
    }

    pub fn toggle_menu() {
        let mut menu = lock_or_recover(&SHOW_MOD_MENU);
        *menu = !*menu;
    }

    /// Show a "(?)" tooltip when hovering.
    ///
    /// Java: ImGui.textDisabled("(?)") + isItemHovered() → tooltip
    pub fn help_marker(ui: &mut egui::Ui, desc: &str) {
        ui.label(egui::RichText::new("(?)").weak())
            .on_hover_text(desc);
    }
}
