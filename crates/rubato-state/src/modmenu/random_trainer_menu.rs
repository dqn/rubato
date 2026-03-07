use super::random_trainer::RandomTrainer;

use std::sync::Mutex;

static RANDOM_TRAINER_ENABLED: Mutex<bool> = Mutex::new(false);
static BLACK_WHITE_RANDOM_PERMUTATION: Mutex<bool> = Mutex::new(false);
static LANE_ORDER: Mutex<Vec<String>> = Mutex::new(Vec::new());
static TRACK_RAN_WHEN_DISABLED: Mutex<bool> = Mutex::new(false);

fn init_lane_order() {
    let mut lo = LANE_ORDER.lock().expect("LANE_ORDER lock poisoned");
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
    #[allow(dead_code)]
    fn random_history() {
        // if (ImGui.treeNode("Random History"))
        {
            let history = RandomTrainer::random_history();
            for entry in &history {
                let _title = entry.title();
                let _random = entry.random();
                // Render table rows
                // Double click to select as current random
            }
        }
        // ImGui.treePop();
    }

    #[allow(dead_code)]
    fn drag_and_drop_key_display() {
        let lane_order = LANE_ORDER.lock().expect("LANE_ORDER lock poisoned");
        let bw_permute = *BLACK_WHITE_RANDOM_PERMUTATION
            .lock()
            .expect("BLACK_WHITE_RANDOM_PERMUTATION lock poisoned");

        for lane in lane_order.iter() {
            let lane_char = lane.chars().next().unwrap_or('1');
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
                    let lane_order = LANE_ORDER.lock().expect("LANE_ORDER lock poisoned");
                    for lane_item in lane_order.iter() {
                        let lane_char = lane_item.chars().next().unwrap_or('1');
                        let is_random =
                            crate::modmenu::random_trainer::RandomTrainer::is_lane_to_random(
                                lane_char,
                            );
                        let label = if is_random {
                            "?".to_string()
                        } else {
                            lane_item.clone()
                        };
                        let color = if is_random {
                            egui::Color32::from_rgb(255, 100, 150) // pink
                        } else if lane_char.to_digit(10).unwrap_or(0).is_multiple_of(2) {
                            egui::Color32::from_rgb(50, 50, 150) // dark blue
                        } else {
                            egui::Color32::from_rgb(200, 200, 200) // light
                        };
                        let btn =
                            egui::Button::new(egui::RichText::new(&label).size(18.0).color(color))
                                .min_size(egui::vec2(40.0, 60.0));
                        ui.add(btn);
                    }
                });

                // Controls
                ui.separator();
                ui.label("Controls");
                ui.indent("random_controls", |ui| {
                    let mut enabled = RANDOM_TRAINER_ENABLED
                        .lock()
                        .expect("RANDOM_TRAINER_ENABLED lock poisoned");
                    ui.checkbox(&mut enabled, "Trainer Enabled");
                    drop(enabled);

                    let mut track = TRACK_RAN_WHEN_DISABLED
                        .lock()
                        .expect("TRACK_RAN_WHEN_DISABLED lock poisoned");
                    ui.checkbox(&mut track, "Track Current Random");
                    drop(track);

                    let mut bw = BLACK_WHITE_RANDOM_PERMUTATION
                        .lock()
                        .expect("BLACK_WHITE_RANDOM_PERMUTATION lock poisoned");
                    ui.checkbox(&mut bw, "Black/White Random Select");
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
                let trainer_enabled = *RANDOM_TRAINER_ENABLED
                    .lock()
                    .expect("RANDOM_TRAINER_ENABLED lock poisoned");
                crate::modmenu::random_trainer::RandomTrainer::set_active(trainer_enabled);
                if trainer_enabled {
                    let current = get_lane_order_string();
                    let trainer = crate::modmenu::random_trainer::RandomTrainer::lane_order();
                    if current != trainer {
                        crate::modmenu::random_trainer::RandomTrainer::set_lane_order(&current);
                    }
                }

                let bw = *BLACK_WHITE_RANDOM_PERMUTATION
                    .lock()
                    .expect("BLACK_WHITE_RANDOM_PERMUTATION lock poisoned");
                crate::modmenu::random_trainer::RandomTrainer::set_black_white_permute(bw);
            });
    }
}

fn change_lane_order(random: &str) {
    let mut lane_order = LANE_ORDER.lock().expect("LANE_ORDER lock poisoned");
    let chars: Vec<char> = random.chars().collect();
    for (slot, &ch) in lane_order.iter_mut().zip(chars.iter()) {
        *slot = ch.to_string();
    }
}

fn get_lane_order_string() -> String {
    let lane_order = LANE_ORDER.lock().expect("LANE_ORDER lock poisoned");
    lane_order.join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Guard to serialize tests that share static LANE_ORDER.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    /// Set lane order to a known state for testing (must hold no locks when calling).
    fn setup_lane_order(order: &str) {
        let mut lo = LANE_ORDER.lock().unwrap();
        *lo = order.chars().map(|c| c.to_string()).collect();
    }

    #[test]
    fn test_mirror_lane_order() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        RandomTrainerMenu::mirror_lane_order();
        assert_eq!(get_lane_order_string(), "7654321");
    }

    #[test]
    fn test_mirror_lane_order_already_reversed() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("7654321");
        RandomTrainerMenu::mirror_lane_order();
        assert_eq!(get_lane_order_string(), "1234567");
    }

    #[test]
    fn test_shift_left_lane_order() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        RandomTrainerMenu::shift_left_lane_order();
        assert_eq!(get_lane_order_string(), "2345671");
    }

    #[test]
    fn test_shift_left_lane_order_twice() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        RandomTrainerMenu::shift_left_lane_order();
        RandomTrainerMenu::shift_left_lane_order();
        assert_eq!(get_lane_order_string(), "3456712");
    }

    #[test]
    fn test_shift_right_lane_order() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        RandomTrainerMenu::shift_right_lane_order();
        assert_eq!(get_lane_order_string(), "7123456");
    }

    #[test]
    fn test_shift_right_lane_order_twice() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        RandomTrainerMenu::shift_right_lane_order();
        RandomTrainerMenu::shift_right_lane_order();
        assert_eq!(get_lane_order_string(), "6712345");
    }

    #[test]
    fn test_shift_left_then_right_is_identity() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        RandomTrainerMenu::shift_left_lane_order();
        RandomTrainerMenu::shift_right_lane_order();
        assert_eq!(get_lane_order_string(), "1234567");
    }

    #[test]
    fn test_change_lane_order_partial() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("1234567");
        change_lane_order("ABC");
        // Only first 3 characters should change
        assert_eq!(get_lane_order_string(), "ABC4567");
    }

    #[test]
    fn test_get_lane_order_string() {
        let _g = TEST_LOCK.lock().unwrap();
        setup_lane_order("3571246");
        assert_eq!(get_lane_order_string(), "3571246");
    }

    #[test]
    fn test_init_lane_order_sets_default() {
        let _g = TEST_LOCK.lock().unwrap();
        // Clear lane order to force init
        {
            let mut lo = LANE_ORDER.lock().unwrap();
            lo.clear();
        }
        init_lane_order();
        assert_eq!(get_lane_order_string(), "1234567");
    }
}
