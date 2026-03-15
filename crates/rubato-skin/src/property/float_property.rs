use crate::lua::skin_lua_accessor::LuaFloatProperty;
use crate::property::float_property_factory::DelegateFloatProperty;
use crate::reexports::MainState;
use crate::types::skin_object::RateProperty;

pub trait FloatProperty: Send + Sync {
    fn get(&self, state: &dyn MainState) -> f32;

    /// Returns the property ID, or `i32::MIN` if unknown.
    fn get_id(&self) -> i32 {
        i32::MIN
    }
}

/// Enum dispatch for FloatProperty, replacing `Box<dyn FloatProperty>`.
pub enum FloatPropertyEnum {
    Delegate(DelegateFloatProperty),
    Rate(RateProperty),
    Lua(LuaFloatProperty),
}

impl FloatProperty for FloatPropertyEnum {
    fn get(&self, state: &dyn MainState) -> f32 {
        match self {
            Self::Delegate(inner) => inner.get(state),
            Self::Rate(inner) => inner.get(state),
            Self::Lua(inner) => inner.get(state),
        }
    }

    fn get_id(&self) -> i32 {
        match self {
            Self::Delegate(inner) => inner.get_id(),
            Self::Rate(inner) => inner.get_id(),
            Self::Lua(inner) => inner.get_id(),
        }
    }
}
