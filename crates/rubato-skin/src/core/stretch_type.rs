use crate::stubs::{Rectangle, TextureRegion};

/// Error returned when an integer does not map to a valid [`StretchType`] variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidStretchTypeId(pub i32);

impl std::fmt::Display for InvalidStretchTypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid StretchType id: {}", self.0)
    }
}

impl std::error::Error for InvalidStretchTypeId {}

/// Image stretch type (StretchType.java)
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StretchType {
    Stretch,
    KeepAspectRatioFitInner,
    KeepAspectRatioFitOuter,
    KeepAspectRatioFitOuterTrimmed,
    KeepAspectRatioFitWidth,
    KeepAspectRatioFitWidthTrimmed,
    KeepAspectRatioFitHeight,
    KeepAspectRatioFitHeightTrimmed,
    KeepAspectRatioNoExpanding,
    NoResize,
    NoResizeTrimmed,
}

impl TryFrom<i32> for StretchType {
    type Error = InvalidStretchTypeId;

    fn try_from(id: i32) -> Result<Self, Self::Error> {
        match id {
            0 => Ok(StretchType::Stretch),
            1 => Ok(StretchType::KeepAspectRatioFitInner),
            2 => Ok(StretchType::KeepAspectRatioFitOuter),
            3 => Ok(StretchType::KeepAspectRatioFitOuterTrimmed),
            4 => Ok(StretchType::KeepAspectRatioFitWidth),
            5 => Ok(StretchType::KeepAspectRatioFitWidthTrimmed),
            6 => Ok(StretchType::KeepAspectRatioFitHeight),
            7 => Ok(StretchType::KeepAspectRatioFitHeightTrimmed),
            8 => Ok(StretchType::KeepAspectRatioNoExpanding),
            9 => Ok(StretchType::NoResize),
            10 => Ok(StretchType::NoResizeTrimmed),
            _ => Err(InvalidStretchTypeId(id)),
        }
    }
}

impl StretchType {
    pub fn id(&self) -> i32 {
        match self {
            StretchType::Stretch => 0,
            StretchType::KeepAspectRatioFitInner => 1,
            StretchType::KeepAspectRatioFitOuter => 2,
            StretchType::KeepAspectRatioFitOuterTrimmed => 3,
            StretchType::KeepAspectRatioFitWidth => 4,
            StretchType::KeepAspectRatioFitWidthTrimmed => 5,
            StretchType::KeepAspectRatioFitHeight => 6,
            StretchType::KeepAspectRatioFitHeightTrimmed => 7,
            StretchType::KeepAspectRatioNoExpanding => 8,
            StretchType::NoResize => 9,
            StretchType::NoResizeTrimmed => 10,
        }
    }

    /// Convert from integer id.
    ///
    /// Thin wrapper around the `TryFrom<i32>` trait impl.
    pub fn from_id(id: i32) -> Option<StretchType> {
        StretchType::try_from(id).ok()
    }

    pub fn values() -> &'static [StretchType] {
        &[
            StretchType::Stretch,
            StretchType::KeepAspectRatioFitInner,
            StretchType::KeepAspectRatioFitOuter,
            StretchType::KeepAspectRatioFitOuterTrimmed,
            StretchType::KeepAspectRatioFitWidth,
            StretchType::KeepAspectRatioFitWidthTrimmed,
            StretchType::KeepAspectRatioFitHeight,
            StretchType::KeepAspectRatioFitHeightTrimmed,
            StretchType::KeepAspectRatioNoExpanding,
            StretchType::NoResize,
            StretchType::NoResizeTrimmed,
        ]
    }

    pub fn stretch_rect(
        &self,
        rectangle: &mut Rectangle,
        trimmed_image: &mut TextureRegion,
        image: &TextureRegion,
    ) {
        match self {
            StretchType::Stretch => {
                trimmed_image.set_from(image);
            }
            StretchType::KeepAspectRatioFitInner => {
                trimmed_image.set_from(image);
                let scale_x = rectangle.width / image.region_width as f32;
                let scale_y = rectangle.height / image.region_height as f32;
                if scale_x <= scale_y {
                    fit_height(rectangle, image.region_height as f32 * scale_x);
                } else {
                    fit_width(rectangle, image.region_width as f32 * scale_y);
                }
            }
            StretchType::KeepAspectRatioFitOuter => {
                trimmed_image.set_from(image);
                let scale_x = rectangle.width / image.region_width as f32;
                let scale_y = rectangle.height / image.region_height as f32;
                if scale_x >= scale_y {
                    fit_height(rectangle, image.region_height as f32 * scale_x);
                } else {
                    fit_width(rectangle, image.region_width as f32 * scale_y);
                }
            }
            StretchType::KeepAspectRatioFitOuterTrimmed => {
                trimmed_image.set_from(image);
                let scale_x = rectangle.width / image.region_width as f32;
                let scale_y = rectangle.height / image.region_height as f32;
                if scale_x >= scale_y {
                    fit_height_trimmed(rectangle, scale_x, trimmed_image);
                } else {
                    fit_width_trimmed(rectangle, scale_y, trimmed_image);
                }
            }
            StretchType::KeepAspectRatioFitWidth => {
                trimmed_image.set_from(image);
                fit_height(
                    rectangle,
                    image.region_height as f32 * rectangle.width / image.region_width as f32,
                );
            }
            StretchType::KeepAspectRatioFitWidthTrimmed => {
                trimmed_image.set_from(image);
                fit_height_trimmed(
                    rectangle,
                    rectangle.width / image.region_width as f32,
                    trimmed_image,
                );
            }
            StretchType::KeepAspectRatioFitHeight => {
                trimmed_image.set_from(image);
                fit_width(
                    rectangle,
                    image.region_width as f32 * rectangle.height / image.region_height as f32,
                );
            }
            StretchType::KeepAspectRatioFitHeightTrimmed => {
                trimmed_image.set_from(image);
                fit_width_trimmed(
                    rectangle,
                    rectangle.height / image.region_height as f32,
                    trimmed_image,
                );
            }
            StretchType::KeepAspectRatioNoExpanding => {
                trimmed_image.set_from(image);
                let scale = 1.0f32.min(
                    (rectangle.width / image.region_width as f32)
                        .min(rectangle.height / image.region_height as f32),
                );
                fit_width(rectangle, image.region_width as f32 * scale);
                fit_height(rectangle, image.region_height as f32 * scale);
            }
            StretchType::NoResize => {
                trimmed_image.set_from(image);
                fit_width(rectangle, image.region_width as f32);
                fit_height(rectangle, image.region_height as f32);
            }
            StretchType::NoResizeTrimmed => {
                trimmed_image.set_from(image);
                fit_width_trimmed(rectangle, 1.0, trimmed_image);
                fit_height_trimmed(rectangle, 1.0, trimmed_image);
            }
        }
    }
}

fn fit_width(rectangle: &mut Rectangle, width: f32) {
    let cx = rectangle.x + rectangle.width * 0.5;
    rectangle.width = width;
    rectangle.x = cx - rectangle.width * 0.5;
}

fn fit_height(rectangle: &mut Rectangle, height: f32) {
    let cy = rectangle.y + rectangle.height * 0.5;
    rectangle.height = height;
    rectangle.y = cy - rectangle.height * 0.5;
}

fn fit_width_trimmed(rectangle: &mut Rectangle, scale: f32, image: &mut TextureRegion) {
    let width = scale * image.region_width as f32;
    if rectangle.width < width {
        let cx = image.region_x as f32 + image.region_width as f32 * 0.5;
        let w = rectangle.width / scale;
        image.region_x = (cx - w * 0.5) as i32;
        image.region_width = w as i32;
    } else {
        fit_width(rectangle, width);
    }
}

fn fit_height_trimmed(rectangle: &mut Rectangle, scale: f32, image: &mut TextureRegion) {
    let height = scale * image.region_height as f32;
    if rectangle.height < height {
        let cy = image.region_y as f32 + image.region_height as f32 * 0.5;
        let h = rectangle.height / scale;
        image.region_y = (cy - h * 0.5) as i32;
        image.region_height = h as i32;
    } else {
        fit_height(rectangle, height);
    }
}
