use crate::download_task_menu::DownloadTaskMenu;
use crate::freq_trainer_menu::FreqTrainerMenu;
use crate::imgui_notify::ImGuiNotify;
use crate::judge_trainer_menu::JudgeTrainerMenu;
use crate::misc_setting_menu::MiscSettingMenu;
use crate::performance_monitor::PerformanceMonitor;
use crate::random_trainer_menu::RandomTrainerMenu;
use crate::skin_menu::SkinMenu;
use crate::skin_widget_manager::SkinWidgetManager;
use crate::stubs::{
    Controller, ImBoolean, InputProcessor, Lwjgl3ControllerManager, Version, version,
};

use std::sync::Mutex;

static WINDOW_WIDTH: Mutex<i32> = Mutex::new(0);
static WINDOW_HEIGHT: Mutex<i32> = Mutex::new(0);

static SHOW_MOD_MENU: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_RANDOM_TRAINER: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_FREQ_PLUS: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_JUDGE_TRAINER: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_SONG_MANAGER: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_DOWNLOAD_MENU: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_SKIN_WIDGET_MANAGER: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_PERFORMANCE_MONITOR: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_SKIN_MENU: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static SHOW_MISC_SETTING: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });

pub fn window_width() -> i32 {
    *WINDOW_WIDTH.lock().unwrap()
}

pub fn window_height() -> i32 {
    *WINDOW_HEIGHT.lock().unwrap()
}

pub struct ImGuiRenderer;

impl ImGuiRenderer {
    pub fn init(width: i32, height: i32) {
        *WINDOW_WIDTH.lock().unwrap() = width;
        *WINDOW_HEIGHT.lock().unwrap() = height;

        // In Java:
        // Lwjgl3Graphics lwjglGraphics = ((Lwjgl3Graphics) Gdx.graphics);
        // imGuiGlfw = new ImGuiImplGlfw();
        // imGuiGl3 = new ImGuiImplGl3();
        // manager = new Lwjgl3ControllerManager();
        // windowHandle = lwjglGraphics.getWindow().getWindowHandle();
        // windowWidth = lwjglGraphics.getWidth();
        // windowHeight = lwjglGraphics.getHeight();
        // ImGui.createContext();
        // ... font loading, glyph ranges, etc.
        // imGuiGlfw.init(windowHandle, true);
        // imGuiGl3.init("#version 150");

        log::warn!("not yet implemented: ImGuiRenderer::init - egui integration");
    }

    pub fn start() {
        // if (tmpProcessor != null) {
        //     Gdx.input.setInputProcessor(tmpProcessor);
        //     tmpProcessor = null;
        // }
        // imGuiGl3.newFrame();
        // imGuiGlfw.newFrame();
        // ImGui.newFrame();
        log::warn!("not yet implemented: ImGuiRenderer::start - egui integration");
    }

    pub fn render() {
        // Relative from top left corner, so 44% from the left, 2% from the top
        let w = window_width();
        let h = window_height();
        let _relative_x = w as f32 * 0.44;
        let _relative_y = h as f32 * 0.02;
        // ImGui.setNextWindowPos(relativeX, relativeY, ImGuiCond.Once);

        let show_mod_menu = SHOW_MOD_MENU.lock().unwrap().get();
        if show_mod_menu {
            // ImGui.begin("Endless Dream", ImGuiWindowFlags.AlwaysAutoResize);

            // Checkboxes for sub-windows
            // ImGui.checkbox("Show Rate Modifier Window", SHOW_FREQ_PLUS);
            // ImGui.checkbox("Show Random Trainer Window", SHOW_RANDOM_TRAINER);
            // ImGui.checkbox("Show Judge Trainer Window", SHOW_JUDGE_TRAINER);
            // if (ImGui.checkbox("Show Skin Configuration Window", SHOW_SKIN_MENU)) { SkinMenu.invalidate(); }
            // ImGui.checkbox("Show Skin Widget Manager Window", SHOW_SKIN_WIDGET_MANAGER);
            // ImGui.checkbox("Show Song Manager Window", SHOW_SONG_MANAGER);
            // ImGui.checkbox("Show Download Tasks Window", SHOW_DOWNLOAD_MENU);
            // if (ImGui.checkbox("Show Performance Monitor Window", SHOW_PERFORMANCE_MONITOR) && SHOW_PERFORMANCE_MONITOR.get())
            //     { PerformanceMonitor.reloadEventTree(); }
            // ImGui.checkbox("Show Misc Setting Window", SHOW_MISC_SETTING);

            if SHOW_FREQ_PLUS.lock().unwrap().get() {
                let mut show = SHOW_FREQ_PLUS.lock().unwrap();
                FreqTrainerMenu::show(&mut show);
            }
            if SHOW_RANDOM_TRAINER.lock().unwrap().get() {
                let mut show = SHOW_RANDOM_TRAINER.lock().unwrap();
                RandomTrainerMenu::show(&mut show);
            }
            if SHOW_JUDGE_TRAINER.lock().unwrap().get() {
                let mut show = SHOW_JUDGE_TRAINER.lock().unwrap();
                JudgeTrainerMenu::show(&mut show);
            }
            if SHOW_SONG_MANAGER.lock().unwrap().get() {
                let mut show = SHOW_SONG_MANAGER.lock().unwrap();
                crate::song_manager_menu::SongManagerMenu::show(&mut show);
            }
            // TODO: This menu should based on config. Should not be rendered if user doesn't flag the http download feature
            if SHOW_DOWNLOAD_MENU.lock().unwrap().get() {
                let mut show = SHOW_DOWNLOAD_MENU.lock().unwrap();
                DownloadTaskMenu::show(&mut show);
            }
            if SHOW_SKIN_WIDGET_MANAGER.lock().unwrap().get() {
                SkinWidgetManager::set_focus(true);
                let mut show = SHOW_SKIN_WIDGET_MANAGER.lock().unwrap();
                SkinWidgetManager::show(&mut show);
            } else {
                SkinWidgetManager::set_focus(false);
            }
            if SHOW_PERFORMANCE_MONITOR.lock().unwrap().get() {
                let mut show = SHOW_PERFORMANCE_MONITOR.lock().unwrap();
                PerformanceMonitor::show(&mut show);
            }
            if SHOW_SKIN_MENU.lock().unwrap().get() {
                let mut show = SHOW_SKIN_MENU.lock().unwrap();
                SkinMenu::show(&mut show);
            }
            if SHOW_MISC_SETTING.lock().unwrap().get() {
                let mut show = SHOW_MISC_SETTING.lock().unwrap();
                MiscSettingMenu::show(&mut show);
            }

            // Debug information tree node
            // if (ImGui.treeNode("Endless Dream Debug Information"))
            {
                let _commit_hash = Version::get_git_commit_hash().unwrap_or("unknown");
                let _build_time = version::get_build_date().unwrap_or("unknown");
                // ImGui.text("Commit hash: " + commit_hash);
                // ImGui.text("Build time: " + build_time);
                // ImGui.text("GLFW version: " + GLFW.glfwGetVersionString());
                // for controller in manager.getControllers() { ... }
                // ImGui.treePop();
            }
            // ImGui.end();
        }

        ImGuiNotify::render_notifications();
    }

    pub fn end() {
        // ImGui.render();
        // imGuiGl3.renderDrawData(ImGui.getDrawData());
        // if (ImGui.getIO().getWantCaptureKeyboard() || ImGui.getIO().getWantCaptureMouse())
        // { ... capture input ... }
        log::warn!("not yet implemented: ImGuiRenderer::end - egui integration");
    }

    pub fn dispose() {
        // imGuiGl3.shutdown();
        // imGuiGlfw.shutdown();
        // ImGui.destroyContext();
        log::warn!("not yet implemented: ImGuiRenderer::dispose - egui integration");
    }

    pub fn get_show_mod_menu() -> bool {
        SHOW_MOD_MENU.lock().unwrap().get()
    }

    pub fn toggle_menu() {
        let mut menu = SHOW_MOD_MENU.lock().unwrap();
        let current = menu.get();
        menu.set(!current);
    }

    pub fn help_marker(_desc: &str) {
        // ImGui.textDisabled("(?)");
        // if (ImGui.isItemHovered()) {
        //     ImGui.beginTooltip();
        //     ImGui.pushTextWrapPos(ImGui.getFontSize() * 35.0f);
        //     ImGui.textUnformatted(desc);
        //     ImGui.popTextWrapPos();
        //     ImGui.endTooltip();
        // }
        log::warn!(
            "not yet implemented: ImGuiRenderer::help_marker - egui tooltip for '{}'",
            _desc
        );
    }
}
