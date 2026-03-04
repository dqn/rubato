#![allow(clippy::manual_find)]
#![allow(clippy::comparison_chain)]
#![allow(dead_code)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::if_same_then_else)]
#![allow(unused_imports)]
#![allow(unused_variables)]

// Stubs for external dependencies
pub mod stubs;

// IR types (moved from stubs.rs)
pub mod ir_initializer;
pub mod ir_resend;
pub mod ir_send_status;
pub mod ir_status;

// Result screen modules
pub mod abstract_result;
pub mod course_result;
pub mod course_result_skin;
pub mod music_result;
pub mod music_result_skin;
pub mod result_key_property;
pub mod skin_gauge_graph_object;
