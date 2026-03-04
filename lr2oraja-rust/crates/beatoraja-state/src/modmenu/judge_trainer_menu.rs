use super::judge_trainer::JudgeTrainer;

use std::sync::Mutex;

static OVERRIDE_CHART_JUDGE: Mutex<bool> = Mutex::new(false);
static OVERRIDE_JUDGE_RANK: Mutex<i32> = Mutex::new(0);

pub struct JudgeTrainerMenu;

impl JudgeTrainerMenu {
    /// Render the judge trainer window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        let mut open = true;
        egui::Window::new("Judge Trainer")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                let mut override_judge = *OVERRIDE_CHART_JUDGE.lock().unwrap();
                if ui
                    .checkbox(&mut override_judge, "Override chart's judge")
                    .changed()
                {
                    *OVERRIDE_CHART_JUDGE.lock().unwrap() = override_judge;
                    JudgeTrainer::set_active(override_judge);
                }

                let mut rank = *OVERRIDE_JUDGE_RANK.lock().unwrap();
                let judge_options = crate::modmenu::judge_trainer::JUDGE_OPTIONS;
                let selected_text = judge_options
                    .get(rank as usize)
                    .copied()
                    .unwrap_or("Unknown");
                egui::ComboBox::from_label("judge")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        for (i, option) in judge_options.iter().enumerate() {
                            if ui.selectable_value(&mut rank, i as i32, *option).clicked() {
                                *OVERRIDE_JUDGE_RANK.lock().unwrap() = rank;
                                JudgeTrainer::set_judge_rank(rank);
                            }
                        }
                    });
            });
        if !open {
            // Window closed
        }
    }
}
