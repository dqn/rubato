// Translated from TrainerView.java

use egui;
use log::warn;
use rubato_core::player_config::PlayerConfig;
use rubato_state::modmenu::random_trainer::RandomTrainer;

/// TrainerView — Random trainer UI.
/// Java: TrainerView with @FXML CheckBox traineractive, Button setbutton, TextField laneorder.
/// In Rust, this is a data struct — egui rendering is deferred.
#[derive(Clone, Debug, Default)]
pub struct TrainerView {
    // Java: @FXML private CheckBox traineractive;
    pub trainer_active: bool,
    // Java: @FXML private Button setbutton;
    // Button state — no data needed, handled in egui rendering
    // Java: @FXML private TextField laneorder;
    pub lane_order: String,
    // Java: @FXML private ListView<String> history; (commented out in Java)
    // Java: private PlayerConfig player;
    pub player: Option<PlayerConfig>,
}

impl TrainerView {
    /// Creates a new TrainerView.
    pub fn new() -> Self {
        TrainerView {
            trainer_active: false,
            lane_order: String::new(),
            player: None,
        }
    }

    /// Updates the view with a PlayerConfig.
    /// Java: public void update(PlayerConfig player)
    pub fn update(&mut self, player: Option<PlayerConfig>) {
        // Java: this.player = player;
        self.player = player;
        // Java: RandomTrainer randomtrainer = new RandomTrainer();
        let _randomtrainer = RandomTrainer::new();
        // Java: if(this.player == null) { return; }
        if self.player.is_none() {
            return;
        }
        // Java: randomtrainer.setActive(false);
        RandomTrainer::set_active(false);
        // Java: traineractive.setSelected(randomtrainer.isActive());
        self.trainer_active = RandomTrainer::is_active();
        // Java: laneorder.setPromptText("1234567");
        // Prompt text is a UI hint — stored but deferred to egui rendering
        // Java: if (randomtrainer.getLaneOrder() != null)
        let lane_order = RandomTrainer::lane_order();
        if !lane_order.is_empty() {
            // Java: laneorder.setText(randomtrainer.getLaneOrder());
            self.lane_order = lane_order;
        } else {
            // Java: randomtrainer.setLaneOrder("1234567");
            RandomTrainer::set_lane_order("1234567");
        }
    }

    /// Sets the active state of the random trainer.
    /// Java: @FXML public void setActive()
    /// Java: RandomTrainer.setActive(traineractive.isSelected());
    pub fn set_active(&self) {
        RandomTrainer::set_active(self.trainer_active);
    }

    // Java: @FXML public void fromHistory() { } (commented out in Java)

    /// Sets the random lane order.
    /// Java: @FXML public void setRandom()
    pub fn set_random(&self) {
        // Java: RandomTrainer randomtrainer = new RandomTrainer();
        let _randomtrainer = RandomTrainer::new();
        // Java: if (this.laneorder == null)
        if self.lane_order.is_empty() {
            // Java: logger.warn("RandomTrainer: Lane field empty");
            warn!("RandomTrainer: Lane field empty");
            return;
        }

        // Java: int[] lanes = this.laneorder.getCharacters().codePoints()
        //     .map(Character::getNumericValue).map(c -> c-1).toArray();
        let lanes: Vec<i32> = self
            .lane_order
            .chars()
            .map(|c| c.to_digit(10).unwrap_or(0) as i32 - 1)
            .collect();

        // Java: int[] has_all = new int[]{0,1,2,3,4,5,6};
        let mut has_all: Vec<i32> = vec![0, 1, 2, 3, 4, 5, 6];
        // Java: Arrays.sort(has_all);
        has_all.sort();
        // Java: int[] l = lanes.clone();
        let mut l = lanes.clone();
        // Java: Arrays.sort(l);
        l.sort();

        // Java: if (l.length != 7)
        if l.len() != 7 {
            // Java: logger.warn("RandomTrainer: Incorrect number of lanes specified");
            warn!("RandomTrainer: Incorrect number of lanes specified");
            return;
        }

        // Java: for (int i = 0; i < has_all.length; i++)
        for i in 0..has_all.len() {
            // Java: if (l[i] != has_all[i])
            if l[i] != has_all[i] {
                // Java: logger.warn("RandomTrainer: Lanes in incorrect format, falling back to nonran or last ran used");
                warn!(
                    "RandomTrainer: Lanes in incorrect format, falling back to nonran or last ran used"
                );
                return;
            }
        }

        // Java: randomtrainer.setLaneOrder(this.laneorder.getCharacters().toString());
        RandomTrainer::set_lane_order(&self.lane_order);
    }

    /// Render the random trainer UI.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        egui::Grid::new("trainer_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Active:");
                if ui.checkbox(&mut self.trainer_active, "").changed() {
                    self.set_active();
                }
                ui.end_row();

                ui.label("Lane Order:");
                ui.text_edit_singleline(&mut self.lane_order);
                ui.end_row();
            });

        if ui.button("Set").clicked() {
            self.set_random();
        }
    }
}
