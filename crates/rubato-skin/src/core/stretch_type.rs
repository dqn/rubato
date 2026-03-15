use crate::reexports::{Rectangle, TextureRegion};

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

    pub fn from_id(id: i32) -> Option<StretchType> {
        match id {
            0 => Some(StretchType::Stretch),
            1 => Some(StretchType::KeepAspectRatioFitInner),
            2 => Some(StretchType::KeepAspectRatioFitOuter),
            3 => Some(StretchType::KeepAspectRatioFitOuterTrimmed),
            4 => Some(StretchType::KeepAspectRatioFitWidth),
            5 => Some(StretchType::KeepAspectRatioFitWidthTrimmed),
            6 => Some(StretchType::KeepAspectRatioFitHeight),
            7 => Some(StretchType::KeepAspectRatioFitHeightTrimmed),
            8 => Some(StretchType::KeepAspectRatioNoExpanding),
            9 => Some(StretchType::NoResize),
            10 => Some(StretchType::NoResizeTrimmed),
            _ => None,
        }
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
        if image.region_width == 0 || image.region_height == 0 {
            trimmed_image.set_from(image);
            return;
        }
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
    if scale == 0.0 {
        return;
    }
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
    if scale == 0.0 {
        return;
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretch_rect_zero_region_width_does_not_produce_nan() {
        let mut rect = Rectangle::new(10.0, 20.0, 100.0, 200.0);
        let image = TextureRegion {
            region_width: 0,
            region_height: 50,
            ..TextureRegion::default()
        };
        let mut trimmed = TextureRegion::default();

        for variant in StretchType::values() {
            let (orig_rect_x, orig_rect_y, orig_rect_w, orig_rect_h) =
                (rect.x, rect.y, rect.width, rect.height);
            variant.stretch_rect(&mut rect, &mut trimmed, &image);

            // trimmed_image should be a copy of image
            assert_eq!(trimmed.region_width, image.region_width);
            assert_eq!(trimmed.region_height, image.region_height);

            // rectangle must not be corrupted with NaN/Inf
            assert!(rect.x.is_finite(), "{variant:?} produced non-finite x");
            assert!(rect.y.is_finite(), "{variant:?} produced non-finite y");
            assert!(
                rect.width.is_finite(),
                "{variant:?} produced non-finite width"
            );
            assert!(
                rect.height.is_finite(),
                "{variant:?} produced non-finite height"
            );

            // Reset rect for next iteration
            rect = Rectangle::new(orig_rect_x, orig_rect_y, orig_rect_w, orig_rect_h);
        }
    }

    #[test]
    fn stretch_rect_zero_region_height_does_not_produce_nan() {
        let mut rect = Rectangle::new(10.0, 20.0, 100.0, 200.0);
        let image = TextureRegion {
            region_width: 50,
            region_height: 0,
            ..TextureRegion::default()
        };
        let mut trimmed = TextureRegion::default();

        for variant in StretchType::values() {
            variant.stretch_rect(&mut rect, &mut trimmed, &image);

            assert_eq!(trimmed.region_width, image.region_width);
            assert_eq!(trimmed.region_height, image.region_height);
            assert!(rect.x.is_finite(), "{variant:?} produced non-finite x");
            assert!(rect.y.is_finite(), "{variant:?} produced non-finite y");
            assert!(
                rect.width.is_finite(),
                "{variant:?} produced non-finite width"
            );
            assert!(
                rect.height.is_finite(),
                "{variant:?} produced non-finite height"
            );

            rect = Rectangle::new(10.0, 20.0, 100.0, 200.0);
        }
    }

    #[test]
    fn stretch_rect_zero_rectangle_dimensions_does_not_produce_nan() {
        // When rectangle width/height are 0, scale = 0.0 and fit_*_trimmed
        // would divide by zero without the guard.
        let image = TextureRegion {
            region_width: 50,
            region_height: 50,
            ..TextureRegion::default()
        };

        for variant in StretchType::values() {
            let mut rect = Rectangle::new(10.0, 20.0, 0.0, 0.0);
            let mut trimmed = TextureRegion::default();
            variant.stretch_rect(&mut rect, &mut trimmed, &image);

            assert!(
                rect.x.is_finite(),
                "{variant:?} produced non-finite x with zero-size rect"
            );
            assert!(
                rect.y.is_finite(),
                "{variant:?} produced non-finite y with zero-size rect"
            );
            assert!(
                rect.width.is_finite(),
                "{variant:?} produced non-finite width with zero-size rect"
            );
            assert!(
                rect.height.is_finite(),
                "{variant:?} produced non-finite height with zero-size rect"
            );
        }
    }

    #[test]
    fn fit_width_trimmed_zero_scale_is_noop() {
        let mut rect = Rectangle::new(10.0, 20.0, 100.0, 200.0);
        let mut image = TextureRegion {
            region_width: 50,
            region_height: 50,
            ..TextureRegion::default()
        };
        let (orig_x, orig_y, orig_w, orig_h) = (rect.x, rect.y, rect.width, rect.height);
        let (orig_rw, orig_rh) = (image.region_width, image.region_height);

        fit_width_trimmed(&mut rect, 0.0, &mut image);

        assert_eq!(rect.x, orig_x);
        assert_eq!(rect.y, orig_y);
        assert_eq!(rect.width, orig_w);
        assert_eq!(rect.height, orig_h);
        assert_eq!(image.region_width, orig_rw);
        assert_eq!(image.region_height, orig_rh);
    }

    #[test]
    fn fit_height_trimmed_zero_scale_is_noop() {
        let mut rect = Rectangle::new(10.0, 20.0, 100.0, 200.0);
        let mut image = TextureRegion {
            region_width: 50,
            region_height: 50,
            ..TextureRegion::default()
        };
        let (orig_x, orig_y, orig_w, orig_h) = (rect.x, rect.y, rect.width, rect.height);
        let (orig_rw, orig_rh) = (image.region_width, image.region_height);

        fit_height_trimmed(&mut rect, 0.0, &mut image);

        assert_eq!(rect.x, orig_x);
        assert_eq!(rect.y, orig_y);
        assert_eq!(rect.width, orig_w);
        assert_eq!(rect.height, orig_h);
        assert_eq!(image.region_width, orig_rw);
        assert_eq!(image.region_height, orig_rh);
    }
}
