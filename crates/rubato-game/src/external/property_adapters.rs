// IntegerProperty / BooleanProperty / StringProperty
// Delegate to skin's real property factories via MainState trait bridge.

use crate::external::main_state_adapter::MainState;

/// Property trait wrapping skin's IntegerProperty for external's &MainState callers.
pub trait IntegerProperty {
    fn get(&self, state: &MainState) -> i32;
}

/// Property trait wrapping skin's BooleanProperty for external's &MainState callers.
pub trait BooleanProperty {
    fn get(&self, state: &MainState) -> bool;
}

/// Property trait wrapping skin's StringProperty for external's &MainState callers.
pub trait StringProperty {
    fn get(&self, state: &MainState) -> String;
}

// --- Wrapper adapters that delegate to skin's real property traits ---

struct SkinIntegerPropertyAdapter(
    Box<dyn rubato_skin::property::integer_property::IntegerProperty>,
);
impl IntegerProperty for SkinIntegerPropertyAdapter {
    fn get(&self, state: &MainState) -> i32 {
        self.0.get(state)
    }
}

struct SkinBooleanPropertyAdapter(
    Box<dyn rubato_skin::property::boolean_property::BooleanProperty>,
);
impl BooleanProperty for SkinBooleanPropertyAdapter {
    fn get(&self, state: &MainState) -> bool {
        self.0.get(state)
    }
}

struct SkinStringPropertyAdapter(Box<dyn rubato_skin::property::string_property::StringProperty>);
impl StringProperty for SkinStringPropertyAdapter {
    fn get(&self, state: &MainState) -> String {
        self.0.get(state)
    }
}

// --- Default fallbacks for IDs not found in skin's factory ---

struct DefaultIntegerProperty;
impl IntegerProperty for DefaultIntegerProperty {
    fn get(&self, _state: &MainState) -> i32 {
        0
    }
}

struct DefaultBooleanProperty;
impl BooleanProperty for DefaultBooleanProperty {
    fn get(&self, _state: &MainState) -> bool {
        false
    }
}

struct DefaultStringProperty;
impl StringProperty for DefaultStringProperty {
    fn get(&self, _state: &MainState) -> String {
        String::new()
    }
}

// --- Factory facades matching original API ---

pub struct IntegerPropertyFactory;
impl IntegerPropertyFactory {
    pub fn integer_property(id: i32) -> Box<dyn IntegerProperty> {
        match rubato_skin::property::integer_property_factory::integer_property_by_id(id) {
            Some(prop) => Box::new(SkinIntegerPropertyAdapter(prop)),
            None => Box::new(DefaultIntegerProperty),
        }
    }
}

pub struct BooleanPropertyFactory;
impl BooleanPropertyFactory {
    pub fn boolean_property(id: i32) -> Box<dyn BooleanProperty> {
        match rubato_skin::property::boolean_property_factory::boolean_property(id) {
            Some(prop) => Box::new(SkinBooleanPropertyAdapter(prop)),
            None => Box::new(DefaultBooleanProperty),
        }
    }
}

pub struct StringPropertyFactory;
impl StringPropertyFactory {
    pub fn string_property(id: i32) -> Box<dyn StringProperty> {
        match rubato_skin::property::string_property_factory::string_property_by_id(id) {
            Some(prop) => Box::new(SkinStringPropertyAdapter(prop)),
            None => Box::new(DefaultStringProperty),
        }
    }
}
