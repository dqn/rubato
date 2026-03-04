use std::sync::{Arc, Mutex};

use beatoraja_song::md_processor::download_task::{DownloadTask, DownloadTaskStatus};
use beatoraja_song::md_processor::http_download_processor::HttpDownloadProcessor;

use super::imgui_renderer;
use beatoraja_song::md_processor::download_task_state::DownloadTaskState;

pub const MAXIMUM_TASK_NAME_LENGTH: usize = 10;

static PROCESSOR: Mutex<Option<Arc<HttpDownloadProcessor>>> = Mutex::new(None);

pub struct DownloadTaskMenu;

impl DownloadTaskMenu {
    /// Sets the HttpDownloadProcessor used by DownloadTaskMenu.
    ///
    /// Translated from: DownloadTaskMenu.setProcessor(HttpDownloadProcessor)
    pub fn set_processor(processor: Arc<HttpDownloadProcessor>) {
        let mut guard = PROCESSOR.lock().unwrap();
        *guard = Some(processor);
    }

    /// Render a table of download tasks using egui.
    ///
    /// Translated from: DownloadTaskMenu.renderTaskTable(List<DownloadTask>)
    fn render_task_table(ui: &mut egui::Ui, tasks: &[Arc<Mutex<DownloadTask>>]) {
        egui::Grid::new("DownloadTaskTable")
            .num_columns(3)
            .striped(true)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                // Header row
                ui.strong("Task");
                ui.strong("Progress");
                ui.strong("Op");
                ui.end_row();

                for task_arc in tasks {
                    let task = task_arc.lock().unwrap();

                    // Column 0: Task name
                    let name = task.get_name();
                    let task_name = if name.len() > MAXIMUM_TASK_NAME_LENGTH {
                        &name[..MAXIMUM_TASK_NAME_LENGTH]
                    } else {
                        name
                    };
                    let display =
                        format!("{} ({})", task_name, task.get_download_task_status().name());
                    ui.label(&display);

                    // Column 1: Progress
                    let error_message = task.get_error_message();
                    if error_message.is_none() || error_message.is_some_and(|s: &str| s.is_empty())
                    {
                        let progress = format!(
                            "{}/{}",
                            humanize_file_size(task.get_download_size()),
                            humanize_file_size(task.get_content_length())
                        );
                        ui.label(&progress);
                    } else {
                        let msg = error_message.unwrap_or("");
                        ui.label(egui::RichText::new(msg).color(egui::Color32::RED));
                    }

                    // Column 2: Operation — retry button for errored tasks
                    let is_error = task.get_download_task_status() == DownloadTaskStatus::Error;
                    drop(task); // release lock before UI interaction
                    if is_error {
                        if ui.button("Retry").clicked() {
                            let processor = PROCESSOR.lock().unwrap();
                            if let Some(ref proc) = *processor {
                                proc.retry_download_task(task_arc.clone());
                            }
                        }
                    } else {
                        ui.label("");
                    }

                    ui.end_row();
                }
            });
    }

    /// Render the download task window using egui.
    ///
    /// Translated from: DownloadTaskMenu.show(ImBoolean)
    pub fn show_ui(ctx: &egui::Context) {
        // Window positioning: 45.5% from left, 4% from top (Java: windowWidth * 0.455f, windowHeight * 0.04f)
        let rel_x = imgui_renderer::window_width() as f32 * 0.455;
        let rel_y = imgui_renderer::window_height() as f32 * 0.04;

        let mut open = true;
        egui::Window::new("Download Tasks")
            .open(&mut open)
            .default_pos(egui::pos2(rel_x, rel_y))
            .auto_sized()
            .show(ctx, |ui| {
                let running = DownloadTaskState::get_running_download_tasks();
                let expired = DownloadTaskState::get_expired_tasks();
                if running.is_empty() && expired.is_empty() {
                    ui.label("No Download Task. Try selecting missing bms to submit new task!");
                } else {
                    let running_tasks: Vec<Arc<Mutex<DownloadTask>>> =
                        running.values().cloned().collect();
                    let expired_tasks: Vec<Arc<Mutex<DownloadTask>>> =
                        expired.values().cloned().collect();

                    // Tab bar: Running / Expired (Java: ImGui.beginTabBar("DownloadTasksTabBar"))
                    ui.horizontal(|ui| {
                        ui.label(format!("Running: {}", running_tasks.len()));
                        ui.separator();
                        ui.label(format!("Expired: {}", expired_tasks.len()));
                    });
                    ui.separator();

                    ui.collapsing(egui::RichText::new("Running").strong(), |ui| {
                        Self::render_task_table(ui, &running_tasks);
                    });
                    ui.collapsing(egui::RichText::new("Expired").strong(), |ui| {
                        Self::render_task_table(ui, &expired_tasks);
                    });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanize_file_size_bytes() {
        assert_eq!(humanize_file_size(0), "0 B");
        assert_eq!(humanize_file_size(1), "1 B");
        assert_eq!(humanize_file_size(999), "999 B");
    }

    #[test]
    fn test_humanize_file_size_kilobytes() {
        assert_eq!(humanize_file_size(1000), "1.0 KB");
        assert_eq!(humanize_file_size(1500), "1.5 KB");
        // 999_999 rounds up to 1.0 MB due to the rounding threshold in the loop
        assert_eq!(humanize_file_size(999_999), "1.0 MB");
        assert_eq!(humanize_file_size(500_000), "500.0 KB");
    }

    #[test]
    fn test_humanize_file_size_megabytes() {
        assert_eq!(humanize_file_size(1_000_000), "1.0 MB");
        assert_eq!(humanize_file_size(5_500_000), "5.5 MB");
    }

    #[test]
    fn test_humanize_file_size_gigabytes() {
        assert_eq!(humanize_file_size(1_000_000_000), "1.0 GB");
        assert_eq!(humanize_file_size(2_500_000_000), "2.5 GB");
    }

    #[test]
    fn test_humanize_file_size_terabytes() {
        assert_eq!(humanize_file_size(1_000_000_000_000), "1.0 TB");
    }

    #[test]
    fn test_humanize_file_size_negative_bytes() {
        assert_eq!(humanize_file_size(-500), "-500 B");
    }

    #[test]
    fn test_humanize_file_size_negative_kilobytes() {
        assert_eq!(humanize_file_size(-1500), "-1.5 KB");
    }

    #[test]
    fn test_maximum_task_name_length_constant() {
        assert_eq!(MAXIMUM_TASK_NAME_LENGTH, 10);
    }
}
