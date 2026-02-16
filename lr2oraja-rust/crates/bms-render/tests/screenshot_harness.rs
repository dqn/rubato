// Headless rendering harness for screenshot tests.
//
// Uses Bevy 0.15 without WinitPlugin, rendering to an off-screen Image.
// This avoids the macOS requirement that EventLoop must be on the main thread.

use bevy::prelude::*;
use bevy::render::camera::RenderTarget;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use bms_render::font_map::FontMap;
use bms_render::plugin::register_render_materials;
use bms_render::skin_renderer::{setup_skin, skin_render_system};
use bms_render::state_provider::SkinStateProvider;
use bms_render::texture_map::TextureMap;
use bms_skin::image_handle::ImageHandle;
use bms_skin::skin::Skin;

use std::collections::{HashMap, HashSet};
use std::path::Path;

/// Handle to the off-screen render target image.
#[derive(Resource, Clone)]
struct RenderTargetHandle(Handle<Image>);

/// Temporary resource to hold uploaded textures before setup_skin is called.
#[derive(Resource, Default)]
struct PendingTextures {
    entries: Vec<(ImageHandle, Handle<Image>, f32, f32)>,
}

/// Render test harness for capturing headless screenshots.
pub struct RenderTestHarness {
    app: App,
    next_handle_id: u32,
}

impl RenderTestHarness {
    /// Creates a new headless rendering harness.
    pub fn new(width: u32, height: u32) -> Self {
        let mut app = App::new();

        // Use DefaultPlugins but disable WinitPlugin to avoid EventLoop creation.
        // On macOS, winit requires EventLoop on the main thread, but tests run
        // on worker threads. We render to an off-screen Image instead.
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::winit::WinitPlugin>()
                .set(WindowPlugin {
                    primary_window: None,
                    ..default()
                })
                .set(bevy::render::RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        );

        // Register embedded shaders and Material2d plugins for DistanceFieldMaterial
        // and BgaLayerMaterial (both required by skin_render_system parameters).
        register_render_materials(&mut app);
        app.add_systems(Update, skin_render_system);

        // Finalize all plugins — this calls Plugin::finish() on every registered
        // plugin, which inserts critical resources like CapturedScreenshots.
        // Without this, app.update() alone will panic on missing resources.
        app.finish();
        app.cleanup();

        // Create off-screen render target image
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let mut render_image = Image::new(
            size,
            TextureDimension::D2,
            vec![0u8; (width * height * 4) as usize],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );
        render_image.texture_descriptor.usage = TextureUsages::COPY_DST
            | TextureUsages::COPY_SRC
            | TextureUsages::RENDER_ATTACHMENT
            | TextureUsages::TEXTURE_BINDING;

        let image_handle = app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(render_image);

        // Spawn camera targeting the off-screen image
        app.world_mut().spawn((
            Camera2d,
            Camera {
                target: RenderTarget::Image(image_handle.clone()),
                ..default()
            },
        ));

        app.world_mut()
            .insert_resource(RenderTargetHandle(image_handle));

        Self {
            app,
            next_handle_id: 0,
        }
    }

    /// Upload an RGBA image into Bevy's asset system.
    pub fn upload_image(&mut self, rgba: &image::RgbaImage) {
        let id = self.next_handle_id;
        self.next_handle_id += 1;

        let size = Extent3d {
            width: rgba.width(),
            height: rgba.height(),
            depth_or_array_layers: 1,
        };
        let bevy_image = Image::new(
            size,
            TextureDimension::D2,
            rgba.as_raw().clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        let handle = self
            .app
            .world_mut()
            .resource_mut::<Assets<Image>>()
            .add(bevy_image);

        if !self.app.world().contains_resource::<PendingTextures>() {
            self.app
                .world_mut()
                .insert_resource(PendingTextures::default());
        }
        self.app
            .world_mut()
            .resource_mut::<PendingTextures>()
            .entries
            .push((
                ImageHandle(id),
                handle,
                rgba.width() as f32,
                rgba.height() as f32,
            ));
    }

    /// Set up the skin with the given skin data and state provider.
    pub fn setup_skin(&mut self, skin: Skin, state_provider: Box<dyn SkinStateProvider>) {
        let mut texture_map = TextureMap::new();
        if let Some(pending) = self.app.world_mut().remove_resource::<PendingTextures>() {
            for (img_handle, bevy_handle, w, h) in pending.entries {
                texture_map.insert(img_handle, bevy_handle, w, h);
            }
        }

        // Load embedded textures and LR2 bitmap fonts.
        let mut font_map = FontMap::new();
        {
            let mut images = self.app.world_mut().resource_mut::<Assets<Image>>();
            bms_render::embedded_textures::load_embedded_textures(&mut images, &mut texture_map);
            font_map.load_lr2_fonts(&skin, &mut images);
        }
        let mut commands = self.app.world_mut().commands();
        setup_skin(&mut commands, skin, texture_map, font_map, state_provider);
        self.app.world_mut().flush();
    }

    /// Load a JSON skin file, resolve source images, and set up for rendering.
    pub fn load_json_skin(
        &mut self,
        skin_json_path: &Path,
        state_provider: Box<dyn SkinStateProvider>,
    ) {
        self.load_json_skin_with_resolution(
            skin_json_path,
            state_provider,
            bms_config::resolution::Resolution::Sd,
        );
    }

    /// Load a JSON skin file with a specific destination resolution.
    pub fn load_json_skin_with_resolution(
        &mut self,
        skin_json_path: &Path,
        state_provider: Box<dyn SkinStateProvider>,
        dest_resolution: bms_config::resolution::Resolution,
    ) {
        let json_str = std::fs::read_to_string(skin_json_path).unwrap_or_else(|e| {
            panic!(
                "Failed to read skin JSON {}: {}",
                skin_json_path.display(),
                e
            )
        });

        let preprocessed = bms_skin::loader::json_loader::preprocess_json(&json_str);
        let skin_dir = skin_json_path.parent().unwrap();

        self.load_skin_from_json_str(
            &json_str,
            &preprocessed,
            skin_dir,
            Some(skin_json_path),
            state_provider,
            dest_resolution,
        );
    }

    /// Load a Lua skin file with a specific destination resolution.
    pub fn load_lua_skin_with_resolution(
        &mut self,
        lua_path: &Path,
        state_provider: Box<dyn SkinStateProvider>,
        dest_resolution: bms_config::resolution::Resolution,
    ) {
        let lua_source = std::fs::read_to_string(lua_path)
            .unwrap_or_else(|e| panic!("Failed to read Lua skin {}: {}", lua_path.display(), e));

        let json_str = bms_skin::loader::lua_loader::lua_to_json_string(
            &lua_source,
            Some(lua_path),
            &HashSet::new(),
            &[],
            None,
        )
        .unwrap_or_else(|e| panic!("Failed to convert Lua skin to JSON: {}", e));

        let skin_dir = lua_path.parent().unwrap();

        self.load_skin_from_json_str(
            &json_str,
            &json_str, // Lua output is already valid JSON, no preprocessing needed
            skin_dir,
            Some(lua_path),
            state_provider,
            dest_resolution,
        );
    }

    /// Common logic for loading a skin from a JSON string.
    fn load_skin_from_json_str(
        &mut self,
        json_str: &str,
        preprocessed: &str,
        skin_dir: &Path,
        skin_path: Option<&Path>,
        state_provider: Box<dyn SkinStateProvider>,
        dest_resolution: bms_config::resolution::Resolution,
    ) {
        let raw: serde_json::Value = serde_json::from_str(preprocessed)
            .unwrap_or_else(|e| panic!("Failed to parse skin JSON: {}", e));

        // Extract source definitions and load images
        let mut source_images: HashMap<String, ImageHandle> = HashMap::new();
        if let Some(sources) = raw.get("source").and_then(|v| v.as_array()) {
            for src in sources {
                let id = match src.get("id") {
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::String(s)) => s.clone(),
                    _ => continue,
                };
                let path_str = match src.get("path").and_then(|v| v.as_str()) {
                    Some(p) => p,
                    None => continue,
                };
                let img_path = skin_dir.join(path_str);

                // Try glob expansion for wildcard paths (e.g., "background/*.png")
                let paths_to_try = if path_str.contains('*') {
                    if let Some(pattern) = img_path.to_str() {
                        glob::glob(pattern)
                            .ok()
                            .map(|entries| entries.filter_map(|e| e.ok()).collect::<Vec<_>>())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    }
                } else {
                    vec![img_path]
                };

                for actual_path in paths_to_try {
                    match image::open(&actual_path) {
                        Ok(img) => {
                            let rgba = img.to_rgba8();
                            self.upload_image(&rgba);
                            let handle = ImageHandle(self.next_handle_id - 1);
                            source_images.insert(id.clone(), handle);
                            break; // Use first matching image
                        }
                        Err(_) => {
                            // Skip missing/unreadable source images gracefully
                        }
                    }
                }
            }
        }

        // Load skin with resolved images
        let skin = bms_skin::loader::json_loader::load_skin_with_images(
            json_str,
            &HashSet::new(),
            dest_resolution,
            skin_path,
            &source_images,
        )
        .unwrap_or_else(|e| panic!("Failed to load skin: {}", e));

        self.setup_skin(skin, state_provider);
    }

    /// Run pre-roll frames, capture a screenshot, and save to disk.
    pub fn capture_frame(&mut self, output_path: &Path) {
        // Run pre-roll frames to let rendering pipeline stabilize.
        // Complex skins with many entities need more frames for GPU upload.
        for _ in 0..60 {
            self.app.update();
        }

        // Capture from the off-screen render target
        let handle = self.app.world().resource::<RenderTargetHandle>().0.clone();
        self.app
            .world_mut()
            .commands()
            .spawn(Screenshot::image(handle))
            .observe(save_to_disk(output_path.to_path_buf()));
        self.app.world_mut().flush();

        // Run frames to complete capture and disk write
        for _ in 0..60 {
            self.app.update();
        }
    }
}
