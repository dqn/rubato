/// Disposable trait - equivalent to LibGDX Disposable
pub trait Disposable {
    fn dispose(&mut self);
}

/// DisposableObject - abstract base for objects with explicit disposal
pub struct DisposableObject {
    disposed: bool,
}

impl DisposableObject {
    pub fn new() -> Self {
        Self { disposed: false }
    }

    pub fn is_disposed(&self) -> bool {
        self.disposed
    }

    pub fn is_not_disposed(&self) -> bool {
        !self.disposed
    }

    pub fn set_disposed(&mut self) {
        self.disposed = true;
    }

    pub fn dispose_all(objects: &mut [Option<&mut DisposableObject>]) {
        for obj in objects.iter_mut() {
            if let Some(o) = obj
                && o.is_not_disposed()
            {
                o.set_disposed();
            }
        }
    }
}

impl Default for DisposableObject {
    fn default() -> Self {
        Self::new()
    }
}
