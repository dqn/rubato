use std::sync::{Arc, Mutex};

use crate::song::md_processor::download_task::{DownloadTask, DownloadTaskStatus};
use crate::song::md_processor::http_download_processor::HttpDownloadProcessor;

use super::imgui_renderer;
use crate::song::md_processor::download_task_state::DownloadTaskState;
use rubato_types::sync_utils::lock_or_recover;

pub const MAXIMUM_TASK_NAME_LENGTH: usize = 10;

static PROCESSOR: Mutex<Option<Arc<HttpDownloadProcessor>>> = Mutex::new(None);

/// Returns the largest byte index `<= index` that is on a UTF-8 char boundary.
/// Equivalent to `str::floor_char_boundary` (stable since Rust 1.91), provided
/// here to stay compatible with MSRV 1.89.
fn floor_char_boundary(s: &str, index: usize) -> usize {
    if index >= s.len() {
        return s.len();
    }
    let mut i = index;
    // Walk backwards until we find a byte that is not a UTF-8 continuation byte.
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

pub struct DownloadTaskMenu;

impl DownloadTaskMenu {
    /// Sets the HttpDownloadProcessor used by DownloadTaskMenu.
    ///
    /// Translated from: DownloadTaskMenu.setProcessor(HttpDownloadProcessor)
    pub fn set_processor(processor: Arc<HttpDownloadProcessor>) {
        let mut guard = lock_or_recover(&PROCESSOR);
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
                    let task = lock_or_recover(task_arc);

                    // Column 0: Task name
                    let name = task.name();
                    let task_name = if name.len() > MAXIMUM_TASK_NAME_LENGTH {
                        let end = floor_char_boundary(name, MAXIMUM_TASK_NAME_LENGTH);
                        &name[..end]
                    } else {
                        name
                    };
                    let display = format!("{} ({})", task_name, task.download_task_status().name());
                    ui.label(&display);

                    // Column 1: Progress
                    let error_message = task.error_message();
                    if error_message.is_none() || error_message.is_some_and(|s: &str| s.is_empty())
                    {
                        let progress = format!(
                            "{}/{}",
                            humanize_file_size(task.download_size),
                            humanize_file_size(task.content_length)
                        );
                        ui.label(&progress);
                    } else {
                        let msg = error_message.unwrap_or("");
                        ui.label(egui::RichText::new(msg).color(egui::Color32::RED));
                    }

                    // Column 2: Operation — retry button for errored tasks
                    let is_error = task.download_task_status() == DownloadTaskStatus::Error;
                    drop(task); // release lock before UI interaction
                    if is_error {
                        if ui.button("Retry").clicked() {
                            let processor = lock_or_recover(&PROCESSOR);
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
        // Update task state before reading (Java: DownloadTaskState.update() called each frame)
        {
            let processor = lock_or_recover(&PROCESSOR);
            if let Some(ref proc) = *processor {
                DownloadTaskState::update(proc);
            }
        }

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

    /// Helper that mirrors the truncation logic in render_task_table.
    fn truncate_task_name(name: &str) -> &str {
        if name.len() > MAXIMUM_TASK_NAME_LENGTH {
            let end = floor_char_boundary(name, MAXIMUM_TASK_NAME_LENGTH);
            &name[..end]
        } else {
            name
        }
    }

    #[test]
    fn test_truncate_task_name_ascii_short() {
        // ASCII string shorter than limit is returned as-is.
        assert_eq!(truncate_task_name("hello"), "hello");
    }

    #[test]
    fn test_truncate_task_name_ascii_exact() {
        // ASCII string exactly at limit is returned as-is.
        assert_eq!(truncate_task_name("0123456789"), "0123456789");
    }

    #[test]
    fn test_truncate_task_name_ascii_over() {
        // ASCII string over limit is truncated at byte 10.
        assert_eq!(truncate_task_name("0123456789abc"), "0123456789");
    }

    #[test]
    fn test_truncate_task_name_japanese_no_panic() {
        // Japanese text where byte position 10 falls mid-character.
        // Each CJK character is 3 bytes in UTF-8, so 4 chars = 12 bytes.
        // floor_char_boundary(10) should round down to byte 9 (3 chars).
        let name = "\u{5929}\u{7A7A}\u{306E}\u{57CE}"; // "天空の城" (12 bytes)
        assert_eq!(name.len(), 12);
        let truncated = truncate_task_name(name);
        // Must not panic, and must be valid UTF-8 with at most 10 bytes.
        assert!(truncated.len() <= MAXIMUM_TASK_NAME_LENGTH);
        assert_eq!(truncated, "天空の"); // 9 bytes (3 chars * 3 bytes)
    }

    #[test]
    fn test_truncate_task_name_mixed_multibyte() {
        // Mixed ASCII + Japanese where boundary falls mid-character.
        // "abc天空" = 3 + 3 + 3 + 3 + 3 = wait, let's be precise:
        // 'a'=1, 'b'=1, 'c'=1, '天'=3, '空'=3, 'の'=3, '城'=3 = 15 bytes
        let name = "abc天空の城";
        assert_eq!(name.len(), 15);
        let truncated = truncate_task_name(name);
        // floor_char_boundary(10) on "abc天空の城":
        // bytes 0-2: "abc", byte 3-5: "天", byte 6-8: "空", byte 9-11: "の"
        // byte 10 is mid-char for "の", so floor rounds to byte 9.
        assert_eq!(truncated, "abc天空"); // 9 bytes
        assert!(truncated.len() <= MAXIMUM_TASK_NAME_LENGTH);
    }

    #[test]
    fn test_truncate_task_name_emoji() {
        // Emoji are 4 bytes each. "🎵🎶🎷" = 12 bytes.
        // floor_char_boundary(10) should round to byte 8 (2 emoji).
        let name = "🎵🎶🎷";
        assert_eq!(name.len(), 12);
        let truncated = truncate_task_name(name);
        assert_eq!(truncated, "🎵🎶"); // 8 bytes
        assert!(truncated.len() <= MAXIMUM_TASK_NAME_LENGTH);
    }
}
