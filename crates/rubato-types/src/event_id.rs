/// Semantic newtype for skin event IDs.
///
/// Wraps the raw `i32` used as event IDs in the skin event system
/// (`event_factory`, `execute_event`, custom events, etc.).
///
/// Provides `From<i32>` / `Into<i32>` for gradual migration at crate boundaries.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct EventId(pub i32);

impl EventId {
    /// Sentinel for "no event ID" / unknown.
    pub const UNDEFINED: EventId = EventId(i32::MIN);

    /// Create a new EventId from a raw i32.
    pub fn new(id: i32) -> Self {
        Self(id)
    }

    /// Extract the raw i32.
    pub fn as_i32(self) -> i32 {
        self.0
    }

    /// Convert to a `usize` index, returning `None` for negative IDs.
    pub fn as_index(self) -> Option<usize> {
        if self.0 >= 0 {
            Some(self.0 as usize)
        } else {
            None
        }
    }
}

impl From<i32> for EventId {
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<EventId> for i32 {
    fn from(id: EventId) -> Self {
        id.0
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EventId({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_as_i32() {
        let id = EventId::new(42);
        assert_eq!(id.as_i32(), 42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_from_i32() {
        let id: EventId = 99.into();
        assert_eq!(id.as_i32(), 99);
    }

    #[test]
    fn test_into_i32() {
        let id = EventId::new(7);
        let raw: i32 = id.into();
        assert_eq!(raw, 7);
    }

    #[test]
    fn test_as_index() {
        assert_eq!(EventId::new(0).as_index(), Some(0));
        assert_eq!(EventId::new(100).as_index(), Some(100));
        assert_eq!(EventId::new(-1).as_index(), None);
        assert_eq!(EventId::UNDEFINED.as_index(), None);
    }

    #[test]
    fn test_undefined() {
        assert_eq!(EventId::UNDEFINED.as_i32(), i32::MIN);
    }

    #[test]
    fn test_default() {
        assert_eq!(EventId::default(), EventId(0));
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", EventId::new(42)), "EventId(42)");
    }

    #[test]
    fn test_ord() {
        assert!(EventId::new(1) < EventId::new(2));
        assert!(EventId::new(5) > EventId::new(3));
    }
}
