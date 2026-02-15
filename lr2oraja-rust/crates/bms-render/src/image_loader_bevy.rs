// Bevy-backed image loader implementing the bms-skin ImageLoader trait.
//
// Loads images from disk using the `image` crate, converts them to Bevy
// textures, and registers them in Assets<Image>.

use std::path::Path;

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bms_skin::image_handle::{ImageHandle, ImageLoader};

use crate::texture_map::TextureMap;

/// Bevy-backed image loader that loads images from disk, converts them
/// to Bevy textures, and registers them in a TextureMap.
pub struct BevyImageLoader<'a> {
    next_id: u32,
    images: &'a mut Assets<Image>,
    texture_map: &'a mut TextureMap,
    filter: i32,
}

impl<'a> BevyImageLoader<'a> {
    /// Creates a new loader.
    ///
    /// - `images`: Bevy image assets storage
    /// - `texture_map`: texture map to populate
    /// - `filter`: 0 = nearest, 1 = linear
    pub fn new(
        images: &'a mut Assets<Image>,
        texture_map: &'a mut TextureMap,
        filter: i32,
    ) -> Self {
        Self {
            next_id: 0,
            images,
            texture_map,
            filter,
        }
    }
}

impl ImageLoader for BevyImageLoader<'_> {
    fn load(&mut self, path: &Path) -> Option<ImageHandle> {
        let img = image::open(path).ok()?.into_rgba8();
        self.register_image(img)
    }

    fn load_with_color_key(&mut self, path: &Path) -> Option<ImageHandle> {
        let mut img = image::open(path).ok()?.into_rgba8();
        let (width, height) = img.dimensions();
        if width > 0 && height > 0 {
            // Bottom-right pixel is the color key
            let key = *img.get_pixel(width - 1, height - 1);
            let key_rgb = [key[0], key[1], key[2]];
            for pixel in img.pixels_mut() {
                if pixel[0] == key_rgb[0] && pixel[1] == key_rgb[1] && pixel[2] == key_rgb[2] {
                    pixel[3] = 0; // set alpha to transparent
                }
            }
        }
        self.register_image(img)
    }

    fn dimensions(&self, handle: ImageHandle) -> Option<(f32, f32)> {
        self.texture_map.dimensions(handle)
    }
}

impl BevyImageLoader<'_> {
    fn register_image(&mut self, img: image::RgbaImage) -> Option<ImageHandle> {
        let (width, height) = img.dimensions();
        let raw = img.into_raw();

        let mut bevy_image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            raw,
            TextureFormat::Rgba8UnormSrgb,
            default(),
        );

        bevy_image.sampler = if self.filter == 0 {
            ImageSampler::nearest()
        } else {
            ImageSampler::linear()
        };

        let handle_id = ImageHandle(self.next_id);
        self.next_id += 1;

        let bevy_handle = self.images.add(bevy_image);
        self.texture_map
            .insert(handle_id, bevy_handle, width as f32, height as f32);

        Some(handle_id)
    }
}
