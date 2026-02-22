use std::sync::{Arc, Mutex};

use md_processor::download_task::{DownloadTask, DownloadTaskStatus};
use md_processor::http_download_processor::HttpDownloadProcessor;

use crate::download_task_state::DownloadTaskState;
use crate::imgui_renderer;
use crate::stubs::ImBoolean;

pub const MAXIMUM_TASK_NAME_LENGTH: usize = 10;

pub struct DownloadTaskMenu;

impl DownloadTaskMenu {
    pub fn show(_show_download_tasks_window: &mut ImBoolean) {
        let _relative_x = imgui_renderer::window_width() as f32 * 0.455f32;
        let _relative_y = imgui_renderer::window_height() as f32 * 0.04f32;
        // ImGui.setNextWindowPos(relativeX, relativeY, ImGuiCond.FirstUseEver);

        // if (ImGui.begin("Download Tasks", showDownloadTasksWindow, ImGuiWindowFlags.AlwaysAutoResize))
        {
            let running = DownloadTaskState::get_running_download_tasks();
            let expired = DownloadTaskState::get_expired_tasks();
            if running.is_empty() && expired.is_empty() {
                // ImGui.text("No Download Task. Try selecting missing bms to submit new task!");
            } else {
                // Tab bar: Running / Expired
                // if (ImGui.beginTabBar("DownloadTasksTabBar"))
                {
                    // Running tab
                    {
                        let tasks: Vec<&Arc<Mutex<DownloadTask>>> = running.values().collect();
                        Self::render_task_table(&tasks);
                    }
                    // Expired tab
                    {
                        let tasks: Vec<&Arc<Mutex<DownloadTask>>> = expired.values().collect();
                        Self::render_task_table(&tasks);
                    }
                }
            }
        }
        // ImGui.end();
        log::warn!("not yet implemented: DownloadTaskMenu::show - egui integration");
    }

    fn render_task_table(tasks: &[&Arc<Mutex<DownloadTask>>]) {
        // if (ImGui.beginTable("DownloadTaskTable", 3, ...))
        for task_arc in tasks {
            let task = task_arc.lock().unwrap();
            // ImGui.tableNextRow();
            // ImGui.pushID(task.getId());

            // Column 0: Task name
            let name = task.get_name();
            let task_name = if name.len() > MAXIMUM_TASK_NAME_LENGTH {
                &name[..MAXIMUM_TASK_NAME_LENGTH]
            } else {
                name
            };
            let _display = format!("{} ({})", task_name, task.get_download_task_status().name());
            // ImGui.text(display);

            // Column 1: Progress
            let error_message = task.get_error_message();
            if error_message.is_none() || error_message.is_some_and(|s| s.is_empty()) {
                let _progress = format!(
                    "{}/{}",
                    humanize_file_size(task.get_download_size()),
                    humanize_file_size(task.get_content_length())
                );
                // ImGui.text(progress);
            } else {
                let _msg = error_message.unwrap_or("");
                // ImGui.textColored(ImColor.rgb(255, 0, 0), msg);
            }

            // Column 2: Operation
            if task.get_download_task_status() == DownloadTaskStatus::Error {
                // if (ImGui.button("Retry")) { processor.retryDownloadTask(task); }
            }

            // ImGui.popID();
        }
        // ImGui.endTable();
        log::warn!("not yet implemented: DownloadTaskMenu::render_task_table - egui integration");
    }

    /// Render the download task window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Download Tasks")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                let running = DownloadTaskState::get_running_download_tasks();
                let expired = DownloadTaskState::get_expired_tasks();
                if running.is_empty() && expired.is_empty() {
                    ui.label("No Download Task. Try selecting missing bms to submit new task!");
                } else {
                    ui.horizontal(|ui| {
                        ui.label(format!("Running: {}", running.len()));
                        ui.label(format!("Expired: {}", expired.len()));
                    });
                    ui.separator();
                    for task_arc in running.values() {
                        let task = task_arc.lock().unwrap();
                        let name = task.get_name();
                        let task_name = if name.len() > MAXIMUM_TASK_NAME_LENGTH {
                            &name[..MAXIMUM_TASK_NAME_LENGTH]
                        } else {
                            name
                        };
                        let download_status = task.get_download_task_status();
                        let status = download_status.name();
                        let progress = format!(
                            "{}/{}",
                            humanize_file_size(task.get_download_size()),
                            humanize_file_size(task.get_content_length())
                        );
                        ui.label(format!("{} ({}) - {}", task_name, status, progress));
                    }
                }
            });
    }
}

pub fn humanize_file_size(bytes: i64) -> String {
    let thresh: i64 = 1000;
    if bytes.abs() < thresh {
        return format!("{} B", bytes);
    }

    let mut result = bytes as f64;
    let units = ["KB", "MB", "GB", "TB"];
    let mut u: i32 = -1;
    let r: f64 = 100.0;

    loop {
        result /= thresh as f64;
        u += 1;
        if !((result.abs() * r).round() / r >= thresh as f64 && (u as usize) < units.len() - 1) {
            break;
        }
    }

    format!("{:.1} {}", result, units[u as usize])
}
