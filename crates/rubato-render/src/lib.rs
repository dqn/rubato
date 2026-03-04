pub mod blend;
pub mod color;
pub mod egui_integration;
pub mod font;
pub mod glyph_atlas;
pub mod gpu_context;
pub mod gpu_texture_manager;
pub mod pixmap;
pub mod render_pipeline;
pub mod shader;
pub mod sprite_batch;
pub mod texture;

pub use blend::*;
pub use color::*;
pub use egui_integration::*;
pub use font::*;
pub use gpu_context::*;
pub use gpu_texture_manager::*;
pub use pixmap::*;
pub use render_pipeline::*;
pub use shader::*;
pub use sprite_batch::*;
pub use texture::*;

// Re-export LibGDX file types that are pure data and belong in the render crate.

/// Stub for com.badlogic.gdx.files.FileHandle
#[derive(Clone, Debug, Default)]
pub struct FileHandle {
    pub path: String,
}

impl FileHandle {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub fn exists(&self) -> bool {
        std::path::Path::new(&self.path).exists()
    }

    pub fn name(&self) -> &str {
        std::path::Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    }

    pub fn extension(&self) -> &str {
        std::path::Path::new(&self.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn parent(&self) -> FileHandle {
        let p = std::path::Path::new(&self.path);
        FileHandle {
            path: p
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    pub fn child(&self, name: &str) -> FileHandle {
        let p = std::path::Path::new(&self.path).join(name);
        FileHandle {
            path: p.to_string_lossy().into_owned(),
        }
    }

    pub fn sibling(&self, name: &str) -> FileHandle {
        self.parent().child(name)
    }

    pub fn list(&self) -> Vec<FileHandle> {
        vec![]
    }

    pub fn read_string(&self) -> String {
        std::fs::read_to_string(&self.path).unwrap_or_default()
    }
}

/// Global accessor matching LibGDX's Gdx class.
pub struct Gdx;

impl Gdx {
    pub fn files_internal(path: &str) -> FileHandle {
        FileHandle::new(path)
    }
}
