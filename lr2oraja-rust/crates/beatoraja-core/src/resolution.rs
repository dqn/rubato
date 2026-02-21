use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum Resolution {
    SD,
    SVGA,
    XGA,
    #[default]
    HD,
    QUADVGA,
    FWXGA,
    SXGAPLUS,
    HDPLUS,
    UXGA,
    WSXGAPLUS,
    FULLHD,
    WUXGA,
    QXGA,
    WQHD,
    ULTRAHD,
}

impl Resolution {
    pub fn width(&self) -> i32 {
        match self {
            Resolution::SD => 640,
            Resolution::SVGA => 800,
            Resolution::XGA => 1024,
            Resolution::HD => 1280,
            Resolution::QUADVGA => 1280,
            Resolution::FWXGA => 1366,
            Resolution::SXGAPLUS => 1400,
            Resolution::HDPLUS => 1600,
            Resolution::UXGA => 1600,
            Resolution::WSXGAPLUS => 1680,
            Resolution::FULLHD => 1920,
            Resolution::WUXGA => 1920,
            Resolution::QXGA => 2048,
            Resolution::WQHD => 2560,
            Resolution::ULTRAHD => 3840,
        }
    }

    pub fn height(&self) -> i32 {
        match self {
            Resolution::SD => 480,
            Resolution::SVGA => 600,
            Resolution::XGA => 768,
            Resolution::HD => 720,
            Resolution::QUADVGA => 960,
            Resolution::FWXGA => 768,
            Resolution::SXGAPLUS => 1050,
            Resolution::HDPLUS => 900,
            Resolution::UXGA => 1200,
            Resolution::WSXGAPLUS => 1050,
            Resolution::FULLHD => 1080,
            Resolution::WUXGA => 1200,
            Resolution::QXGA => 1536,
            Resolution::WQHD => 1440,
            Resolution::ULTRAHD => 2160,
        }
    }
}

impl fmt::Display for Resolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} ({} x {})", self, self.width(), self.height())
    }
}
