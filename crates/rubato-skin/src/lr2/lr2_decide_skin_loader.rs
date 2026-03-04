use crate::lr2::lr2_skin_csv_loader::{LR2SkinCSVLoaderState, LR2SkinLoaderAccess};
use crate::stubs::Resolution;

/// LR2 decide skin loader
///
/// Translated from LR2DecideSkinLoader.java (25 lines)
/// Loads LR2 decide (music decision) skins.
/// This is a minimal loader that extends LR2SkinCSVLoader with no additional commands.
///
/// Decide skin loader state
pub struct LR2DecideSkinLoaderState {
    pub csv: LR2SkinCSVLoaderState,
}

impl LR2DecideSkinLoaderState {
    pub fn new(src: Resolution, dst: Resolution, usecim: bool, skinpath: String) -> Self {
        Self {
            csv: LR2SkinCSVLoaderState::new(src, dst, usecim, skinpath),
        }
    }

    /// Process decide-specific commands (none - all delegated to CSV loader)
    pub fn process_decide_command(&mut self, cmd: &str, str_parts: &[String]) {
        self.csv.process_csv_command(cmd, str_parts);
    }
}

impl LR2SkinLoaderAccess for LR2DecideSkinLoaderState {
    fn csv_mut(&mut self) -> &mut LR2SkinCSVLoaderState {
        &mut self.csv
    }

    fn assemble_objects(&mut self, _skin: &mut crate::skin::Skin) {
        // Decide skin has no LR2-specific objects to assemble.
        // All objects are generic SRC/DST images handled by the base CSV loader.
    }
}
