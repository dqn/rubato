#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(clippy::if_same_then_else)]

// beatoraja-external: External integrations (screenshot, webhook, BMS search, Discord, score import)

// Discord RPC (merged from discord-rpc crate)
pub mod discord_rpc;

// OBS WebSocket (merged from beatoraja-obs crate)
pub mod obs;

// Stubs for external dependencies not yet available
pub mod stubs;

// Real implementations (moved from stubs.rs in Phase 25a)
pub mod clipboard_helper;
pub mod pixmap_io;

// BMS Search API accessor
pub mod bms_search_accessor;

// Discord Rich Presence listener
pub mod discord_listener;

// Score data import from LR2
pub mod score_data_importer;

// Screenshot export interface (trait)
pub mod screen_shot_exporter;

// Screenshot file exporter
pub mod screen_shot_file_exporter;

// Screenshot Twitter exporter
pub mod screen_shot_twitter_exporter;

// Webhook handler for Discord webhooks
pub mod webhook_handler;
