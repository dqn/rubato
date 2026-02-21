use crate::stubs::IRConnectionManager;
use crate::validatable::Validatable;

const KEY: &str = "0123456789abcdef";

pub const IR_SEND_ALWAYS: i32 = 0;
pub const IR_SEND_COMPLETE_SONG: i32 = 1;
pub const IR_SEND_UPDATE_SCORE: i32 = 2;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct IRConfig {
    pub irname: String,
    pub userid: String,
    pub cuserid: String,
    pub password: String,
    pub cpassword: String,
    pub irsend: i32,
    pub importscore: bool,
    pub importrival: bool,
}

impl Default for IRConfig {
    fn default() -> Self {
        IRConfig {
            irname: String::new(),
            userid: String::new(),
            cuserid: String::new(),
            password: String::new(),
            cpassword: String::new(),
            irsend: 0,
            importscore: false,
            importrival: true,
        }
    }
}

impl IRConfig {
    pub fn get_userid(&self) -> String {
        if !self.cuserid.is_empty() {
            match cipher_decrypt(&self.cuserid, KEY) {
                Ok(decrypted) => return decrypted,
                Err(e) => {
                    log::error!("Failed to decrypt userid: {}", e);
                }
            }
        }
        self.userid.clone()
    }

    pub fn set_userid(&mut self, userid: String) {
        self.userid = userid.clone();
        if userid.is_empty() {
            self.cuserid = String::new();
        }
        self.validate();
    }

    pub fn get_password(&self) -> String {
        if !self.cpassword.is_empty() {
            match cipher_decrypt(&self.cpassword, KEY) {
                Ok(decrypted) => return decrypted,
                Err(e) => {
                    log::error!("Failed to decrypt password: {}", e);
                }
            }
        }
        self.password.clone()
    }

    pub fn set_password(&mut self, password: String) {
        self.password = password.clone();
        if password.is_empty() {
            self.cpassword = String::new();
        }
        self.validate();
    }

    pub fn get_irsend(&self) -> i32 {
        self.irsend
    }

    pub fn get_irname(&self) -> &str {
        &self.irname
    }
}

impl Validatable for IRConfig {
    fn validate(&mut self) -> bool {
        if self.irname.is_empty()
            || IRConnectionManager::get_ir_connection_class(&self.irname).is_none()
        {
            return false;
        }

        if !self.userid.is_empty() {
            match cipher_encrypt(&self.userid, KEY) {
                Ok(encrypted) => {
                    self.cuserid = encrypted;
                    self.userid = String::new();
                }
                Err(e) => {
                    log::error!("Failed to encrypt userid: {}", e);
                }
            }
        }
        if !self.password.is_empty() {
            match cipher_encrypt(&self.password, KEY) {
                Ok(encrypted) => {
                    self.cpassword = encrypted;
                    self.password = String::new();
                }
                Err(e) => {
                    log::error!("Failed to encrypt password: {}", e);
                }
            }
        }
        true
    }
}

// CipherUtils - AES encryption/decryption
// TODO: Implement actual AES-ECB encryption when crypto library is integrated
fn cipher_encrypt(_source: &str, _key: &str) -> anyhow::Result<String> {
    todo!("AES encryption not yet implemented")
}

fn cipher_decrypt(_encrypt_source: &str, _key: &str) -> anyhow::Result<String> {
    todo!("AES decryption not yet implemented")
}
