// Translates: bms.player.beatoraja.launcher.DiscordConfigurationView

use egui;
use rubato_core::config::Config;

/// Inner struct: WebhookInfo
/// Translates: DiscordConfigurationView.WebhookInfo (private static class)
#[derive(Clone, Debug)]
pub struct WebhookInfo {
    // public StringProperty url;
    pub url: String,
}

impl WebhookInfo {
    // public WebhookInfo(String url)
    pub fn new(url: String) -> Self {
        WebhookInfo { url }
    }

    // public void setUrl(String url)
    // public StringProperty urlProperty()
    // In Rust, the field is accessed directly — no JavaFX property wrapper needed.

    // public static String[] toURLArray(List<WebhookInfo> webhooks)
    pub fn to_url_array(webhooks: &[WebhookInfo]) -> Vec<String> {
        // return webhooks.stream().map(WebhookInfo::getUrl).toArray(String[]::new);
        webhooks.iter().map(|w| w.url.clone()).collect()
    }

    // public static void populateList(List<WebhookInfo> webhooks, String[] urls)
    pub fn populate_list(webhooks: &mut Vec<WebhookInfo>, urls: &[String]) {
        // webhooks.clear();
        webhooks.clear();
        // for (String url : urls) {
        for url in urls {
            // webhooks.add(new WebhookInfo(url));
            webhooks.push(WebhookInfo::new(url.clone()));
        }
    }
}

/// Translates: DiscordConfigurationView (JavaFX → egui)
///
/// Discord/webhook configuration UI with editable table for webhook URLs.
#[derive(Default)]
pub struct DiscordConfigurationView {
    // @FXML public CheckBox discordRichPresence;
    pub discord_rich_presence: bool,
    // @FXML private ComboBox<String> webhookOption;
    webhook_option: i32,
    // @FXML private TextField webhookName;
    webhook_name: String,
    // @FXML private TextField webhookAvatar;
    webhook_avatar: String,
    // @FXML private TextField url;
    url: String,
    // @FXML private EditableTableView<WebhookInfo> webhookURL;
    webhook_url: Vec<WebhookInfo>,
    webhook_url_selected_indices: Vec<usize>,

    config: Option<Config>,
}

impl DiscordConfigurationView {
    // public void initialize(URL location, ResourceBundle resources)
    pub fn initialize(&mut self) {
        // (empty in Java)
    }

    // public void init(PlayConfigurationView main)
    pub fn init(&mut self) {
        // TableColumn<WebhookInfo, String> urlColumn = new TableColumn("Discord WebHook URL");
        // urlColumn.setCellValueFactory(p -> p.getValue().urlProperty());
        // urlColumn.setSortable(false);
        // urlColumn.setMinWidth(510);
        // urlColumn.setMinWidth(0);
        // webhookURL.getColumns().setAll(urlColumn);
        // webhookURL.getSelectionModel().setSelectionMode(SelectionMode.MULTIPLE);
        // Table column setup deferred to egui integration
        // egui: table columns defined during render() — no JavaFX-style pre-initialization needed
    }

    // public void update(Config config)
    pub fn update(&mut self, config: &mut Config) {
        // this.config = config;
        // discordRichPresence.setSelected(config.isUseDiscordRPC());
        self.discord_rich_presence = config.integration.use_discord_rpc;
        // webhookName.setText(config.getWebhookName());
        self.webhook_name = config.integration.webhook_name.clone();
        // webhookAvatar.setText(config.getWebhookAvatar());
        self.webhook_avatar = config.integration.webhook_avatar.clone();
        // webhookOption.getSelectionModel().select(config.getWebhookOption());
        self.webhook_option = config.integration.webhook_option;
        // WebhookInfo.populateList(webhookURL.getItems(), config.getWebhookUrl());
        WebhookInfo::populate_list(&mut self.webhook_url, &config.integration.webhook_url);

        self.config = Some(config.clone());
    }

    // public void commit()
    pub fn commit(&mut self) {
        if let Some(ref mut config) = self.config {
            // config.setUseDiscordRPC(discordRichPresence.isSelected());
            config.integration.use_discord_rpc = self.discord_rich_presence;
            // config.setWebhookOption(webhookOption.getSelectionModel().getSelectedIndex());
            config.integration.webhook_option = self.webhook_option;
            // config.setWebhookName(webhookName.getText());
            config.integration.webhook_name = self.webhook_name.clone();
            // config.setWebhookAvatar(webhookAvatar.getText());
            config.integration.webhook_avatar = self.webhook_avatar.clone();
            // config.setWebhookUrl(WebhookInfo.toURLArray(webhookURL.getItems()));
            config.integration.webhook_url = WebhookInfo::to_url_array(&self.webhook_url);
        }
    }

    // @FXML public void addWebhookURL()
    pub fn add_webhook_url(&mut self) {
        // String s = url.getText();
        let s = self.url.clone();
        // boolean find = webhookURL.getItems().stream().anyMatch(url -> url.getUrl().equals(s));
        let find = self.webhook_url.iter().any(|w| w.url == s);
        // if (!find) {
        if !find {
            // webhookURL.addItem(new WebhookInfo(url.getText()));
            self.webhook_url.push(WebhookInfo::new(self.url.clone()));
        }
    }

    // @FXML public void removeWebhookURL()
    pub fn remove_webhook_url(&mut self) {
        // webhookURL.removeSelectedItems();
        let mut indices = self.webhook_url_selected_indices.clone();
        indices.sort_unstable();
        indices.reverse();
        for idx in indices {
            if idx < self.webhook_url.len() {
                self.webhook_url.remove(idx);
            }
        }
        self.webhook_url_selected_indices.clear();
    }

    // @FXML public void moveWebhookURLUp()
    pub fn move_webhook_url_up(&mut self) {
        // webhookURL.moveSelectedItemsUp();
        let mut indices = self.webhook_url_selected_indices.clone();
        if indices.is_empty() {
            return;
        }
        indices.sort_unstable();
        let mut last_block_index = 0usize;
        let len = indices.len();
        for i in 1..=len {
            if i == len || indices[i] > indices[i - 1] + 1 {
                if indices[last_block_index] > 0 {
                    let item = self.webhook_url.remove(indices[last_block_index] - 1);
                    self.webhook_url.insert(indices[i - 1], item);
                }
                last_block_index = i;
            }
        }
    }

    // @FXML public void moveWebhookURLDown()
    pub fn move_webhook_url_down(&mut self) {
        // webhookURL.moveSelectedItemsDown();
        let mut indices = self.webhook_url_selected_indices.clone();
        if indices.is_empty() {
            return;
        }
        indices.sort_unstable();
        let num_items = self.webhook_url.len();
        let len = indices.len();
        let mut last_block_index = len - 1;
        let mut i = len as i32 - 2;
        while i >= -1 {
            let iu = i as usize;
            if i == -1 || indices[iu] < indices[iu + 1] - 1 {
                if indices[last_block_index] < num_items - 1 {
                    let item = self.webhook_url.remove(indices[last_block_index] + 1);
                    self.webhook_url.insert(indices[(i + 1) as usize], item);
                }
                last_block_index = iu;
            }
            i -= 1;
        }
    }

    /// Render the Discord configuration UI.
    ///
    /// Shows Discord Rich Presence toggle, webhook settings (option, name, avatar),
    /// and an editable webhook URL list with add/remove/reorder controls.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading("Discord Configuration");

        ui.checkbox(
            &mut self.discord_rich_presence,
            "Enable Discord Rich Presence",
        );

        ui.separator();
        ui.heading("Webhook");

        let webhook_options = ["All Clear", "FC / AAA", "Clear"];

        egui::Grid::new("discord_config_grid")
            .num_columns(2)
            .show(ui, |ui| {
                ui.label("Send On:");
                let clamped_index = crate::launcher_ui::tabs::clamped_option_index(
                    self.webhook_option,
                    webhook_options.len(),
                );
                let selected_label = webhook_options[clamped_index];
                egui::ComboBox::from_id_salt("discord_config_webhook_option")
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for (i, label) in webhook_options.iter().enumerate() {
                            ui.selectable_value(&mut self.webhook_option, i as i32, *label);
                        }
                    });
                ui.end_row();

                ui.label("Bot Name:");
                ui.text_edit_singleline(&mut self.webhook_name);
                ui.end_row();

                ui.label("Avatar URL:");
                ui.text_edit_singleline(&mut self.webhook_avatar);
                ui.end_row();
            });

        ui.separator();
        ui.label("Webhook URLs:");

        // Webhook URL list with selection
        let mut remove_indices: Vec<usize> = Vec::new();
        for (i, webhook) in self.webhook_url.iter().enumerate() {
            let selected = self.webhook_url_selected_indices.contains(&i);
            ui.horizontal(|ui| {
                if ui.selectable_label(selected, &webhook.url).clicked() {
                    if selected {
                        self.webhook_url_selected_indices.retain(|&idx| idx != i);
                    } else {
                        self.webhook_url_selected_indices.push(i);
                    }
                }
                if ui.small_button("x").clicked() {
                    remove_indices.push(i);
                }
            });
        }
        // Remove in reverse order to preserve indices
        remove_indices.sort_unstable();
        for idx in remove_indices.into_iter().rev() {
            if idx < self.webhook_url.len() {
                self.webhook_url.remove(idx);
            }
            self.webhook_url_selected_indices.retain(|&i| i != idx);
        }

        // Add URL input
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.url);
            if ui.button("Add").clicked() {
                self.add_webhook_url();
            }
        });

        // Reorder / remove buttons
        ui.horizontal(|ui| {
            if ui.button("Up").clicked() {
                self.move_webhook_url_up();
            }
            if ui.button("Down").clicked() {
                self.move_webhook_url_down();
            }
            if ui.button("Remove Selected").clicked() {
                self.remove_webhook_url();
            }
        });
    }
}
