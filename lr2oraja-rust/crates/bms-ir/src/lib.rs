//! Internet ranking (IR) client for score submission and leaderboard retrieval.
//!
//! Provides [`IRConnection`] trait and [`LR2IRConnection`] for the LR2IR protocol,
//! [`IRConnectionManager`] for managing active sessions, and data types such as
//! [`IRScoreData`], [`IRChartData`], [`RankingData`], and [`LeaderboardEntry`].
//! Used by the play and result states to submit scores and fetch rival rankings.

pub mod account;
pub mod chart_data;
pub mod connection;
pub mod connection_manager;
pub mod course_data;
pub mod leaderboard;
pub mod lr2ir;
pub mod player_data;
pub mod ranking_cache;
pub mod ranking_data;
pub mod response;
pub mod score_data;
pub mod table_data;

pub use account::IRAccount;
pub use chart_data::IRChartData;
pub use connection::IRConnection;
pub use connection_manager::IRConnectionManager;
pub use course_data::{CourseDataConstraint, IRCourseData, IRTrophyData};
pub use leaderboard::{IRType, LeaderboardEntry};
pub use lr2ir::LR2IRConnection;
pub use player_data::IRPlayerData;
pub use ranking_cache::RankingDataCache;
pub use ranking_data::{RankingData, RankingState};
pub use response::IRResponse;
pub use score_data::IRScoreData;
pub use table_data::{IRTableData, IRTableFolder};
