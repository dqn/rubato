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
pub mod music_result;
pub mod result_key_property;
pub mod result_skin_data;
pub mod skin_gauge_graph_object;

// Backward-compatible re-exports: both old skin modules now alias ResultSkinData.
pub mod music_result_skin {
    pub type MusicResultSkin = super::result_skin_data::ResultSkinData;
}
pub mod course_result_skin {
    pub type CourseResultSkin = super::result_skin_data::ResultSkinData;
}
