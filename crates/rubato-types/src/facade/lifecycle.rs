//! Lifecycle and cross-crate bridge traits facade.
//!
//! Re-exports types from main_controller_access, player_resource_access,
//! input_processor_access, stream_controller_access, music_download_access,
//! obs_access, imgui_access, imgui_notify, ir_resend_service, ir_rival_provider,
//! ranking_data_cache_access, http_download_submitter, and table_update_source modules.

pub use crate::input_processor_access::*;
pub use crate::player_resource_access::*;
