// Cross-platform clipboard helper using arboard crate.
// Moved from stubs.rs in Phase 25a.

/// Cross-platform clipboard helper using arboard crate
pub struct ClipboardHelper;

impl ClipboardHelper {
    /// Copy an image file to the system clipboard.
    /// Uses arboard for cross-platform clipboard access.
    pub fn copy_image_to_clipboard(path: &str) -> anyhow::Result<()> {
        use arboard::{Clipboard, ImageData};
        use std::borrow::Cow;

        let img = image::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to open image {}: {}", path, e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let bytes = rgba.into_raw();

        let img_data = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(bytes),
        };

        let mut clipboard =
            Clipboard::new().map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;
        clipboard
            .set_image(img_data)
            .map_err(|e| anyhow::anyhow!("Failed to copy image to clipboard: {}", e))?;
        Ok(())
    }
}
