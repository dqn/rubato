//! External service integrations: Discord RPC, OBS WebSocket, screenshots, and webhooks.
//!
//! Provides [`discord::DiscordRpcClient`] for rich presence updates,
//! [`obs::ObsWebSocketClient`] for scene/source control, [`screenshot`] for
//! capture and social export, [`webhook`] for HTTP notifications,
//! [`bms_search::BmsSearchClient`] for song discovery, and
//! [`version_check::GithubVersionChecker`] for update notifications.

pub mod bms_search;
pub mod discord;
pub mod obs;
pub mod score_importer;
pub mod screenshot;
pub mod version_check;
pub mod webhook;
