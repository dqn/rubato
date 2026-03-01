// Translates: bms.player.beatoraja.launcher.DiscordConfigurationView

use beatoraja_core::config::Config;

/// Inner struct: WebhookInfo
/// Translates: DiscordConfigurationView.WebhookInfo (private static class)
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct WebhookInfo {
    // public StringProperty url;
    pub url: String,
}

#[allow(dead_code)]
impl WebhookInfo {
    // public WebhookInfo(String url)
    pub fn new(url: String) -> Self {
        WebhookInfo { url }
    }

    // public String getUrl()
    pub fn get_url(&self) -> &str {
        &self.url
    }

    // public void setUrl(String url)
    pub fn set_url(&mut self, url: String) {
        self.url = url;
    }

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
#[allow(dead_code)]
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

#[allow(dead_code)]
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
        self.discord_rich_presence = config.use_discord_rpc;
        // webhookName.setText(config.getWebhookName());
        self.webhook_name = config.webhook_name.clone();
        // webhookAvatar.setText(config.getWebhookAvatar());
        self.webhook_avatar = config.webhook_avatar.clone();
        // webhookOption.getSelectionModel().select(config.getWebhookOption());
        self.webhook_option = config.webhook_option;
        // WebhookInfo.populateList(webhookURL.getItems(), config.getWebhookUrl());
        WebhookInfo::populate_list(&mut self.webhook_url, &config.webhook_url);

        self.config = Some(config.clone());
    }

    // public void commit()
    pub fn commit(&mut self) {
        if let Some(ref mut config) = self.config {
            // config.setUseDiscordRPC(discordRichPresence.isSelected());
            config.use_discord_rpc = self.discord_rich_presence;
            // config.setWebhookOption(webhookOption.getSelectionModel().getSelectedIndex());
            config.webhook_option = self.webhook_option;
            // config.setWebhookName(webhookName.getText());
            config.webhook_name = self.webhook_name.clone();
            // config.setWebhookAvatar(webhookAvatar.getText());
            config.webhook_avatar = self.webhook_avatar.clone();
            // config.setWebhookUrl(WebhookInfo.toURLArray(webhookURL.getItems()));
            config.webhook_url = WebhookInfo::to_url_array(&self.webhook_url);
        }
    }

    // @FXML public void addWebhookURL()
    pub fn add_webhook_url(&mut self) {
        // String s = url.getText();
        let s = self.url.clone();
        // boolean find = webhookURL.getItems().stream().anyMatch(url -> url.getUrl().equals(s));
        let find = self.webhook_url.iter().any(|w| w.get_url() == s);
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
}
