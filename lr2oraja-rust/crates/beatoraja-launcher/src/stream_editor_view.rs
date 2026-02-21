// Translates: bms.player.beatoraja.launcher.StreamEditorView

use beatoraja_core::player_config::PlayerConfig;

/// Translates: StreamEditorView (JavaFX → egui)
///
/// Stream request configuration UI: enable, notify, max count.
#[allow(dead_code)]
pub struct StreamEditorView {
    // @FXML private CheckBox enableRequest;
    enable_request: bool,
    // @FXML private CheckBox notifyRequest;
    notify_request: bool,
    // @FXML private Spinner<Integer> maxRequestCount;
    max_request_count: i32,

    // private PlayerConfig player;
    player: Option<PlayerConfig>,
}

impl Default for StreamEditorView {
    fn default() -> Self {
        StreamEditorView {
            enable_request: false,
            notify_request: false,
            max_request_count: 0,
            player: None,
        }
    }
}

#[allow(dead_code)]
impl StreamEditorView {
    // public void initialize(URL arg0, ResourceBundle arg1)
    pub fn initialize(&mut self) {
        // (empty in Java)
    }

    // public void update(PlayerConfig player)
    pub fn update(&mut self, player: &PlayerConfig) {
        // this.player = player;
        self.player = Some(player.clone());
        // if(this.player == null) { return; }
        // (In Rust, this is handled by Option)

        // enableRequest.setSelected(this.player.getRequestEnable());
        self.enable_request = player.enable_request;
        // notifyRequest.setSelected(this.player.getRequestNotify());
        self.notify_request = player.notify_request;
        // maxRequestCount.getValueFactory().setValue(this.player.getMaxRequestCount());
        self.max_request_count = player.max_request_count;
    }

    // public void commit()
    pub fn commit(&mut self) {
        // if(this.player == null) { return; }
        if let Some(ref mut player) = self.player {
            // player.setRequestEnable(enableRequest.isSelected());
            player.enable_request = self.enable_request;
            // player.setRequestNotify(notifyRequest.isSelected());
            player.notify_request = self.notify_request;
            // player.setMaxRequestCount(maxRequestCount.getValue());
            player.max_request_count = self.max_request_count;
        }
    }
}
