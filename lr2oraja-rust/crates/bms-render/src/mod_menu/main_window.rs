// Main ModMenu window — renders checkboxes to toggle sub-windows.
//
// Corresponds to Java `ImGuiRenderer.render()` main menu section.
// The window title "Endless Dream" matches the Java original.

use super::ModMenuState;
use super::menus;

pub fn render(ctx: &egui::Context, state: &mut ModMenuState) {
    egui::Window::new("Endless Dream")
        .resizable(false)
        .show(ctx, |ui| {
            ui.checkbox(&mut state.show_freq_trainer, "Show Rate Modifier Window");
            ui.checkbox(&mut state.show_random_trainer, "Show Random Trainer Window");
            ui.checkbox(&mut state.show_judge_trainer, "Show Judge Trainer Window");
            ui.checkbox(&mut state.show_song_manager, "Show Song Manager Window");
            ui.checkbox(&mut state.show_download_tasks, "Show Download Tasks Window");
            ui.checkbox(&mut state.show_misc_setting, "Show Misc Setting Window");
            ui.checkbox(
                &mut state.show_skin_widget_manager,
                "Show Skin Widget Manager Window",
            );
            ui.checkbox(
                &mut state.show_performance_monitor,
                "Show Performance Monitor Window",
            );
            ui.checkbox(
                &mut state.show_window_settings,
                "Show Window Settings Window",
            );

            ui.separator();
            ui.label("Rust Extensions");

            ui.checkbox(
                &mut state.show_gauge_visualizer,
                "Show Gauge Visualizer Window",
            );
            ui.checkbox(&mut state.show_timer_display, "Show Timer Display Window");
            ui.checkbox(&mut state.show_event_trace, "Show Event Trace Window");
            ui.checkbox(&mut state.show_profiler, "Show Profiler Window");
            ui.checkbox(&mut state.show_skin_options, "Show Skin Options Window");
        });

    if state.show_freq_trainer {
        menus::freq_trainer::render(ctx, &mut state.show_freq_trainer, &mut state.freq_trainer);
    }
    if state.show_random_trainer {
        menus::random_trainer::render(
            ctx,
            &mut state.show_random_trainer,
            &mut state.random_trainer,
        );
    }
    if state.show_judge_trainer {
        menus::judge_trainer::render(ctx, &mut state.show_judge_trainer, &mut state.judge_trainer);
    }
    if state.show_song_manager {
        menus::song_manager::render(ctx, &mut state.show_song_manager, &mut state.song_manager);
    }
    if state.show_download_tasks {
        menus::download_task::render(
            ctx,
            &mut state.show_download_tasks,
            &mut state.download_tasks,
        );
    }
    if state.show_misc_setting {
        menus::misc_setting::render(ctx, &mut state.show_misc_setting, &mut state.misc_setting);
    }
    if state.show_skin_widget_manager {
        menus::skin_widget_manager::render(
            ctx,
            &mut state.show_skin_widget_manager,
            &mut state.skin_widget_manager,
        );
    }
    if state.show_performance_monitor {
        menus::performance_monitor::render(
            ctx,
            &mut state.show_performance_monitor,
            &mut state.performance_monitor,
        );
    }
    if state.show_window_settings {
        menus::window_settings::render(
            ctx,
            &mut state.show_window_settings,
            &mut state.window_settings,
        );
    }
    if state.show_gauge_visualizer {
        menus::gauge_visualizer::render(
            ctx,
            &mut state.show_gauge_visualizer,
            &mut state.gauge_visualizer,
        );
    }
    if state.show_timer_display {
        menus::timer_display::render(ctx, &mut state.show_timer_display, &mut state.timer_display);
    }
    if state.show_event_trace {
        menus::event_trace::render(ctx, &mut state.show_event_trace, &mut state.event_trace);
    }
    if state.show_profiler {
        menus::profiler::render(ctx, &mut state.show_profiler, &mut state.profiler);
    }
    if state.show_skin_options {
        menus::skin_options::render(ctx, &mut state.show_skin_options, &mut state.skin_options);
    }
}
