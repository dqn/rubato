use crate::imgui_renderer;
use crate::judge_trainer::JudgeTrainer;
use crate::stubs::{ImBoolean, ImInt};

use std::sync::Mutex;

static OVERRIDE_CHART_JUDGE: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static OVERRIDE_JUDGE_RANK: Mutex<ImInt> = Mutex::new(ImInt { value: 0 });

pub struct JudgeTrainerMenu;

impl JudgeTrainerMenu {
    pub fn show(_show_judge_trainer: &mut ImBoolean) {
        let _relative_x = imgui_renderer::window_width() as f32 * 0.455f32;
        let _relative_y = imgui_renderer::window_height() as f32 * 0.04f32;
        // ImGui.setWindowPos(relativeX, relativeY, ImGuiCond.FirstUseEver);

        // if (ImGui.begin("Judge Trainer", showJudgeTrainer, ImGuiWindowFlags.AlwaysAutoResize))
        {
            // if (ImGui.checkbox("Override chart's judge", OVERRIDE_CHART_JUDGE))
            {
                let override_judge = OVERRIDE_CHART_JUDGE.lock().unwrap();
                JudgeTrainer::set_active(override_judge.get());
            }
            // if (ImGui.combo("judge", OVERRIDE_JUDGE_RANK, JudgeTrainer.JUDGE_OPTIONS))
            {
                let override_rank = OVERRIDE_JUDGE_RANK.lock().unwrap();
                JudgeTrainer::set_judge_rank(override_rank.get());
            }
            // ImGui.end();
        }
        log::warn!("not yet implemented: JudgeTrainerMenu::show - egui integration");
    }
}
