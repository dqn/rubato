// SongDatabaseAccessor trait moved to beatoraja-types (Phase 15c)
pub use crate::song_database_accessor::SongDatabaseAccessor;

// Re-export types used in update methods
pub use crate::song::song_database_update_listener::SongDatabaseUpdateListener;
pub use crate::song::song_information_accessor::SongInformationAccessor;
