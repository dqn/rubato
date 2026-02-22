use crate::imgui_renderer;
use crate::random_trainer::RandomTrainer;
use crate::stubs::ImBoolean;

use std::sync::Mutex;

static RANDOM_TRAINER_ENABLED: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static BLACK_WHITE_RANDOM_PERMUTATION: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });
static LANE_ORDER: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TRACK_RAN_WHEN_DISABLED: Mutex<ImBoolean> = Mutex::new(ImBoolean { value: false });

fn init_lane_order() {
    let mut lo = LANE_ORDER.lock().unwrap();
    if lo.is_empty() {
        *lo = vec![
            "1".to_string(),
            "2".to_string(),
            "3".to_string(),
            "4".to_string(),
            "5".to_string(),
            "6".to_string(),
            "7".to_string(),
        ];
    }
}

pub struct RandomTrainerMenu;

impl RandomTrainerMenu {
    pub fn show(_show_random_trainer: &mut ImBoolean) {
        init_lane_order();
        let _relative_x = imgui_renderer::window_width() as f32 * 0.455f32;
        let _relative_y = imgui_renderer::window_height() as f32 * 0.04f32;
        // ImGui.setNextWindowPos(relativeX, relativeY, ImGuiCond.FirstUseEver);

        // if(ImGui.begin("Random Trainer", showRandomTrainer, ImGuiWindowFlags.AlwaysAutoResize))
        {
            // Update key display when tracking random
            let track_ran = TRACK_RAN_WHEN_DISABLED.lock().unwrap().get();
            if track_ran {
                let history = RandomTrainer::get_random_history();
                if !history.is_empty() {
                    let last_ran = history.front().unwrap().get_random().to_string();
                    change_lane_order(&last_ran);
                }
            }

            let bw_permute = BLACK_WHITE_RANDOM_PERMUTATION.lock().unwrap().get();
            RandomTrainer::set_black_white_permute(bw_permute);

            // Key display
            Self::drag_and_drop_key_display();

            // Random History
            Self::random_history();

            // Controls
            // ImGui.text("Controls");
            // ImGui.indent();
            // ImGui.checkbox("Trainer Enabled", RANDOM_TRAINER_ENABLED);
            // ImGui.checkbox("Track Current Random", TRACK_RAN_WHEN_DISABLED);
            // ImGui.checkbox("Black/White Random Select", BLACK_WHITE_RANDOM_PERMUTATION);
            // ImGui.unindent();

            // Mirror / Shift buttons
            // if (ImGui.button("Mirror")) { mirrorLaneOrder(); }
            // if (ImGui.button("Shift Left")) { shiftLeftLaneOrder(); }
            // if (ImGui.button("Shift Right")) { shiftRightLaneOrder(); }

            let trainer_enabled = RANDOM_TRAINER_ENABLED.lock().unwrap().get();
            RandomTrainer::set_active(trainer_enabled);
            if trainer_enabled {
                let current_ui_lane_order = get_lane_order_string();
                let trainer_lane_order = RandomTrainer::get_lane_order();
                if current_ui_lane_order != trainer_lane_order {
                    RandomTrainer::set_lane_order(&current_ui_lane_order);
                }
            }
        }
        // ImGui.end();
        log::warn!("not yet implemented: RandomTrainerMenu::show - egui integration");
    }

    fn random_history() {
        // if (ImGui.treeNode("Random History"))
        {
            let history = RandomTrainer::get_random_history();
            for entry in &history {
                let _title = entry.get_title();
                let _random = entry.get_random();
                // Render table rows
                // Double click to select as current random
            }
        }
        // ImGui.treePop();
    }

    fn drag_and_drop_key_display() {
        let lane_order = LANE_ORDER.lock().unwrap();
        let bw_permute = BLACK_WHITE_RANDOM_PERMUTATION.lock().unwrap().get();

        for i in 0..lane_order.len() {
            let lane_char = lane_order[i].chars().next().unwrap_or('1');
            let to_random = RandomTrainer::is_lane_to_random(lane_char);

            // Color selection based on black/white keys and random state
            if to_random {
                // push pink style
            } else if lane_char.to_digit(10).unwrap_or(0).is_multiple_of(2) {
                // push dark blue style (black key)
            } else {
                // push light style (white key)
            }

            if bw_permute {
                // ImGui.button("", 50, 80);
            } else if to_random {
                // ImGui.button("?", 50, 80);
            } else {
                // ImGui.button(lane_order[i], 50, 80);
            }

            // Drag & drop source/target for reordering
            // Right-click to toggle random
            if to_random {
                // RandomTrainer.removeLaneToRandom(...)
            } else {
                // RandomTrainer.setLaneToRandom(...)
            }
        }
    }

    pub fn mirror_lane_order() {
        let s = get_lane_order_string();
        let reversed: String = s.chars().rev().collect();
        change_lane_order(&reversed);
    }

    /// 1234567 -> 2345671
    pub fn shift_left_lane_order() {
        let s = get_lane_order_string();
        if s.len() > 1 {
            let rotated = format!("{}{}", &s[1..], &s[..1]);
            change_lane_order(&rotated);
        }
    }

    /// 1234567 -> 7123456
    pub fn shift_right_lane_order() {
        let s = get_lane_order_string();
        if s.len() > 1 {
            let last = &s[s.len() - 1..];
            let rest = &s[..s.len() - 1];
            let rotated = format!("{}{}", last, rest);
            change_lane_order(&rotated);
        }
    }

    /// Render the random trainer window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        init_lane_order();
        let mut open = true;
        egui::Window::new("Random Trainer")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                // Key display
                ui.horizontal(|ui| {
                    let lane_order = LANE_ORDER.lock().unwrap();
                    for i in 0..lane_order.len() {
                        let lane_char = lane_order[i].chars().next().unwrap_or('1');
                        let is_random =
                            crate::random_trainer::RandomTrainer::is_lane_to_random(lane_char);
                        let label = if is_random {
                            "?".to_string()
                        } else {
                            lane_order[i].clone()
                        };
                        let color = if is_random {
                            egui::Color32::from_rgb(255, 100, 150) // pink
                        } else if lane_char.to_digit(10).unwrap_or(0) % 2 == 0 {
                            egui::Color32::from_rgb(50, 50, 150) // dark blue
                        } else {
                            egui::Color32::from_rgb(200, 200, 200) // light
                        };
                        let btn = egui::Button::new(
                            egui::RichText::new(&label).size(18.0).color(color),
                        )
                        .min_size(egui::vec2(40.0, 60.0));
                        ui.add(btn);
                    }
                });

                // Controls
                ui.separator();
                ui.label("Controls");
                ui.indent("random_controls", |ui| {
                    let mut enabled = RANDOM_TRAINER_ENABLED.lock().unwrap();
                    ui.checkbox(&mut enabled.value, "Trainer Enabled");
                    drop(enabled);

                    let mut track = TRACK_RAN_WHEN_DISABLED.lock().unwrap();
                    ui.checkbox(&mut track.value, "Track Current Random");
                    drop(track);

                    let mut bw = BLACK_WHITE_RANDOM_PERMUTATION.lock().unwrap();
                    ui.checkbox(&mut bw.value, "Black/White Random Select");
                    drop(bw);
                });

                ui.horizontal(|ui| {
                    if ui.button("Mirror").clicked() {
                        Self::mirror_lane_order();
                    }
                    if ui.button("Shift Left").clicked() {
                        Self::shift_left_lane_order();
                    }
                    if ui.button("Shift Right").clicked() {
                        Self::shift_right_lane_order();
                    }
                });

                // Sync state
                let trainer_enabled = RANDOM_TRAINER_ENABLED.lock().unwrap().get();
                crate::random_trainer::RandomTrainer::set_active(trainer_enabled);
                if trainer_enabled {
                    let current = get_lane_order_string();
                    let trainer = crate::random_trainer::RandomTrainer::get_lane_order();
                    if current != trainer {
                        crate::random_trainer::RandomTrainer::set_lane_order(&current);
                    }
                }

                let bw = BLACK_WHITE_RANDOM_PERMUTATION.lock().unwrap().get();
                crate::random_trainer::RandomTrainer::set_black_white_permute(bw);
            });
    }
}

fn change_lane_order(random: &str) {
    let mut lane_order = LANE_ORDER.lock().unwrap();
    let chars: Vec<char> = random.chars().collect();
    for i in 0..lane_order.len().min(chars.len()) {
        lane_order[i] = chars[i].to_string();
    }
}

fn get_lane_order_string() -> String {
    let lane_order = LANE_ORDER.lock().unwrap();
    lane_order.join("")
}
