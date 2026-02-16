// Skin loading state manager.

use std::path::{Path, PathBuf};

/// Skin types matching Java SkinType enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Most variants not yet used; reserved for future screen implementation
pub enum SkinType {
    MusicSelect,
    Decide,
    Play5,
    Play7,
    Play9,
    Play10,
    Play14,
    Play24,
    Result,
    CourseResult,
    KeyConfig,
    SkinConfig,
}

/// Status of the most recent skin load attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(dead_code)] // Reserved for skin load fallback chain (not yet integrated)
pub enum SkinLoadStatus {
    /// No skin load has been attempted.
    #[default]
    None,
    /// The configured skin loaded successfully.
    Loaded,
    /// The configured skin failed; default skin loaded instead.
    Fallback,
    /// All skins failed; using minimal UI (black bg + error message).
    MinimalUi,
}

/// Result of a skin load attempt with fallback chain.
#[derive(Debug)]
#[allow(dead_code)] // Reserved for skin load fallback chain (not yet integrated)
pub struct SkinLoadResult {
    pub status: SkinLoadStatus,
    pub error_message: Option<String>,
}

/// Returns the default skin path for the given skin type.
#[allow(dead_code)] // Reserved for skin load fallback chain (not yet integrated)
fn default_skin_path(skin_type: SkinType) -> Option<PathBuf> {
    let relative = match skin_type {
        SkinType::Play7 => "skin/default/play/play7.luaskin",
        SkinType::Play5 => "skin/default/play5.json",
        SkinType::Play14 => "skin/default/play14.json",
        SkinType::Play10 => "skin/default/play10.json",
        SkinType::Play9 => "skin/default/play9.json",
        SkinType::Play24 => "skin/default/play24.json",
        SkinType::MusicSelect => "skin/default/select.json",
        SkinType::Decide => "skin/default/decide/decide.luaskin",
        SkinType::Result => "skin/default/result/result.luaskin",
        SkinType::CourseResult => "skin/default/graderesult.json",
        SkinType::KeyConfig => "skin/default/keyconfig/keyconfig.luaskin",
        SkinType::SkinConfig => "skin/default/skinselect/skinselect.luaskin",
    };
    Some(PathBuf::from(relative))
}

/// Manages skin loading requests and state.
#[derive(Default)]
pub struct SkinManager {
    /// Pending skin load request (set by states, consumed by system).
    request: Option<SkinType>,
    /// Whether the current skin is fully loaded.
    loaded: bool,
    /// Currently active skin type.
    current: Option<SkinType>,
    /// Status of the most recent skin load.
    #[allow(dead_code)] // Reserved for skin load fallback chain (not yet integrated)
    pub load_status: SkinLoadStatus,
    /// Error message from the most recent failed load attempt.
    #[allow(dead_code)] // Reserved for skin load fallback chain (not yet integrated)
    pub last_error: Option<String>,
}

impl SkinManager {
    #[allow(dead_code)] // Reserved for skin load system (not yet integrated)
    pub fn new() -> Self {
        Self::default()
    }

    /// Request a skin to be loaded.
    pub fn request_load(&mut self, skin_type: SkinType) {
        self.request = Some(skin_type);
        self.loaded = false;
    }

    /// Take the pending request (consumed by skin loading system).
    #[allow(dead_code)] // Reserved for skin load system (not yet integrated)
    pub fn take_request(&mut self) -> Option<SkinType> {
        self.request.take()
    }

    /// Mark the current skin as loaded.
    #[allow(dead_code)] // Reserved for skin load system (not yet integrated)
    pub fn mark_loaded(&mut self, skin_type: SkinType) {
        self.current = Some(skin_type);
        self.loaded = true;
    }

    #[allow(dead_code)] // Reserved for skin load system (not yet integrated)
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn current_type(&self) -> Option<SkinType> {
        self.current
    }

    /// Try loading a skin with fallback chain:
    /// 1. Try `config_path` (if provided and file exists) -> `Loaded`
    /// 2. Try default skin for `skin_type` -> `Fallback`
    /// 3. Use minimal UI -> `MinimalUi`
    ///
    /// The `load_fn` callback attempts to load a skin from the given path,
    /// returning `Ok(())` on success or an error message on failure.
    #[allow(dead_code)] // Reserved for skin load fallback chain (not yet integrated)
    pub fn try_load_with_fallback<F>(
        &mut self,
        skin_type: SkinType,
        config_path: Option<&str>,
        load_fn: F,
    ) -> SkinLoadResult
    where
        F: Fn(&Path) -> Result<(), String>,
    {
        // Step 1: Try the configured skin path.
        if let Some(path_str) = config_path {
            let path = Path::new(path_str);
            match load_fn(path) {
                Ok(()) => {
                    self.mark_loaded(skin_type);
                    self.load_status = SkinLoadStatus::Loaded;
                    self.last_error = None;
                    return SkinLoadResult {
                        status: SkinLoadStatus::Loaded,
                        error_message: None,
                    };
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load configured skin '{}': {}. Trying default.",
                        path_str,
                        e
                    );
                }
            }
        }

        // Step 2: Try the default skin.
        if let Some(default_path) = default_skin_path(skin_type) {
            match load_fn(&default_path) {
                Ok(()) => {
                    self.mark_loaded(skin_type);
                    self.load_status = SkinLoadStatus::Fallback;
                    self.last_error = None;
                    return SkinLoadResult {
                        status: SkinLoadStatus::Fallback,
                        error_message: None,
                    };
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to load default skin '{}': {}. Using minimal UI.",
                        default_path.display(),
                        e
                    );
                }
            }
        }

        // Step 3: Minimal UI fallback.
        let error_msg = format!(
            "All skin load attempts failed for {:?}. Press ESC to return.",
            skin_type
        );
        self.load_status = SkinLoadStatus::MinimalUi;
        self.last_error = Some(error_msg.clone());
        self.loaded = false;
        self.current = None;

        SkinLoadResult {
            status: SkinLoadStatus::MinimalUi,
            error_message: Some(error_msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_starts_unloaded() {
        let mgr = SkinManager::new();
        assert!(!mgr.is_loaded());
        assert_eq!(mgr.current_type(), None);
        assert_eq!(mgr.load_status, SkinLoadStatus::None);
        assert!(mgr.last_error.is_none());
    }

    #[test]
    fn request_load_sets_request() {
        let mut mgr = SkinManager::new();
        mgr.request_load(SkinType::Play7);
        assert_eq!(mgr.take_request(), Some(SkinType::Play7));
        assert!(!mgr.is_loaded());
    }

    #[test]
    fn take_request_clears_request() {
        let mut mgr = SkinManager::new();
        mgr.request_load(SkinType::MusicSelect);
        assert_eq!(mgr.take_request(), Some(SkinType::MusicSelect));
        assert_eq!(mgr.take_request(), None);
    }

    #[test]
    fn mark_loaded_sets_loaded_and_current() {
        let mut mgr = SkinManager::new();
        mgr.request_load(SkinType::Result);
        mgr.mark_loaded(SkinType::Result);
        assert!(mgr.is_loaded());
        assert_eq!(mgr.current_type(), Some(SkinType::Result));
    }

    #[test]
    fn default_skin_path_returns_path_for_all_types() {
        let types = [
            SkinType::MusicSelect,
            SkinType::Decide,
            SkinType::Play5,
            SkinType::Play7,
            SkinType::Play9,
            SkinType::Play10,
            SkinType::Play14,
            SkinType::Play24,
            SkinType::Result,
            SkinType::CourseResult,
            SkinType::KeyConfig,
            SkinType::SkinConfig,
        ];
        for skin_type in types {
            assert!(
                default_skin_path(skin_type).is_some(),
                "default_skin_path should return Some for {:?}",
                skin_type
            );
        }
    }

    #[test]
    fn fallback_configured_skin_loads_successfully() {
        let mut mgr = SkinManager::new();
        let result = mgr.try_load_with_fallback(SkinType::Play7, Some("my/skin.lua"), |_| Ok(()));

        assert_eq!(result.status, SkinLoadStatus::Loaded);
        assert!(result.error_message.is_none());
        assert_eq!(mgr.load_status, SkinLoadStatus::Loaded);
        assert!(mgr.is_loaded());
        assert_eq!(mgr.current_type(), Some(SkinType::Play7));
        assert!(mgr.last_error.is_none());
    }

    #[test]
    fn fallback_to_default_skin_on_config_failure() {
        let mut mgr = SkinManager::new();
        let call_count = std::cell::Cell::new(0u32);
        let result = mgr.try_load_with_fallback(SkinType::Play7, Some("bad/skin.lua"), |path| {
            let n = call_count.get();
            call_count.set(n + 1);
            if n == 0 {
                // First call (configured skin) fails
                Err("file not found".to_string())
            } else {
                // Second call (default skin) succeeds
                assert!(path.to_str().unwrap().contains("skin/default"));
                Ok(())
            }
        });

        assert_eq!(result.status, SkinLoadStatus::Fallback);
        assert!(result.error_message.is_none());
        assert_eq!(mgr.load_status, SkinLoadStatus::Fallback);
        assert!(mgr.is_loaded());
        assert!(mgr.last_error.is_none());
    }

    #[test]
    fn fallback_to_minimal_ui_when_all_fail() {
        let mut mgr = SkinManager::new();
        let result = mgr.try_load_with_fallback(SkinType::MusicSelect, Some("bad.lua"), |_| {
            Err("load error".to_string())
        });

        assert_eq!(result.status, SkinLoadStatus::MinimalUi);
        assert!(result.error_message.is_some());
        assert!(
            result
                .error_message
                .as_ref()
                .unwrap()
                .contains("ESC to return")
        );
        assert_eq!(mgr.load_status, SkinLoadStatus::MinimalUi);
        assert!(!mgr.is_loaded());
        assert!(mgr.last_error.is_some());
    }

    #[test]
    fn fallback_no_config_path_tries_default_first() {
        let mut mgr = SkinManager::new();
        let result = mgr.try_load_with_fallback(SkinType::Decide, None, |path| {
            // Should go straight to default skin
            assert!(path.to_str().unwrap().contains("skin/default"));
            Ok(())
        });

        assert_eq!(result.status, SkinLoadStatus::Fallback);
        assert!(mgr.is_loaded());
    }

    #[test]
    fn fallback_no_config_path_all_fail() {
        let mut mgr = SkinManager::new();
        let result =
            mgr.try_load_with_fallback(SkinType::Result, None, |_| Err("unavailable".to_string()));

        assert_eq!(result.status, SkinLoadStatus::MinimalUi);
        assert!(!mgr.is_loaded());
        assert!(mgr.last_error.is_some());
    }
}
