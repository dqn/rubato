//! Lifecycle and cross-crate bridge traits facade.
//!
//! Re-exports types from main_controller_access, player_resource_access,
//! input_processor_access, stream_controller_access, music_download_access,
//! obs_access, imgui_access, imgui_notify, ir_resend_service, ir_rival_provider,
//! ranking_data_cache_access, http_download_submitter, and table_update_source modules.

pub use crate::http_download_submitter::*;
pub use crate::imgui_access::*;
pub use crate::imgui_notify::*;
pub use crate::input_processor_access::*;
pub use crate::ir_resend_service::*;
pub use crate::ir_rival_provider::*;
pub use crate::main_controller_access::*;
pub use crate::music_download_access::*;
pub use crate::obs_access::*;
pub use crate::player_resource_access::*;
pub use crate::ranking_data_cache_access::*;
pub use crate::stream_controller_access::*;
pub use crate::table_update_source::*;
