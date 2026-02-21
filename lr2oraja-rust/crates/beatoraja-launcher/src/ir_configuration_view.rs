// Translates: bms.player.beatoraja.launcher.IRConfigurationView

use std::collections::HashMap;

use beatoraja_core::ir_config::IRConfig;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_ir::ir_connection_manager::IRConnectionManager;

use crate::stubs::open_url_in_browser;

/// Translates: IRConfigurationView (JavaFX → egui)
///
/// IR connection configuration UI with user/password fields,
/// send options, and primary IR selection.
#[allow(dead_code)]
pub struct IRConfigurationView {
    // @FXML private Button primarybutton;
    primarybutton_visible: bool,
    // @FXML private ComboBox<String> irname;
    irname: Option<String>,
    irname_items: Vec<String>,
    // @FXML private Hyperlink irhome;
    irhome: String,
    // @FXML private TextField iruserid;
    iruserid: String,
    // @FXML private PasswordField irpassword;
    irpassword: String,
    // @FXML private ComboBox<Integer> irsend;
    irsend: Option<i32>,
    irsend_items: Vec<i32>,
    // @FXML private CheckBox importrival;
    importrival: bool,
    // @FXML private CheckBox importscore;
    importscore: bool,

    // private Map<String, IRConfig> irmap = new HashMap<String, IRConfig>();
    irmap: HashMap<String, IRConfig>,
    // private String primary;
    primary: Option<String>,
    // private IRConfig currentir;
    currentir: Option<IRConfig>,

    // private PlayerConfig player;
    player: Option<PlayerConfig>,
}

impl Default for IRConfigurationView {
    fn default() -> Self {
        IRConfigurationView {
            primarybutton_visible: false,
            irname: None,
            irname_items: Vec::new(),
            irhome: String::new(),
            iruserid: String::new(),
            irpassword: String::new(),
            irsend: None,
            irsend_items: Vec::new(),
            importrival: false,
            importscore: false,
            irmap: HashMap::new(),
            primary: None,
            currentir: None,
            player: None,
        }
    }

}

#[allow(dead_code)]
impl IRConfigurationView {
    pub fn new() -> Self {
        Self::default()
    }

    // private void initComboBox(ComboBox<Integer> combo, final String[] values)
    // (This is a UI helper — in Rust we just store the values)

    // public void initialize(URL arg0, ResourceBundle arg1)
    pub fn initialize(&mut self) {
        // initComboBox(irsend, new String[] { arg1.getString("IR_SEND_ALWAYS"), arg1.getString("IR_SEND_FINISH"), arg1.getString("IR_SEND_UPDATE")});
        // In Rust, we store the integer indices; label strings are for egui rendering
        self.irsend_items = vec![0, 1, 2];

        // irname.getItems().setAll(IRConnectionManager.getAllAvailableIRConnectionName());
        self.irname_items = IRConnectionManager::get_all_available_ir_connection_name();
    }

    // public void update(PlayerConfig player)
    pub fn update(&mut self, player: &mut PlayerConfig) {
        // this.player = player;
        self.player = Some(player.clone());

        // for(IRConfig ir : player.getIrconfig()) {
        for ir_opt in &player.irconfig {
            if let Some(ir) = ir_opt {
                // irmap.put(ir.getIrname(), ir);
                self.irmap.insert(ir.irname.clone(), ir.clone());
            }
        }

        // primary = player.getIrconfig().length > 0 ? player.getIrconfig()[0].getIrname() : null;
        self.primary = if !player.irconfig.is_empty() {
            player.irconfig[0].as_ref().map(|ir| ir.irname.clone())
        } else {
            None
        };

        // if(!irname.getItems().contains(primary)) {
        let primary_contained = if let Some(ref p) = self.primary {
            self.irname_items.contains(p)
        } else {
            false
        };
        if !primary_contained {
            // if (irname.getItems().size() == 0) {
            if self.irname_items.is_empty() {
                // primary = null;
                self.primary = None;
            } else {
                // primary = irname.getItems().get(0);
                self.primary = Some(self.irname_items[0].clone());
            }
        }

        // irname.setValue(primary);
        self.irname = self.primary.clone();
        // updateIRConnection();
        self.update_ir_connection();
    }

    // public void commit()
    pub fn commit(&mut self) {
        // updateIRConnection();
        self.update_ir_connection();

        // List<IRConfig> irlist = new ArrayList<IRConfig>();
        let mut irlist: Vec<IRConfig> = Vec::new();

        // for(String s : irname.getItems()) {
        for s in &self.irname_items {
            // IRConfig ir = irmap.get(s);
            if let Some(ir) = self.irmap.get(s) {
                // if(ir != null && ir.getUserid().length() > 0) {
                if !ir.get_userid().is_empty() {
                    // if(s.equals(primary) ) {
                    if Some(s) == self.primary.as_ref() {
                        // irlist.add(0, ir);
                        irlist.insert(0, ir.clone());
                    } else {
                        // irlist.add(ir);
                        irlist.push(ir.clone());
                    }
                }
            }
        }

        // player.setIrconfig(irlist.toArray(new IRConfig[irlist.size()]));
        if let Some(ref mut player) = self.player {
            player.irconfig = irlist.into_iter().map(Some).collect();
        }
    }

    // @FXML public void setPrimary()
    pub fn set_primary(&mut self) {
        // primary = irname.getValue();
        self.primary = self.irname.clone();
        // updateIRConnection();
        self.update_ir_connection();
    }

    // @FXML public void updateIRConnection()
    pub fn update_ir_connection(&mut self) {
        // if(currentir != null) {
        if let Some(ref mut currentir) = self.currentir {
            // currentir.setUserid(iruserid.getText());
            currentir.set_userid(self.iruserid.clone());
            // currentir.setPassword(irpassword.getText());
            currentir.set_password(self.irpassword.clone());
            // currentir.setIrsend(irsend.getValue());
            currentir.irsend = self.irsend.unwrap_or(0);
            // currentir.setImportscore(importscore.isSelected());
            currentir.importscore = self.importscore;
            // currentir.setImportrival(importrival.isSelected());
            currentir.importrival = self.importrival;

            // Write back to irmap
            let irname = currentir.irname.clone();
            let updated = currentir.clone();
            self.irmap.insert(irname, updated);
        }
        self.currentir = None;

        // String homeurl = IRConnectionManager.getHomeURL(irname.getValue());
        let current_name = self.irname.clone().unwrap_or_default();
        let homeurl = IRConnectionManager::get_home_url(&current_name).unwrap_or_default();
        // irhome.setText(homeurl);
        self.irhome = homeurl.clone();
        // irhome.setOnAction((event) -> {
        //     Desktop desktop = Desktop.getDesktop();
        //     URI uri;
        //     try {
        //         uri = new URI(homeurl);
        //         desktop.browse(uri);
        //     } catch (Exception e) {
        //         logger.warn("最新版URLアクセス時例外:{}", e.getMessage());
        //     }
        // });
        // (On-click action: call open_url_in_browser — deferred to egui integration)
        let _ = homeurl; // homeurl captured for on-click handler

        // if(!irmap.containsKey(irname.getValue())) {
        if !self.irmap.contains_key(&current_name) {
            // IRConfig ir = new IRConfig();
            let mut ir = IRConfig::default();
            // ir.setIrname(irname.getValue());
            ir.irname = current_name.clone();
            // irmap.put(irname.getValue(), ir);
            self.irmap.insert(current_name.clone(), ir);
        }

        // currentir = irmap.get(irname.getValue());
        let ir = self.irmap.get(&current_name).cloned();
        if let Some(ref ir) = ir {
            // iruserid.setText(currentir.getUserid());
            self.iruserid = ir.get_userid();
            // irpassword.setText(currentir.getPassword());
            self.irpassword = ir.get_password();
            // irsend.setValue(currentir.getIrsend());
            self.irsend = Some(ir.irsend);
            // importscore.setSelected(currentir.isImportscore());
            self.importscore = ir.importscore;
            // importrival.setSelected(currentir.isImportrival());
            self.importrival = ir.importrival;
        }
        self.currentir = ir;

        // primarybutton.setVisible(!(primary != null && irname.getValue().equals(primary)));
        self.primarybutton_visible =
            !(self.primary.is_some() && self.irname.as_ref() == self.primary.as_ref());
    }

    /// Opens the IR home URL in the browser (called from egui click handler)
    pub fn open_home_url(&self) {
        if !self.irhome.is_empty() {
            open_url_in_browser(&self.irhome);
        }
    }
}
