use crate::objects::skin_image::SkinImage;
use crate::objects::skin_number::SkinNumber;
use crate::stubs::{Rectangle, TextureRegion};
use crate::text::skin_text::SkinText;

/// Bar data extracted from select skin loaders (LR2, JSON).
/// Transferred to MusicSelector after skin loading so BarRenderer can use it.
pub struct SelectBarData {
    /// Bar body images for the selected (focused) bar
    pub barimageon: Vec<Option<SkinImage>>,
    /// Bar body images for non-selected bars
    pub barimageoff: Vec<Option<SkinImage>>,
    /// Center bar index (which bar slot is the cursor)
    pub center_bar: i32,
    /// Clickable bar indices
    pub clickable_bar: Vec<i32>,
    /// Bar level SkinNumber objects (e.g., difficulty level display)
    pub barlevel: Vec<Option<SkinNumber>>,
    /// Bar title SkinText objects (e.g., song title text)
    pub bartext: Vec<Option<Box<dyn SkinText>>>,
    /// Lamp images indexed by lamp ID (0-10)
    pub barlamp: Vec<Option<SkinImage>>,
    /// Player lamp images indexed by lamp ID (0-10)
    pub barmylamp: Vec<Option<SkinImage>>,
    /// Rival lamp images indexed by lamp ID (0-10)
    pub barrivallamp: Vec<Option<SkinImage>>,
    /// Trophy images indexed by trophy ID (0-2)
    pub bartrophy: Vec<Option<SkinImage>>,
    /// Label images indexed by label ID (0-2)
    pub barlabel: Vec<Option<SkinImage>>,
    /// Distribution graph type (0 = lamp, 1 = rank)
    pub graph_type: Option<i32>,
    /// Custom images for the distribution graph (replaces default colors)
    pub graph_images: Option<Vec<TextureRegion>>,
    /// Distribution graph region set by DST_BAR_GRAPH
    pub graph_region: Rectangle,
}
