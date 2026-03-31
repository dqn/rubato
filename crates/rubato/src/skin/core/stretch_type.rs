use crate::skin::reexports::{Rectangle, TextureRegion};

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
        // Recalculate horizontal UVs to match the trimmed region.
        // In Java (LibGDX), setRegionX/setRegionWidth recalculate UVs internally.
        if let Some(tex) = image.texture.as_ref().filter(|t| t.width > 0) {
            let inv_w = 1.0 / tex.width as f32;
            image.u = image.region_x as f32 * inv_w;
            image.u2 = (image.region_x + image.region_width) as f32 * inv_w;
        }
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
        // Recalculate vertical UVs to match the trimmed region.
        // In Java (LibGDX), setRegionY/setRegionHeight recalculate UVs internally.
        if let Some(tex) = image.texture.as_ref().filter(|t| t.height > 0) {
            let inv_h = 1.0 / tex.height as f32;
            image.v = image.region_y as f32 * inv_h;
            image.v2 = (image.region_y + image.region_height) as f32 * inv_h;
        }
    } else {
        fit_height(rectangle, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::reexports::Texture;

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

    fn make_texture(w: i32, h: i32) -> Texture {
        Texture {
            width: w,
            height: h,
            disposed: false,
            path: None,
            rgba_data: None,
            ..Default::default()
        }
    }

    #[test]
    fn fit_width_trimmed_recalculates_uvs() {
        // Image: 200x100 pixels in a 400x200 texture, starting at (100, 50)
        let tex = make_texture(400, 200);
        let mut image = TextureRegion::from_texture_region(tex, 100, 50, 200, 100);
        // Initial UVs: u=100/400=0.25, u2=(100+200)/400=0.75
        assert!((image.u - 0.25).abs() < 1e-6);
        assert!((image.u2 - 0.75).abs() < 1e-6);

        // Rectangle narrower than scaled image width triggers trimming branch
        let mut rect = Rectangle::new(0.0, 0.0, 50.0, 200.0);
        // scale=1.0, width = 1.0 * 200 = 200 > rect.width(50), so trimming happens
        fit_width_trimmed(&mut rect, 1.0, &mut image);

        // region_x and region_width should have changed
        // cx = 100 + 200*0.5 = 200, w = 50/1.0 = 50
        // region_x = (200 - 25) = 175, region_width = 50
        assert_eq!(image.region_x, 175);
        assert_eq!(image.region_width, 50);

        // UVs must reflect the new region: u = 175/400, u2 = (175+50)/400 = 225/400
        let expected_u = 175.0 / 400.0;
        let expected_u2 = 225.0 / 400.0;
        assert!(
            (image.u - expected_u).abs() < 1e-6,
            "u: expected {expected_u}, got {}",
            image.u
        );
        assert!(
            (image.u2 - expected_u2).abs() < 1e-6,
            "u2: expected {expected_u2}, got {}",
            image.u2
        );
    }

    #[test]
    fn fit_height_trimmed_recalculates_uvs() {
        // Image: 100x200 pixels in a 200x400 texture, starting at (50, 100)
        let tex = make_texture(200, 400);
        let mut image = TextureRegion::from_texture_region(tex, 50, 100, 100, 200);
        // Initial UVs: v=100/400=0.25, v2=(100+200)/400=0.75
        assert!((image.v - 0.25).abs() < 1e-6);
        assert!((image.v2 - 0.75).abs() < 1e-6);

        // Rectangle shorter than scaled image height triggers trimming branch
        let mut rect = Rectangle::new(0.0, 0.0, 200.0, 50.0);
        // scale=1.0, height = 1.0 * 200 = 200 > rect.height(50), so trimming happens
        fit_height_trimmed(&mut rect, 1.0, &mut image);

        // region_y and region_height should have changed
        // cy = 100 + 200*0.5 = 200, h = 50/1.0 = 50
        // region_y = (200 - 25) = 175, region_height = 50
        assert_eq!(image.region_y, 175);
        assert_eq!(image.region_height, 50);

        // UVs must reflect the new region: v = 175/400, v2 = (175+50)/400 = 225/400
        let expected_v = 175.0 / 400.0;
        let expected_v2 = 225.0 / 400.0;
        assert!(
            (image.v - expected_v).abs() < 1e-6,
            "v: expected {expected_v}, got {}",
            image.v
        );
        assert!(
            (image.v2 - expected_v2).abs() < 1e-6,
            "v2: expected {expected_v2}, got {}",
            image.v2
        );
    }

    #[test]
    fn stretch_rect_trimmed_variants_update_uvs() {
        // Test all 4 trimmed variants via stretch_rect to ensure UVs are updated
        let tex = make_texture(400, 400);
        let image = TextureRegion::from_texture_region(tex, 50, 50, 300, 300);
        // Initial UVs: u=50/400=0.125, u2=350/400=0.875, v=0.125, v2=0.875

        // FitOuterTrimmed: image 300x300, rect 100x100, scale_x=scale_y=0.333
        // Since equal, goes to fit_height_trimmed branch
        {
            let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
            let mut trimmed = TextureRegion::default();
            StretchType::KeepAspectRatioFitOuterTrimmed.stretch_rect(
                &mut rect,
                &mut trimmed,
                &image,
            );
            // Trimming should have happened on at least one axis and UVs should match region
            if let Some(ref tex) = trimmed.texture {
                if tex.width > 0 {
                    let expected_u = trimmed.region_x as f32 / tex.width as f32;
                    let expected_u2 =
                        (trimmed.region_x + trimmed.region_width) as f32 / tex.width as f32;
                    assert!(
                        (trimmed.u - expected_u).abs() < 1e-5,
                        "FitOuterTrimmed u: expected {expected_u}, got {}",
                        trimmed.u
                    );
                    assert!(
                        (trimmed.u2 - expected_u2).abs() < 1e-5,
                        "FitOuterTrimmed u2: expected {expected_u2}, got {}",
                        trimmed.u2
                    );
                }
                if tex.height > 0 {
                    let expected_v = trimmed.region_y as f32 / tex.height as f32;
                    let expected_v2 =
                        (trimmed.region_y + trimmed.region_height) as f32 / tex.height as f32;
                    assert!(
                        (trimmed.v - expected_v).abs() < 1e-5,
                        "FitOuterTrimmed v: expected {expected_v}, got {}",
                        trimmed.v
                    );
                    assert!(
                        (trimmed.v2 - expected_v2).abs() < 1e-5,
                        "FitOuterTrimmed v2: expected {expected_v2}, got {}",
                        trimmed.v2
                    );
                }
            }
        }

        // NoResizeTrimmed: image 300x300, rect 100x100
        {
            let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
            let mut trimmed = TextureRegion::default();
            StretchType::NoResizeTrimmed.stretch_rect(&mut rect, &mut trimmed, &image);
            if let Some(ref tex) = trimmed.texture {
                if tex.width > 0 {
                    let expected_u = trimmed.region_x as f32 / tex.width as f32;
                    let expected_u2 =
                        (trimmed.region_x + trimmed.region_width) as f32 / tex.width as f32;
                    assert!(
                        (trimmed.u - expected_u).abs() < 1e-5,
                        "NoResizeTrimmed u: expected {expected_u}, got {}",
                        trimmed.u
                    );
                    assert!(
                        (trimmed.u2 - expected_u2).abs() < 1e-5,
                        "NoResizeTrimmed u2: expected {expected_u2}, got {}",
                        trimmed.u2
                    );
                }
                if tex.height > 0 {
                    let expected_v = trimmed.region_y as f32 / tex.height as f32;
                    let expected_v2 =
                        (trimmed.region_y + trimmed.region_height) as f32 / tex.height as f32;
                    assert!(
                        (trimmed.v - expected_v).abs() < 1e-5,
                        "NoResizeTrimmed v: expected {expected_v}, got {}",
                        trimmed.v
                    );
                    assert!(
                        (trimmed.v2 - expected_v2).abs() < 1e-5,
                        "NoResizeTrimmed v2: expected {expected_v2}, got {}",
                        trimmed.v2
                    );
                }
            }
        }
    }

    // ---------------------------------------------------------------
    // Phase 5: Functional correctness tests for each StretchType variant
    // ---------------------------------------------------------------

    fn make_image(w: i32, h: i32) -> TextureRegion {
        TextureRegion {
            region_width: w,
            region_height: h,
            ..TextureRegion::default()
        }
    }

    #[test]
    fn test_stretch_preserves_rectangle() {
        let mut rect = Rectangle::new(10.0, 20.0, 100.0, 200.0);
        let image = make_image(50, 80);
        let mut trimmed = TextureRegion::default();
        let (orig_x, orig_y, orig_w, orig_h) = (rect.x, rect.y, rect.width, rect.height);

        StretchType::Stretch.stretch_rect(&mut rect, &mut trimmed, &image);

        assert_eq!(rect.x, orig_x);
        assert_eq!(rect.y, orig_y);
        assert_eq!(rect.width, orig_w);
        assert_eq!(rect.height, orig_h);
    }

    #[test]
    fn test_fit_inner_landscape_in_square() {
        // 200x100 image in 100x100 rect
        // scale_x = 100/200 = 0.5, scale_y = 100/100 = 1.0
        // scale_x < scale_y -> fit_height(100 * 0.5 = 50)
        let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let image = make_image(200, 100);
        let mut trimmed = TextureRegion::default();

        StretchType::KeepAspectRatioFitInner.stretch_rect(&mut rect, &mut trimmed, &image);

        // height centered: cy=50, new_h=50 -> y = 50 - 25 = 25
        assert!((rect.height - 50.0).abs() < 0.01);
        assert!((rect.y - 25.0).abs() < 0.01);
        // width unchanged
        assert!((rect.width - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_fit_outer_landscape_in_square() {
        // 200x100 image in 100x100 rect
        // scale_x = 100/200 = 0.5, scale_y = 100/100 = 1.0
        // scale_x < scale_y -> fit_width(200 * 1.0 = 200)
        let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let image = make_image(200, 100);
        let mut trimmed = TextureRegion::default();

        StretchType::KeepAspectRatioFitOuter.stretch_rect(&mut rect, &mut trimmed, &image);

        // width centered: cx=50, new_w=200 -> x = 50 - 100 = -50
        assert!((rect.width - 200.0).abs() < 0.01);
        assert!((rect.x - (-50.0)).abs() < 0.01);
        // height unchanged
        assert!((rect.height - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_fit_width_preserves_aspect() {
        // 200x100 image in 100x100 rect
        // height = 100 * (100/200) = 50, centered
        let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let image = make_image(200, 100);
        let mut trimmed = TextureRegion::default();

        StretchType::KeepAspectRatioFitWidth.stretch_rect(&mut rect, &mut trimmed, &image);

        assert!((rect.height - 50.0).abs() < 0.01);
        // cy=50, new_h=50 -> y = 50 - 25 = 25
        assert!((rect.y - 25.0).abs() < 0.01);
        assert!((rect.width - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_fit_height_preserves_aspect() {
        // 200x100 image in 100x100 rect
        // width = 200 * (100/100) = 200, centered
        let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let image = make_image(200, 100);
        let mut trimmed = TextureRegion::default();

        StretchType::KeepAspectRatioFitHeight.stretch_rect(&mut rect, &mut trimmed, &image);

        assert!((rect.width - 200.0).abs() < 0.01);
        // cx=50, new_w=200 -> x = 50 - 100 = -50
        assert!((rect.x - (-50.0)).abs() < 0.01);
        assert!((rect.height - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_no_expanding_caps_at_1x() {
        // 50x50 image in 100x100 rect
        // scale = min(1.0, min(100/50, 100/50)) = min(1.0, 2.0) = 1.0
        // width = 50*1 = 50, height = 50*1 = 50
        let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let image = make_image(50, 50);
        let mut trimmed = TextureRegion::default();

        StretchType::KeepAspectRatioNoExpanding.stretch_rect(&mut rect, &mut trimmed, &image);

        assert!((rect.width - 50.0).abs() < 0.01);
        assert!((rect.height - 50.0).abs() < 0.01);
        // centered: cx=50, x=50-25=25; cy=50, y=50-25=25
        assert!((rect.x - 25.0).abs() < 0.01);
        assert!((rect.y - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_no_resize_uses_image_dimensions() {
        // 200x100 image in 100x100 rect
        // NoResize: width=200, height=100, centered on original rect center
        let mut rect = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        let image = make_image(200, 100);
        let mut trimmed = TextureRegion::default();

        StretchType::NoResize.stretch_rect(&mut rect, &mut trimmed, &image);

        assert!((rect.width - 200.0).abs() < 0.01);
        assert!((rect.height - 100.0).abs() < 0.01);
        // cx=50, new_w=200 -> x = 50 - 100 = -50
        assert!((rect.x - (-50.0)).abs() < 0.01);
        // cy=50, new_h=100 -> y = 50 - 50 = 0
        assert!((rect.y - 0.0).abs() < 0.01);
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
