use bms_model::bms_model::BMSModel;

use crate::song_data::SongData;
use crate::song_information::SongInformation;

/// Song information database accessor interface.
///
/// Defines the cross-crate contract for querying and updating song information.
/// Concrete implementation lives in beatoraja-song (SongInformationAccessor).
/// The trait lives in beatoraja-types to avoid circular dependencies
/// (beatoraja-core cannot depend on beatoraja-song).
pub trait SongInformationDb: Send + Sync {
    /// Query informations by custom SQL WHERE clause
    fn get_informations(&self, sql: &str) -> Vec<SongInformation>;

    /// Query single record by SHA256
    fn get_information(&self, sha256: &str) -> Option<SongInformation>;

    /// Batch-load song information for an array of songs (chunked)
    fn get_information_for_songs(&self, songs: &mut [SongData]);

    /// Begin update transaction
    fn start_update(&self) -> anyhow::Result<()>;

    /// Insert/replace information record computed from a BMS model
    fn update(&self, model: &BMSModel);

    /// Commit update transaction
    fn end_update(&self);
}
