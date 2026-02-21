use std::collections::HashMap;
use std::sync::Mutex;

/// ShaderProgram stub (LibGDX equivalent)
pub struct ShaderProgram {
    pub name: String,
}

impl ShaderProgram {
    pub fn dispose(&mut self) {
        // LibGDX ShaderProgram.dispose()
    }
}

static SHADERS: OnceLockShaders = OnceLockShaders::new();

struct OnceLockShaders {
    inner: std::sync::OnceLock<Mutex<HashMap<String, ShaderProgram>>>,
}

impl OnceLockShaders {
    const fn new() -> Self {
        Self {
            inner: std::sync::OnceLock::new(),
        }
    }

    fn get(&self) -> &Mutex<HashMap<String, ShaderProgram>> {
        self.inner.get_or_init(|| Mutex::new(HashMap::new()))
    }
}

/// ShaderManager - manages shader programs
pub struct ShaderManager;

impl ShaderManager {
    pub fn get_shader(name: &str) -> Option<()> {
        let shaders = SHADERS.get().lock().unwrap();
        if !shaders.contains_key(name) {
            // In Java:
            // ShaderProgram shader = new ShaderProgram(
            //     Gdx.files.classpath("glsl/" + name + ".vert"),
            //     Gdx.files.classpath("glsl/" + name + ".frag"));
            // if(shader.isCompiled()) { shaders.put(name, shader); return shader; }
            // Phase 5+ dependency: LibGDX shader compilation
            return None;
        }
        if shaders.contains_key(name) {
            Some(())
        } else {
            None
        }
    }

    pub fn dispose() {
        let mut shaders = SHADERS.get().lock().unwrap();
        for (_name, mut shader) in shaders.drain() {
            shader.dispose();
        }
    }
}
