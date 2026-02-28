// Phase 7: Cross-Subsystem Lifecycle — Document OnceLock limitations
//
// MainLoader uses OnceLock<Mutex<Option<...>>> for SONGDB, which means the
// OnceLock itself can only be initialized once (creating the Mutex), but
// the inner Option can be swapped via Mutex. This is a deliberate design
// choice that allows re-setting the song DB via set/take/clear methods.
//
// However, the OnceLock pattern means a process-lifetime commitment to the
// Mutex wrapper — the Mutex itself can never be replaced or freed.

use beatoraja_core::main_loader::MainLoader;
use beatoraja_types::folder_data::FolderData;
use beatoraja_types::song_data::SongData;
use beatoraja_types::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;

/// Minimal mock SongDatabaseAccessor for lifecycle testing.
struct MockSongDb;

impl SongDatabaseAccessorTrait for MockSongDb {
    fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
        Vec::new()
    }
    fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
        Vec::new()
    }
    fn get_song_datas_by_sql(
        &self,
        _sql: &str,
        _score: &str,
        _scorelog: &str,
        _info: Option<&str>,
    ) -> Vec<SongData> {
        Vec::new()
    }
    fn set_song_datas(&self, _songs: &[SongData]) {}
    fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
        Vec::new()
    }
    fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
        Vec::new()
    }
}

// Global lock to serialize tests that touch shared statics.
// MainLoader uses process-global OnceLock statics, so tests touching them
// must not run concurrently.
static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// SONGDB uses OnceLock<Mutex<Option<...>>> — the inner Option CAN be replaced,
/// but the Mutex wrapper lives for the process lifetime. This test documents
/// that set → clear → set works, unlike a bare OnceLock which rejects set() twice.
#[test]
fn lifecycle_songdb_mutex_allows_replacement_via_clear_and_set() {
    let _lock = TEST_LOCK.lock().unwrap();

    // Ensure clean state
    MainLoader::clear_score_database_accessor();

    // First set
    MainLoader::set_score_database_accessor(Box::new(MockSongDb));

    // Clear and re-set (this works because the Mutex inside OnceLock allows it)
    MainLoader::clear_score_database_accessor();
    MainLoader::set_score_database_accessor(Box::new(MockSongDb));

    // Clean up
    MainLoader::clear_score_database_accessor();
}

/// ILLEGAL_SONGS uses OnceLock<Mutex<HashSet<String>>> — once the Mutex is
/// created, illegal songs can be added and cleared freely. This documents
/// that the clear method works correctly for test isolation.
#[test]
fn lifecycle_illegal_songs_can_be_cleared_and_repopulated() {
    let _lock = TEST_LOCK.lock().unwrap();

    // Ensure clean state
    MainLoader::clear_illegal_songs();
    assert_eq!(MainLoader::get_illegal_song_count(), 0);

    // Add some illegal songs
    MainLoader::put_illegal_song("hash_a");
    MainLoader::put_illegal_song("hash_b");
    assert_eq!(MainLoader::get_illegal_song_count(), 2);

    // Clear and verify
    MainLoader::clear_illegal_songs();
    assert_eq!(MainLoader::get_illegal_song_count(), 0);

    // Re-add after clear
    MainLoader::put_illegal_song("hash_c");
    assert_eq!(MainLoader::get_illegal_song_count(), 1);

    // Clean up
    MainLoader::clear_illegal_songs();
}
