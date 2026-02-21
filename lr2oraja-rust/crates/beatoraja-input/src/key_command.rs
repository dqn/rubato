/// KeyCommand enum
///
/// Translated from: bms.player.beatoraja.input.KeyCommand
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCommand {
    ShowFps,
    UpdateFolder,
    OpenExplorer,
    CopySongMd5Hash,
    CopySongSha256Hash,
    SwitchScreenMode,
    SaveScreenshot,
    PostTwitter,
    AddFavoriteSong,
    AddFavoriteChart,
    AutoplayFolder,
    OpenIr,
    OpenSkinConfiguration,
    ToggleModMenu,
    CopyHighlightedMenuText,
}
