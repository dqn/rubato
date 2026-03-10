// Static lookup tables for integer property factory.
// Extracted from integer_property_factory.rs for navigability.

use rubato_types::value_id::ValueId;

pub(super) struct ValueTypeEntry {
    pub(super) id: ValueId,
    pub(super) name: &'static str,
}

pub(super) struct IndexTypeEntry {
    pub(super) id: ValueId,
    pub(super) name: &'static str,
}
