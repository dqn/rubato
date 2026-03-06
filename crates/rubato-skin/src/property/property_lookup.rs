//! Macros for common property registry lookup patterns.
//!
//! The property factories (integer, float, string, event) all share a pattern
//! of scanning a static slice of entries to find a match by `id` or `name`,
//! then constructing a boxed delegate. These macros eliminate that boilerplate.
//!
//! The boolean property factory is intentionally excluded because its lookup
//! logic uses complex match-based branching rather than registry scans.

/// Scan `$entries` for an entry whose `id` field matches `$id`,
/// then return `Some(Box::new($delegate { id: entry.id }))`.
///
/// Expands to a block that may `return` early from the enclosing function.
macro_rules! find_by_id {
    ($entries:expr, $id:expr, $delegate:ident) => {
        for entry in $entries.iter() {
            if entry.id == $id {
                return Some(Box::new($delegate { id: entry.id }));
            }
        }
    };
}

/// Scan `$entries` for an entry whose `name` field matches `$name`,
/// then return `Some(Box::new($delegate { id: entry.id }))`.
///
/// Expands to a block that may `return` early from the enclosing function.
macro_rules! find_by_name {
    ($entries:expr, $name:expr, $delegate:ident) => {
        for entry in $entries.iter() {
            if entry.name == $name {
                return Some(Box::new($delegate { id: entry.id }));
            }
        }
    };
}

pub(crate) use find_by_id;
pub(crate) use find_by_name;
