/// Thin wrapper around arboard::Clipboard that silences errors in non-critical paths.
pub struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
        }
    }

    pub fn set_contents(&self, text: &str) {
        if let Some(ref mut cb) = self
            .inner
            .as_ref()
            .and_then(|_| arboard::Clipboard::new().ok())
        {
            if let Err(e) = cb.set_text(text) {
                log::warn!("Clipboard::set_contents failed: {}", e);
            }
        } else {
            log::warn!("Clipboard::set_contents: clipboard unavailable");
        }
    }
}
