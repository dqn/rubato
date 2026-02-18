use aes::Aes128;
use aes::cipher::{BlockDecryptMut, KeyInit, block_padding::Pkcs7};
use aes_gcm::aead::{Aead, KeyInit as AeadKeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub const IR_SEND_ALWAYS: i32 = 0;
pub const IR_SEND_COMPLETE_SONG: i32 = 1;
pub const IR_SEND_UPDATE_SCORE: i32 = 2;

/// Internet Ranking configuration.
///
/// The `cuserid` and `cpassword` fields hold AES-encrypted values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

type LegacyAes128EcbDecryptor = ecb::Decryptor<Aes128>;

const LEGACY_IR_CONFIG_AES_KEY: &[u8; 16] = b"0123456789abcdef";
const CREDENTIAL_V2_PREFIX: &str = "v2:";
const CREDENTIAL_NONCE_SIZE: usize = 12;

impl Default for IRConfig {
    fn default() -> Self {
        Self {
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
    /// Validates this IR config. Returns false if irname is empty.
    pub fn validate(&self) -> bool {
        !self.irname.is_empty()
    }

    /// Encrypt plaintext credentials into `cuserid` / `cpassword` and
    /// clear plaintext fields before writing to disk.
    pub fn sanitize_credentials_for_write(&mut self) {
        if !self.userid.is_empty()
            && let Some(encrypted) = encrypt_credential(&self.userid)
        {
            self.cuserid = encrypted;
            self.userid.clear();
        }

        if !self.password.is_empty()
            && let Some(encrypted) = encrypt_credential(&self.password)
        {
            self.cpassword = encrypted;
            self.password.clear();
        }
    }

    /// Decrypt `cuserid` / `cpassword` into plaintext fields for in-memory use.
    ///
    /// Keeps encrypted fields unchanged for backwards compatibility.
    pub fn hydrate_credentials_for_use(&mut self) {
        if self.userid.is_empty()
            && !self.cuserid.is_empty()
            && let Some(decrypted) = decrypt_credential(&self.cuserid)
        {
            self.userid = decrypted;
        }

        if self.password.is_empty()
            && !self.cpassword.is_empty()
            && let Some(decrypted) = decrypt_credential(&self.cpassword)
        {
            self.password = decrypted;
        }
    }
}

fn encrypt_credential(plaintext: &str) -> Option<String> {
    let cipher = <Aes256Gcm as AeadKeyInit>::new_from_slice(&credential_key()).ok()?;
    let mut nonce_bytes = [0_u8; CREDENTIAL_NONCE_SIZE];
    rand::rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = cipher.encrypt(nonce, plaintext.as_bytes()).ok()?;

    let mut payload = Vec::with_capacity(CREDENTIAL_NONCE_SIZE + encrypted.len());
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&encrypted);

    Some(format!(
        "{CREDENTIAL_V2_PREFIX}{}",
        BASE64_STANDARD.encode(payload)
    ))
}

fn decrypt_credential(ciphertext_b64: &str) -> Option<String> {
    if let Some(payload) = ciphertext_b64.strip_prefix(CREDENTIAL_V2_PREFIX) {
        return decrypt_v2_credential(payload);
    }

    decrypt_legacy_credential(ciphertext_b64).or_else(|| decrypt_v2_credential(ciphertext_b64))
}

fn decrypt_v2_credential(payload_b64: &str) -> Option<String> {
    let payload = BASE64_STANDARD.decode(payload_b64).ok()?;
    if payload.len() <= CREDENTIAL_NONCE_SIZE {
        return None;
    }

    let (nonce_bytes, ciphertext) = payload.split_at(CREDENTIAL_NONCE_SIZE);
    let cipher = <Aes256Gcm as AeadKeyInit>::new_from_slice(&credential_key()).ok()?;
    let decrypted = cipher
        .decrypt(Nonce::from_slice(nonce_bytes), ciphertext)
        .ok()?;
    String::from_utf8(decrypted).ok()
}

fn decrypt_legacy_credential(ciphertext_b64: &str) -> Option<String> {
    let mut ciphertext = BASE64_STANDARD.decode(ciphertext_b64).ok()?;
    let cipher = LegacyAes128EcbDecryptor::new_from_slice(LEGACY_IR_CONFIG_AES_KEY).ok()?;
    let decrypted = cipher.decrypt_padded_mut::<Pkcs7>(&mut ciphertext).ok()?;
    String::from_utf8(decrypted.to_vec()).ok()
}

fn credential_key() -> [u8; 32] {
    static KEY: OnceLock<[u8; 32]> = OnceLock::new();
    *KEY.get_or_init(derive_credential_key)
}

fn derive_credential_key() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"brs-ir-credential-key-v2");

    if let Ok(seed) = std::env::var("BRS_IR_KEY_SEED") {
        hasher.update(seed.as_bytes());
    }

    if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
        hasher.update(user.as_bytes());
    }

    if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
        hasher.update(home.as_bytes());
    }

    if let Ok(hostname) = std::env::var("HOSTNAME").or_else(|_| std::env::var("COMPUTERNAME")) {
        hasher.update(hostname.as_bytes());
    }

    #[cfg(not(target_os = "windows"))]
    {
        for machine_id_path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
            if let Ok(machine_id) = std::fs::read(machine_id_path) {
                hasher.update(machine_id);
                break;
            }
        }
    }

    let digest = hasher.finalize();
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let ir = IRConfig::default();
        assert!(ir.irname.is_empty());
        assert!(ir.userid.is_empty());
        assert!(ir.cuserid.is_empty());
        assert!(ir.password.is_empty());
        assert!(ir.cpassword.is_empty());
        assert_eq!(ir.irsend, 0);
        assert!(!ir.importscore);
        assert!(ir.importrival);
    }

    #[test]
    fn test_validate_empty_irname() {
        let ir = IRConfig::default();
        assert!(!ir.validate());
    }

    #[test]
    fn test_validate_with_irname() {
        let ir = IRConfig {
            irname: "LR2IR".to_string(),
            ..Default::default()
        };
        assert!(ir.validate());
    }

    #[test]
    fn test_sanitize_credentials_for_write_encrypts_plaintext() {
        let mut ir = IRConfig {
            irname: "LR2IR".to_string(),
            userid: "alice".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };

        ir.sanitize_credentials_for_write();

        assert!(ir.userid.is_empty());
        assert!(ir.password.is_empty());
        assert!(!ir.cuserid.is_empty());
        assert!(!ir.cpassword.is_empty());
        assert!(ir.cuserid.starts_with("v2:"));
        assert!(ir.cpassword.starts_with("v2:"));
        assert_ne!(ir.cuserid, "alice");
        assert_ne!(ir.cpassword, "secret");
    }

    #[test]
    fn test_hydrate_credentials_for_use_decrypts_ciphertext() {
        let mut ir = IRConfig {
            irname: "LR2IR".to_string(),
            userid: "alice".to_string(),
            password: "secret".to_string(),
            ..Default::default()
        };
        ir.sanitize_credentials_for_write();

        let mut hydrated = IRConfig {
            irname: ir.irname.clone(),
            cuserid: ir.cuserid.clone(),
            cpassword: ir.cpassword.clone(),
            ..Default::default()
        };
        hydrated.hydrate_credentials_for_use();

        assert_eq!(hydrated.userid, "alice");
        assert_eq!(hydrated.password, "secret");
        assert_eq!(hydrated.cuserid, ir.cuserid);
        assert_eq!(hydrated.cpassword, ir.cpassword);
    }

    #[test]
    fn test_serde_round_trip() {
        let ir = IRConfig {
            irname: "LR2IR".to_string(),
            cuserid: "encrypted_user".to_string(),
            cpassword: "encrypted_pass".to_string(),
            irsend: IR_SEND_COMPLETE_SONG,
            importscore: true,
            importrival: false,
            ..Default::default()
        };
        let json = serde_json::to_string(&ir).unwrap();
        let back: IRConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.irname, "LR2IR");
        assert_eq!(back.cuserid, "encrypted_user");
        assert_eq!(back.cpassword, "encrypted_pass");
        assert_eq!(back.irsend, IR_SEND_COMPLETE_SONG);
        assert!(back.importscore);
        assert!(!back.importrival);
    }

    #[test]
    fn test_deserialize_from_empty() {
        let ir: IRConfig = serde_json::from_str("{}").unwrap();
        assert!(ir.irname.is_empty());
        assert!(ir.importrival); // default is true
    }

    #[test]
    fn test_constants() {
        assert_eq!(IR_SEND_ALWAYS, 0);
        assert_eq!(IR_SEND_COMPLETE_SONG, 1);
        assert_eq!(IR_SEND_UPDATE_SCORE, 2);
    }

    #[test]
    fn test_hydrate_credentials_for_use_decrypts_legacy_ciphertext() {
        let legacy_user = legacy_encrypt_credential("legacy_user");
        let legacy_pass = legacy_encrypt_credential("legacy_pass");
        let mut ir = IRConfig {
            irname: "LR2IR".to_string(),
            cuserid: legacy_user,
            cpassword: legacy_pass,
            ..Default::default()
        };

        ir.hydrate_credentials_for_use();

        assert_eq!(ir.userid, "legacy_user");
        assert_eq!(ir.password, "legacy_pass");
    }

    fn legacy_encrypt_credential(plaintext: &str) -> String {
        use aes::cipher::BlockEncryptMut;
        type LegacyAes128EcbEncryptor = ecb::Encryptor<Aes128>;

        let cipher = LegacyAes128EcbEncryptor::new_from_slice(LEGACY_IR_CONFIG_AES_KEY).unwrap();
        let msg_len = plaintext.len();
        let mut buf = vec![0_u8; msg_len + 16];
        buf[..msg_len].copy_from_slice(plaintext.as_bytes());
        let encrypted = cipher
            .encrypt_padded_mut::<Pkcs7>(&mut buf, msg_len)
            .unwrap();
        BASE64_STANDARD.encode(encrypted)
    }
}
