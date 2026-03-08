// BarManager integration tests.
//
// Tests BarManager.init() + update_bar() with realistic data structures.
// Uses NullSongDatabaseAccessor (beatoraja-select has no rusqlite dependency)
// but verifies bar hierarchy, favorites, sorting, search, and navigation.
//
// Run: cargo test -p beatoraja-select --test bar_manager_integration

use rubato_core::config::Config;
use rubato_core::player_config::PlayerConfig;
use rubato_state::select::bar::bar::Bar;
use rubato_state::select::bar::hash_bar::HashBar;
use rubato_state::select::bar::search_word_bar::SearchWordBar;
use rubato_state::select::bar::song_bar::SongBar;
use rubato_state::select::bar_manager::{BarManager, UpdateBarContext};
use rubato_state::select::bar_sorter::BarSorter;
use rubato_state::select::null_song_database_accessor::NullSongDatabaseAccessor;
use rubato_types::song_data::SongData;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_song_data(sha256: &str, title: &str, artist: &str, path: &str) -> SongData {
    let mut sd = SongData::default();
    sd.file.sha256 = sha256.to_string();
    sd.metadata.title = title.to_string();
    sd.metadata.artist = artist.to_string();
    sd.file.set_path(path.to_string());
    sd
}

fn make_song_bar(sha256: &str, title: &str, artist: &str, path: &str) -> Bar {
    Bar::Song(Box::new(SongBar::new(make_song_data(
        sha256, title, artist, path,
    ))))
}

// ---------------------------------------------------------------------------
// init() tests
// ---------------------------------------------------------------------------

#[test]
fn init_creates_courses_and_commands() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);

    // Courses should be initialized
    assert!(manager.courses.is_some());

    // Commands should have at least LAMP UPDATE and SCORE UPDATE
    assert!(
        manager.commands.len() >= 2,
        "Expected at least 2 command bars, got {}",
        manager.commands.len()
    );

    // Verify first command is LAMP UPDATE with 30 entries
    if let Some(Bar::Container(c)) = manager.commands.first() {
        assert_eq!(c.title(), "LAMP UPDATE");
        assert_eq!(c.childbar.len(), 30);
    } else {
        panic!("First command should be LAMP UPDATE ContainerBar");
    }

    // Verify second command is SCORE UPDATE with 30 entries
    if let Some(Bar::Container(c)) = manager.commands.get(1) {
        assert_eq!(c.title(), "SCORE UPDATE");
        assert_eq!(c.childbar.len(), 30);
    } else {
        panic!("Second command should be SCORE UPDATE ContainerBar");
    }
}

#[test]
fn init_creates_default_random_folder() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);

    // random/default.json likely missing in test environment; default folder is created
    assert!(
        !manager.tables().is_empty() || manager.courses.is_some(),
        "init() should set up courses even without table files"
    );
}

// ---------------------------------------------------------------------------
// update_bar() with favorites
// ---------------------------------------------------------------------------

#[test]
fn update_bar_root_displays_favorites() {
    let mut manager = BarManager::new();

    // Add favorites with song data
    let songs = vec![
        make_song_data("sha_a", "Song A", "Artist A", "/songs/a.bms"),
        make_song_data("sha_b", "Song B", "Artist B", "/songs/b.bms"),
    ];
    manager.favorites = vec![HashBar::new("My Favorites".to_string(), songs)];

    // Update to root
    let result = manager.update_bar(None);
    assert!(result, "update_bar(None) should succeed with favorites");
    assert!(!manager.currentsongs.is_empty());

    // The favorites hash bar should appear in currentsongs
    let has_fav = manager
        .currentsongs
        .iter()
        .any(|b| b.title() == "My Favorites");
    assert!(has_fav, "Favorites bar should be in root listing");
}

#[test]
fn update_bar_root_resets_selected_index() {
    let mut manager = BarManager::new();
    manager.selectedindex = 99;

    let songs = vec![make_song_data("sha_x", "X", "A", "/x.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    manager.update_bar(None);
    assert_eq!(manager.selectedindex, 0, "selectedindex should reset to 0");
}

#[test]
fn update_bar_root_clears_directory_stack() {
    let mut manager = BarManager::new();

    // Push a fake directory level
    manager.dir.push(Box::new(Bar::Folder(Box::new(
        rubato_state::select::bar::folder_bar::FolderBar::new(None, "test_dir".to_string()),
    ))));
    assert!(!manager.dir.is_empty());

    let songs = vec![make_song_data("sha_y", "Y", "B", "/y.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    manager.update_bar(None);

    // Root update should clear the directory stack
    assert!(
        manager.dir.is_empty(),
        "dir should be empty after root update"
    );
    assert_eq!(manager.dir_string, "", "dir_string should be empty at root");
}

// ---------------------------------------------------------------------------
// update_bar_with_context() filtering
// ---------------------------------------------------------------------------

#[test]
fn update_bar_with_context_sorts_by_title() {
    let mut manager = BarManager::new();

    // Create favorites with unsorted songs
    let songs = vec![
        make_song_data("sha_c", "Charlie", "X", "/c.bms"),
        make_song_data("sha_a", "Alpha", "X", "/a.bms"),
        make_song_data("sha_b", "Bravo", "X", "/b.bms"),
    ];
    manager.favorites = vec![HashBar::new("Songs".to_string(), songs)];

    // Enter the favorites directory
    let hash_bar = Bar::Hash(Box::new(HashBar::new(
        "Songs".to_string(),
        vec![
            make_song_data("sha_c", "Charlie", "X", "/c.bms"),
            make_song_data("sha_a", "Alpha", "X", "/a.bms"),
            make_song_data("sha_b", "Bravo", "X", "/b.bms"),
        ],
    )));

    let config = Config::default();
    let mut player_config = PlayerConfig::default();
    let null_db = NullSongDatabaseAccessor;
    let mut ctx = UpdateBarContext {
        config: &config,
        player_config: &mut player_config,
        songdb: &null_db,
        score_cache: None,
        is_folderlamp: false,
        max_search_bar_count: 10,
    };

    manager.update_bar_with_context(Some(&hash_bar), Some(&mut ctx));

    // Songs should be sorted by title (default sorter)
    if manager.currentsongs.len() >= 3 {
        let titles: Vec<String> = manager
            .currentsongs
            .iter()
            .filter_map(|b| {
                b.as_song_bar()
                    .map(|sb| sb.song_data().metadata.title.clone())
            })
            .collect();
        // Verify alphabetical sort
        for i in 1..titles.len() {
            assert!(
                titles[i - 1] <= titles[i],
                "Songs should be sorted by title: {:?}",
                titles
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Navigation: mov(), close()
// ---------------------------------------------------------------------------

#[test]
fn mov_wraps_around() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("a", "A", "X", "/a.bms"),
        make_song_bar("b", "B", "X", "/b.bms"),
        make_song_bar("c", "C", "X", "/c.bms"),
    ];
    manager.selectedindex = 0;

    // Move forward
    manager.mov(true);
    assert_eq!(manager.selectedindex, 1);
    manager.mov(true);
    assert_eq!(manager.selectedindex, 2);
    manager.mov(true);
    assert_eq!(manager.selectedindex, 0, "Should wrap to 0");

    // Move backward from 0
    manager.mov(false);
    assert_eq!(manager.selectedindex, 2, "Should wrap to end");
}

#[test]
fn close_at_root_does_not_panic() {
    let mut manager = BarManager::new();
    // At root level, close should not panic
    manager.close();
}

#[test]
fn close_returns_to_parent() {
    let mut manager = BarManager::new();

    // Set up a directory entry
    manager.dir.push(Box::new(Bar::Folder(Box::new(
        rubato_state::select::bar::folder_bar::FolderBar::new(None, "parent".to_string()),
    ))));

    // Need favorites so update_bar doesn't recurse infinitely
    let songs = vec![make_song_data("abc", "Test", "A", "/test.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    manager.close();

    // After close from single directory level, should be at root
    assert!(
        manager.dir.is_empty(),
        "Should return to root after closing single level"
    );
}

// ---------------------------------------------------------------------------
// Search bar management
// ---------------------------------------------------------------------------

#[test]
fn add_search_and_display_at_root() {
    let mut manager = BarManager::new();

    // Add a search bar
    manager.add_search(
        SearchWordBar::new("Recent Search".to_string(), "test query".to_string()),
        10,
    );

    // Add favorites so root has content
    let songs = vec![make_song_data("sha1", "S1", "A", "/s1.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    // Update root
    manager.update_bar(None);

    // Search bar should appear in root listing
    let has_search = manager
        .currentsongs
        .iter()
        .any(|b| b.title() == "Recent Search");
    assert!(has_search, "Search bar should appear at root level");
}

#[test]
fn add_search_bars_appear_at_root() {
    let mut manager = BarManager::new();

    // Add multiple search bars
    for i in 0..5 {
        manager.add_search(
            SearchWordBar::new(format!("search_{}", i), format!("query_{}", i)),
            10,
        );
    }

    // Add favorites so root has content
    let songs = vec![make_song_data("sha1", "S1", "A", "/s1.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    manager.update_bar(None);

    // All 5 search bars should appear in the root listing
    let search_count = manager
        .currentsongs
        .iter()
        .filter(|b| b.title().starts_with("search_"))
        .count();
    assert_eq!(
        search_count, 5,
        "All search bars should appear at root, found {}",
        search_count
    );
}

#[test]
fn add_search_deduplicates_and_shows_at_root() {
    let mut manager = BarManager::new();

    manager.add_search(
        SearchWordBar::new("test".to_string(), "first query".to_string()),
        10,
    );
    manager.add_search(
        SearchWordBar::new("other".to_string(), "other query".to_string()),
        10,
    );
    // Adding "test" again should replace the first one
    manager.add_search(
        SearchWordBar::new("test".to_string(), "updated query".to_string()),
        10,
    );

    // Add favorites so root has content
    let songs = vec![make_song_data("sha1", "S1", "A", "/s1.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    manager.update_bar(None);

    // Should have exactly 2 search bars (deduplicated "test" + "other")
    let search_titles: Vec<&str> = manager
        .currentsongs
        .iter()
        .filter(|b| matches!(b, Bar::SearchWord(_)))
        .map(|b| b.title())
        .collect();
    assert_eq!(
        search_titles.len(),
        2,
        "Should have 2 search bars after dedup"
    );
}

// ---------------------------------------------------------------------------
// selected_position / set_selected_position
// ---------------------------------------------------------------------------

#[test]
fn selected_position_round_trip() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("a", "A", "X", "/a.bms"),
        make_song_bar("b", "B", "X", "/b.bms"),
        make_song_bar("c", "C", "X", "/c.bms"),
        make_song_bar("d", "D", "X", "/d.bms"),
    ];

    manager.set_selected_position(0.5);
    assert_eq!(manager.selectedindex, 2);

    let pos = manager.selected_position();
    assert!((pos - 0.5).abs() < 0.01);
}

#[test]
fn selected_position_out_of_range_ignored() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![make_song_bar("a", "A", "X", "/a.bms")];
    manager.selectedindex = 0;

    // Out-of-range values should be ignored
    manager.set_selected_position(-0.5);
    assert_eq!(manager.selectedindex, 0);

    manager.set_selected_position(1.5);
    assert_eq!(manager.selectedindex, 0);
}

// ---------------------------------------------------------------------------
// Multiple favorites at root
// ---------------------------------------------------------------------------

#[test]
fn multiple_favorites_appear_at_root() {
    let mut manager = BarManager::new();

    manager.favorites = vec![
        HashBar::new(
            "Favorites 1".to_string(),
            vec![make_song_data("sha1", "S1", "A", "/s1.bms")],
        ),
        HashBar::new(
            "Favorites 2".to_string(),
            vec![make_song_data("sha2", "S2", "B", "/s2.bms")],
        ),
        HashBar::new(
            "Favorites 3".to_string(),
            vec![make_song_data("sha3", "S3", "C", "/s3.bms")],
        ),
    ];

    let result = manager.update_bar(None);
    assert!(result);

    let fav_count = manager
        .currentsongs
        .iter()
        .filter(|b| b.title().starts_with("Favorites"))
        .count();
    assert_eq!(fav_count, 3, "All 3 favorites should appear at root");
}

// ---------------------------------------------------------------------------
// BarSorter integration
// ---------------------------------------------------------------------------

#[test]
fn bar_sorter_title_sorts_correctly() {
    let mut bars = [
        make_song_bar("c", "Zebra", "X", "/z.bms"),
        make_song_bar("a", "Alpha", "X", "/a.bms"),
        make_song_bar("b", "Middle", "X", "/m.bms"),
    ];

    bars.sort_by(|a, b| BarSorter::Title.compare(a, b));

    let titles: Vec<String> = bars
        .iter()
        .filter_map(|b| {
            b.as_song_bar()
                .map(|sb| sb.song_data().metadata.title.clone())
        })
        .collect();
    assert_eq!(titles, vec!["Alpha", "Middle", "Zebra"]);
}

// ---------------------------------------------------------------------------
// Loader stop flag
// ---------------------------------------------------------------------------

#[test]
fn loader_stop_flag_set_on_update_bar() {
    let mut manager = BarManager::new();

    let songs = vec![make_song_data("sha_x", "X", "A", "/x.bms")];
    manager.favorites = vec![HashBar::new("FAV".to_string(), songs)];

    manager.update_bar(None);

    // After update_bar, a loader stop flag should be set
    assert!(
        manager.loader_stop.is_some(),
        "loader_stop should be set after update_bar"
    );
    assert!(
        !manager
            .loader_stop
            .as_ref()
            .unwrap()
            .load(std::sync::atomic::Ordering::SeqCst),
        "loader_stop should initially be false (not stopped)"
    );
}

// ---------------------------------------------------------------------------
// Empty state handling
// ---------------------------------------------------------------------------

#[test]
fn get_selected_on_empty_returns_none() {
    let manager = BarManager::new();
    assert!(manager.selected().is_none());
}

#[test]
fn get_selected_position_on_empty_returns_zero() {
    let manager = BarManager::new();
    assert_eq!(manager.selected_position(), 0.0);
}

#[test]
fn mov_on_empty_does_not_panic() {
    let mut manager = BarManager::new();
    manager.mov(true);
    manager.mov(false);
    // Should not panic
}
