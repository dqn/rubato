use crate::imgui_renderer;
use crate::stubs::ImBoolean;

use std::sync::Mutex;

static FREQ_TRAINER_ENABLED: Mutex<bool> = Mutex::new(false);
static FREQ: Mutex<i32> = Mutex::new(100);

pub struct FreqTrainerMenu;

impl FreqTrainerMenu {
    pub fn show(_show_freq_trainer: &mut ImBoolean) {
        let _relative_x = imgui_renderer::window_width() as f32 * 0.47f32;
        let _relative_y = imgui_renderer::window_height() as f32 * 0.06f32;
        // ImGui.setNextWindowPos(relativeX, relativeY, ImGuiCond.FirstUseEver);

        // if(ImGui.begin("Rate Modifier", showFreqTrainer, ImGuiWindowFlags.AlwaysAutoResize))
        {
            // ImGui.text("Modifies the chart playback rate to be faster or");
            // ImGui.text("slower by a given percent.");

            let button_vals: Vec<i32> = vec![-10, -5, -1, 100, 1, 5, 10];
            for value in &button_vals {
                if *value == 100 {
                    // if(ImGui.button("Reset")) { freq[0] = 100; }
                    // Reset button
                } else {
                    // Button with label like "+5%" or "-10%"
                    let _label = if *value > 0 {
                        format!("+{}%", value)
                    } else {
                        format!("{}%", value)
                    };
                    // if(ImGui.button(label)) { freq[0] = clamp(freq[0] + value); }
                }
                // ImGui.sameLine();
            }
            // ImGui.newLine();
            // ImGui.sliderInt("%", freq, 50, 200);

            // ImGui.text("Controls");
            // ImGui.indent();
            // ImGui.checkbox("Rate Enabled", FREQ_TRAINER_ENABLED);
            // ImGui.sameLine();
            // helpMarker("When enabled positive rate scores will save locally, ...");

            let mut freq = FREQ.lock().unwrap();
            *freq = clamp(*freq);
        }
        // ImGui.end();
        log::warn!("not yet implemented: FreqTrainerMenu::show - egui integration");
    }

    pub fn is_freq_trainer_enabled() -> bool {
        *FREQ_TRAINER_ENABLED.lock().unwrap()
    }

    pub fn set_freq_trainer_enabled(enabled: bool) {
        *FREQ_TRAINER_ENABLED.lock().unwrap() = enabled;
    }

    pub fn get_freq() -> i32 {
        *FREQ.lock().unwrap()
    }

    pub fn is_freq_negative() -> bool {
        *FREQ.lock().unwrap() < 100
    }

    pub fn get_freq_string() -> String {
        let freq = *FREQ.lock().unwrap();
        let rate = freq as f32 / 100.0f32;
        format!("[{:.02}x]", rate)
    }

    /// Render the rate modifier window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Rate Modifier")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                ui.label("Modifies the chart playback rate to be faster or");
                ui.label("slower by a given percent.");

                ui.horizontal(|ui| {
                    let button_vals: Vec<i32> = vec![-10, -5, -1, 100, 1, 5, 10];
                    for value in &button_vals {
                        let label = if *value == 100 {
                            "Reset".to_string()
                        } else if *value > 0 {
                            format!("+{}%", value)
                        } else {
                            format!("{}%", value)
                        };
                        if ui.button(&label).clicked() {
                            let mut freq = FREQ.lock().unwrap();
                            if *value == 100 {
                                *freq = 100;
                            } else {
                                *freq = clamp(*freq + *value);
                            }
                        }
                    }
                });

                let mut freq = *FREQ.lock().unwrap();
                ui.add(egui::Slider::new(&mut freq, 50..=200).text("%"));
                *FREQ.lock().unwrap() = clamp(freq);

                ui.separator();
                ui.label("Controls");
                ui.indent("freq_controls", |ui| {
                    let mut enabled = *FREQ_TRAINER_ENABLED.lock().unwrap();
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut enabled, "Rate Enabled");
                        crate::imgui_renderer::ImGuiRenderer::help_marker(
                            ui,
                            "When enabled positive rate scores will save locally, negative rate scores never save.",
                        );
                    });
                    *FREQ_TRAINER_ENABLED.lock().unwrap() = enabled;
                });
            });
    }
}

fn clamp(result: i32) -> i32 {
    result.clamp(50, 200)
}
