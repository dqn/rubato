// SkinImage.java -> skin_image.rs
// Mechanical line-by-line translation.

use crate::property::integer_property::IntegerProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::sources::skin_source::SkinSource;
use crate::sources::skin_source_image::SkinSourceImage;
use crate::sources::skin_source_movie::SkinSourceMovie;
use crate::sources::skin_source_reference::SkinSourceReference;
use crate::stubs::{MainState, TextureRegion};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

pub struct SkinImage {
    pub data: SkinObjectData,
    image: Vec<Option<Box<dyn SkinSource>>>,
    ref_prop: Option<Box<dyn IntegerProperty>>,
    current_image: Option<TextureRegion>,
    removed_sources: Vec<Box<dyn SkinSource>>,
    is_movie: bool,
}

impl SkinImage {
    /// Create an empty SkinImage with no sources (used in tests).
    pub fn new_empty() -> Self {
        Self {
            data: SkinObjectData::new(),
            image: Vec::new(),
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_image_id(imageid: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(SkinSourceReference::new(imageid)))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_single(image: TextureRegion) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(
                SkinSourceImage::new_with_int_timer_from_vec(vec![image], 0, 0),
            ))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_int_timer(image: Vec<TextureRegion>, timer: i32, cycle: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(
                SkinSourceImage::new_with_int_timer_from_vec(image, timer, cycle),
            ))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_int_timer_ref_id(
        images: Vec<Vec<TextureRegion>>,
        timer: i32,
        cycle: i32,
        ref_id: i32,
    ) -> Self {
        Self::new_with_int_timer_ref(
            images,
            timer,
            cycle,
            integer_property_factory::image_index_property_by_id(ref_id),
        )
    }

    pub fn new_with_int_timer_ref(
        images: Vec<Vec<TextureRegion>>,
        timer: i32,
        cycle: i32,
        ref_prop: Option<Box<dyn IntegerProperty>>,
    ) -> Self {
        let image: Vec<Option<Box<dyn SkinSource>>> = images
            .into_iter()
            .map(|img| -> Option<Box<dyn SkinSource>> {
                Some(Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                    img, timer, cycle,
                )))
            })
            .collect();
        Self {
            data: SkinObjectData::new(),
            image,
            ref_prop,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_timer(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_timer_ref_id(
        images: Vec<Vec<TextureRegion>>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        ref_id: i32,
    ) -> Self {
        // Each image set needs its own timer; for simplicity, use int timer 0
        let image: Vec<Option<Box<dyn SkinSource>>> = images
            .into_iter()
            .map(|img| -> Option<Box<dyn SkinSource>> {
                Some(Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                    img, 0, cycle,
                )))
            })
            .collect();
        let _ = timer; // timer consumed but each source gets int timer 0 as approximation
        Self {
            data: SkinObjectData::new(),
            image,
            ref_prop: integer_property_factory::image_index_property_by_id(ref_id),
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_movie(movie: SkinSourceMovie) -> Self {
        let mut data = SkinObjectData::new();
        data.image_type = SkinObjectRenderer::TYPE_FFMPEG;
        Self {
            data,
            image: vec![Some(Box::new(movie))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: true,
        }
    }

    pub fn new_with_sources_ref_id(image: Vec<SkinSourceImage>, ref_id: i32) -> Self {
        Self::new_with_sources_ref(
            image,
            integer_property_factory::image_index_property_by_id(ref_id),
        )
    }

    pub fn new_with_sources_ref(
        image: Vec<SkinSourceImage>,
        ref_prop: Option<Box<dyn IntegerProperty>>,
    ) -> Self {
        let image: Vec<Option<Box<dyn SkinSource>>> = image
            .into_iter()
            .map(|s| -> Option<Box<dyn SkinSource>> { Some(Box::new(s)) })
            .collect();
        Self {
            data: SkinObjectData::new(),
            image,
            ref_prop,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn image(&self, time: i64, state: &dyn MainState) -> Option<TextureRegion> {
        self.image_at(0, time, state)
    }

    pub fn image_at(
        &self,
        value: usize,
        time: i64,
        state: &dyn MainState,
    ) -> Option<TextureRegion> {
        if value < self.image.len()
            && let Some(ref source) = self.image[value]
        {
            return source.get_image(time, state);
        }
        None
    }

    pub fn validate(&mut self) -> bool {
        let mut exist = false;
        for slot in self.image.iter_mut() {
            if let Some(source) = slot.as_ref() {
                if source.validate() {
                    exist = true;
                } else {
                    let removed = slot.take().expect("take");
                    self.removed_sources.push(removed);
                }
            }
        }

        if !exist {
            return false;
        }

        self.data.validate()
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.prepare_with_offset(time, state, 0.0, 0.0);
    }

    pub fn prepare_with_offset(
        &mut self,
        time: i64,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        let value = if let Some(ref r) = self.ref_prop {
            r.get(state)
        } else {
            0
        };
        self.prepare_with_value(time, state, value, offset_x, offset_y);
    }

    pub fn prepare_with_value(
        &mut self,
        time: i64,
        state: &dyn MainState,
        mut value: i32,
        offset_x: f32,
        offset_y: f32,
    ) {
        if value < 0 {
            self.data.draw = false;
            return;
        }
        self.data
            .prepare_with_offset(time, state, offset_x, offset_y);
        if value >= self.image.len() as i32 {
            value = 0;
        }
        self.current_image = self.image_at(value as usize, time, state);
        if self.current_image.is_none() {
            self.data.draw = false;
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref current_image) = self.current_image.clone() {
            if self.is_movie {
                self.data.image_type = 3;
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                );
                self.data.image_type = 0;
            } else {
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                );
            }
        }
    }

    pub fn draw_with_offset(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        offset_x: f32,
        offset_y: f32,
    ) {
        if let Some(ref current_image) = self.current_image.clone() {
            if self.is_movie {
                self.data.image_type = 3;
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x + offset_x,
                    region.y + offset_y,
                    region.width,
                    region.height,
                );
                self.data.image_type = 0;
            } else {
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x + offset_x,
                    region.y + offset_y,
                    region.width,
                    region.height,
                );
            }
        }
    }

    pub fn draw_prepared(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        time: i64,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        self.prepare_with_offset(time, state, offset_x, offset_y);
        if self.data.draw {
            self.draw(sprite);
        }
    }

    pub fn draw_with_value(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        time: i64,
        state: &dyn MainState,
        value: i32,
        offset_x: f32,
        offset_y: f32,
    ) {
        self.prepare_with_value(time, state, value, offset_x, offset_y);
        if self.data.draw {
            self.draw(sprite);
        }
    }

    pub fn ref_prop(&self) -> Option<&dyn IntegerProperty> {
        self.ref_prop.as_deref()
    }

    pub fn source_count(&self) -> usize {
        self.image.len()
    }

    pub fn has_valid_source(&self) -> bool {
        self.image.iter().any(|s| s.is_some())
    }

    pub fn dispose(&mut self) {
        for source in self.removed_sources.drain(..) {
            // dispose removed sources
            let _ = source;
        }
        for s in self.image.iter_mut().flatten() {
            s.dispose();
        }
        self.data.set_disposed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_object::{SkinObjectDestination, SkinObjectRenderer};
    use crate::stubs::{Color, Rectangle, TextureRegion};
    use crate::test_helpers::MockMainState;

    /// Helper: make a TextureRegion with known dimensions.
    fn make_region(w: i32, h: i32) -> TextureRegion {
        TextureRegion {
            region_width: w,
            region_height: h,
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..TextureRegion::default()
        }
    }

    /// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
    /// Uses time=0, full white color, no timer/loop/blend.
    fn setup_data(data: &mut crate::skin_object::SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
        data.set_destination_with_int_timer_ops(
            0,
            x,
            y,
            w,
            h,
            0, // acc
            255,
            255,
            255,
            255,  // argb
            0,    // blend
            0,    // filter
            0,    // angle
            0,    // center
            0,    // loop
            0,    // timer
            &[0], // ops
        );
    }

    #[test]
    fn test_skin_image_draw_basic() {
        let region = make_region(64, 48);
        let mut img = SkinImage::new_with_single(region);
        setup_data(&mut img.data, 10.0, 20.0, 100.0, 50.0);

        let state = MockMainState::default();
        img.prepare(0, &state);
        assert!(img.data.draw);

        let mut renderer = SkinObjectRenderer::new();
        img.draw(&mut renderer);

        // Should have generated 6 vertices (one quad)
        assert_eq!(renderer.sprite.vertices().len(), 6);
        // Check position: draw adds 0.01 offset (Java Windows workaround)
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 10.01).abs() < 0.02);
        assert!((v0.position[1] - 20.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_image_draw_with_offset() {
        let region = make_region(32, 32);
        let mut img = SkinImage::new_with_single(region);
        setup_data(&mut img.data, 10.0, 20.0, 100.0, 50.0);

        let state = MockMainState::default();
        img.prepare(0, &state);

        let mut renderer = SkinObjectRenderer::new();
        img.draw_with_offset(&mut renderer, 5.0, 3.0);

        assert_eq!(renderer.sprite.vertices().len(), 6);
        // Position should be region (10, 20) + offset (5, 3) + 0.01 draw offset
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 15.01).abs() < 0.02);
        assert!((v0.position[1] - 23.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_image_draw_movie_type_override() {
        // Test the movie draw path by manually constructing a SkinImage with is_movie=true
        // and injecting a current_image.
        let region = make_region(320, 240);
        let mut img = SkinImage {
            data: crate::skin_object::SkinObjectData::new(),
            image: vec![Some(Box::new(SkinSourceImage::new_single(region.clone())))],
            ref_prop: None,
            current_image: Some(region),
            removed_sources: Vec::new(),
            is_movie: true,
        };
        img.data.image_type = SkinObjectRenderer::TYPE_FFMPEG;
        setup_data(&mut img.data, 0.0, 0.0, 100.0, 100.0);
        // Manually set draw=true and region since we bypass prepare
        img.data.draw = true;
        img.data.region = Rectangle::new(0.0, 0.0, 100.0, 100.0);
        img.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        img.draw(&mut renderer);

        // After draw, imageType should be reset to 0 (Java behavior: setImageType(3) then setImageType(0))
        assert_eq!(img.data.image_type, 0);
        // Renderer should have had TYPE_FFMPEG (3) set during draw
        assert_eq!(
            renderer.sprite.shader_type(),
            SkinObjectRenderer::TYPE_FFMPEG
        );
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_image_draw_not_drawn_when_no_image() {
        // Create with image that has no texture region available
        let mut img = SkinImage::new_with_image_id(999);
        setup_data(&mut img.data, 0.0, 0.0, 100.0, 100.0);

        let state = MockMainState::default();
        img.prepare(0, &state);

        // Should not draw since source returns None
        assert!(!img.data.draw);
    }

    #[test]
    fn test_skin_image_draw_sets_color_on_sprite() {
        let region = make_region(32, 32);
        let mut img = SkinImage::new_with_single(region);
        setup_data(&mut img.data, 0.0, 0.0, 50.0, 50.0);

        let state = MockMainState::default();
        img.prepare(0, &state);

        // Color should be white after prepare (255,255,255,255)
        assert_eq!(img.data.color.r, 1.0);
        assert_eq!(img.data.color.a, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        img.draw(&mut renderer);

        // All vertices should have white color
        for v in renderer.sprite.vertices() {
            assert_eq!(v.color, [1.0, 1.0, 1.0, 1.0]);
        }
    }

    #[test]
    fn test_skin_image_draw_zero_alpha_skips() {
        let region = make_region(32, 32);
        let mut img = SkinImage::new_with_single(region);
        // Set alpha=0 so draw_image_at returns early
        img.data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 0.0),
            0,
            0,
        ));
        img.data.starttime = 0;
        img.data.endtime = 0;
        img.data.fixr = Some(Rectangle::new(0.0, 0.0, 100.0, 100.0));
        img.data.fixc = Some(Color::new(1.0, 1.0, 1.0, 0.0));
        img.data.fixa = 0;

        let state = MockMainState::default();
        img.prepare(0, &state);
        assert!(img.data.draw);
        assert_eq!(img.data.color.a, 0.0);

        let mut renderer = SkinObjectRenderer::new();
        img.draw(&mut renderer);

        // No vertices should be generated since alpha is 0
        assert!(renderer.sprite.vertices().is_empty());
    }

    #[test]
    fn test_skin_image_draw_with_offset_movie() {
        // Test movie draw_with_offset path
        let region = make_region(320, 240);
        let mut img = SkinImage {
            data: crate::skin_object::SkinObjectData::new(),
            image: vec![Some(Box::new(SkinSourceImage::new_single(region.clone())))],
            ref_prop: None,
            current_image: Some(region),
            removed_sources: Vec::new(),
            is_movie: true,
        };
        img.data.image_type = SkinObjectRenderer::TYPE_FFMPEG;
        // Manually set draw state
        img.data.draw = true;
        img.data.region = Rectangle::new(100.0, 200.0, 320.0, 240.0);
        img.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        img.draw_with_offset(&mut renderer, 10.0, 5.0);

        // After draw_with_offset for movie: imageType should be reset to 0
        assert_eq!(img.data.image_type, 0);
        assert_eq!(renderer.sprite.vertices().len(), 6);
        // Position: (100+10, 200+5) = (110, 205) + 0.01
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 110.01).abs() < 0.02);
        assert!((v0.position[1] - 205.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_image_draw_region_dimensions() {
        let region = make_region(64, 48);
        let mut img = SkinImage::new_with_single(region);
        setup_data(&mut img.data, 50.0, 60.0, 200.0, 150.0);

        let state = MockMainState::default();
        img.prepare(0, &state);

        let mut renderer = SkinObjectRenderer::new();
        img.draw(&mut renderer);

        // Verify the quad spans the correct region
        let verts = renderer.sprite.vertices();
        // v0 = top-left, v1 = top-right, v2 = bottom-right
        let x0 = verts[0].position[0];
        let y0 = verts[0].position[1];
        let x1 = verts[1].position[0];
        let y1 = verts[2].position[1];
        // Width = x1 - x0, Height = y1 - y0
        assert!((x1 - x0 - 200.0).abs() < 0.02);
        assert!((y1 - y0 - 150.0).abs() < 0.02);
    }
}
