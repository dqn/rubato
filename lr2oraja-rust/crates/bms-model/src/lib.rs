//! BMS chart data model: parsing, note types, timeline, and play mode definitions.
//!
//! Provides [`BmsDecoder`] and [`BmsonDecoder`] for decoding `.bms`/`.bmson` files,
//! [`OsuDecoder`] for osu!mania format, and the core [`BmsModel`] struct that holds
//! the parsed chart data including [`TimeLine`], [`Note`], and [`BgNote`] collections.
//! Other crates consume `BmsModel` as the single source of truth for chart information.

#[allow(dead_code)] // Parsed for completeness (full bmson format model)
mod bmson;
mod bmson_decode;
mod decode_log;
pub mod lane_property;
mod mode;
mod model;
mod note;
mod osu;
mod osu_decode;
mod parse;
mod timeline;

pub use bmson_decode::BmsonDecoder;
pub use decode_log::{DecodeLog, LogLevel};
pub use lane_property::LaneProperty;
pub use mode::PlayMode;
pub use model::{BmsModel, JudgeRankType, NoteFilter, Side, TotalType};
pub use note::{BgNote, LnType, Note, NoteType};
pub use osu_decode::OsuDecoder;
pub use parse::BmsDecoder;
pub use timeline::{BgaEvent, BgaLayer, BpmChange, StopEvent, TimeLine};
