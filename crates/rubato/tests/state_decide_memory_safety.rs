// Phase 60c: Verify MusicDecide can be constructed and dropped without leak.
//
// Previously tested MainControllerRef memory safety. After migration to
// direct config fields, this test validates basic construction/drop safety.

use rubato::core::player_resource::PlayerResource;
use rubato::decide::music_decide::MusicDecide;

/// MusicDecide can be constructed and dropped without leak.
#[test]
fn construction_and_drop_is_safe() {
    let decide = MusicDecide::new(
        rubato_skin::config::Config::default(),
        PlayerResource::new(
            rubato_skin::config::Config::default(),
            rubato_skin::player_config::PlayerConfig::default(),
        ),
        rubato::core::timer_manager::TimerManager::new(),
    );
    drop(decide);
}
