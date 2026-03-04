use std::sync::Mutex;

static FREQ_TRAINER_ENABLED: Mutex<bool> = Mutex::new(false);
static FREQ: Mutex<i32> = Mutex::new(100);

pub struct FreqTrainerMenu;

impl FreqTrainerMenu {
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
                        crate::modmenu::imgui_renderer::ImGuiRenderer::help_marker(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clamp_within_range() {
        assert_eq!(clamp(100), 100);
        assert_eq!(clamp(50), 50);
        assert_eq!(clamp(200), 200);
        assert_eq!(clamp(150), 150);
    }

    #[test]
    fn test_clamp_below_minimum() {
        assert_eq!(clamp(0), 50);
        assert_eq!(clamp(49), 50);
        assert_eq!(clamp(-100), 50);
    }

    #[test]
    fn test_clamp_above_maximum() {
        assert_eq!(clamp(201), 200);
        assert_eq!(clamp(500), 200);
    }

    #[test]
    fn test_get_freq_string_default() {
        // Reset state to known value
        *FREQ.lock().unwrap() = 100;
        assert_eq!(FreqTrainerMenu::get_freq_string(), "[1.00x]");
    }

    #[test]
    fn test_get_freq_string_half_speed() {
        *FREQ.lock().unwrap() = 50;
        assert_eq!(FreqTrainerMenu::get_freq_string(), "[0.50x]");
    }

    #[test]
    fn test_get_freq_string_double_speed() {
        *FREQ.lock().unwrap() = 200;
        assert_eq!(FreqTrainerMenu::get_freq_string(), "[2.00x]");
    }

    #[test]
    fn test_get_freq_string_fractional() {
        *FREQ.lock().unwrap() = 75;
        assert_eq!(FreqTrainerMenu::get_freq_string(), "[0.75x]");
    }

    #[test]
    fn test_is_freq_negative_below_100() {
        *FREQ.lock().unwrap() = 99;
        assert!(FreqTrainerMenu::is_freq_negative());

        *FREQ.lock().unwrap() = 50;
        assert!(FreqTrainerMenu::is_freq_negative());
    }

    #[test]
    fn test_is_freq_negative_at_or_above_100() {
        *FREQ.lock().unwrap() = 100;
        assert!(!FreqTrainerMenu::is_freq_negative());

        *FREQ.lock().unwrap() = 150;
        assert!(!FreqTrainerMenu::is_freq_negative());
    }

    #[test]
    fn test_freq_trainer_enabled_toggle() {
        FreqTrainerMenu::set_freq_trainer_enabled(false);
        assert!(!FreqTrainerMenu::is_freq_trainer_enabled());

        FreqTrainerMenu::set_freq_trainer_enabled(true);
        assert!(FreqTrainerMenu::is_freq_trainer_enabled());

        // Clean up
        FreqTrainerMenu::set_freq_trainer_enabled(false);
    }
}
