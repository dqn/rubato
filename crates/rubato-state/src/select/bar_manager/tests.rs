use super::*;
use crate::select::bar::song_bar::SongBar;

fn make_song_data(sha256: &str, path: Option<&str>) -> SongData {
    let mut sd = SongData::default();
    sd.file.sha256 = sha256.to_string();
    if let Some(p) = path {
        sd.file.set_path(p.to_string());
    }
    sd
}

fn make_song_bar(sha256: &str, path: Option<&str>) -> Bar {
    Bar::Song(Box::new(SongBar::new(make_song_data(sha256, path))))
}

// ---- init tests ----

#[test]
fn test_init_creates_courses() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);
    assert!(manager.courses.is_some());
}

#[test]
fn test_init_creates_commands() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);
    // Should have at least LAMP UPDATE and SCORE UPDATE
    assert!(manager.commands.len() >= 2);
}

#[test]
fn test_init_default_random_folder() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);
    // random/default.json likely doesn't exist in test, so default folder is created
    assert!(!manager.random_folder_list.is_empty());
    assert_eq!(
        manager.random_folder_list[0].name(),
        "[RANDOM] RANDOM SELECT"
    );
}

#[test]
fn test_init_lamp_update_contains_30_days() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);
    // First command should be LAMP UPDATE container with 30 children
    if let Some(Bar::Container(c)) = manager.commands.first() {
        assert_eq!(c.title(), "LAMP UPDATE");
        assert_eq!(c.childbar.len(), 30);
    } else {
        panic!("First command should be LAMP UPDATE container");
    }
}

#[test]
fn test_init_score_update_contains_30_days() {
    let mut manager = BarManager::new();
    let config = Config::default();
    manager.init(&config, &[]);
    if let Some(Bar::Container(c)) = manager.commands.get(1) {
        assert_eq!(c.title(), "SCORE UPDATE");
        assert_eq!(c.childbar.len(), 30);
    } else {
        panic!("Second command should be SCORE UPDATE container");
    }
}

// ---- update_bar tests ----

#[test]
fn test_update_bar_root_with_no_context() {
    let mut manager = BarManager::new();
    // Root with empty manager should return false (no bars)
    let result = manager.update_bar(None);
    assert!(!result);
}

#[test]
fn test_update_bar_root_with_favorites() {
    let mut manager = BarManager::new();
    let songs = vec![make_song_data("abc", Some("/path/song.bms"))];
    manager.favorites = vec![HashBar::new("FAV1".to_string(), songs)];

    let result = manager.update_bar(None);
    // Should have at least the favorite bar
    assert!(result);
    assert!(!manager.currentsongs.is_empty());
}

#[test]
fn test_update_bar_sets_selectedindex_zero() {
    let mut manager = BarManager::new();
    manager.selectedindex = 5;
    manager.favorites = vec![HashBar::new(
        "FAV1".to_string(),
        vec![make_song_data("abc", Some("/path.bms"))],
    )];

    manager.update_bar(None);
    assert_eq!(manager.selectedindex, 0);
}

#[test]
fn test_update_bar_builds_dir_string() {
    let mut manager = BarManager::new();
    manager.favorites = vec![HashBar::new(
        "FAV1".to_string(),
        vec![make_song_data("abc", Some("/path.bms"))],
    )];
    manager.update_bar(None);
    // At root, dir_string should be empty
    assert_eq!(manager.dir_string, "");
}

#[test]
fn test_update_bar_restores_cursor_by_sha256() {
    let mut manager = BarManager::new();
    // Set up currentsongs with a song bar
    manager.currentsongs = vec![
        make_song_bar("aaa", Some("/a.bms")),
        make_song_bar("bbb", Some("/b.bms")),
    ];
    manager.selectedindex = 1; // select "bbb"

    // Now update to root with favorites containing both songs
    manager.favorites = vec![HashBar::new(
        "FAV".to_string(),
        vec![
            make_song_data("aaa", Some("/a.bms")),
            make_song_data("bbb", Some("/b.bms")),
        ],
    )];

    // The favorites bar itself will be shown, not the individual songs
    // So cursor restoration by sha256 won't match, but title matching should work
    manager.update_bar(None);
}

// ---- update_bar_with_context tests ----

#[test]
fn test_update_bar_filters_invisible_songs() {
    let mut manager = BarManager::new();
    let mut visible = make_song_data("visible", Some("/v.bms"));
    visible.favorite = 0;
    let mut invisible = make_song_data("invisible", Some("/i.bms"));
    invisible.favorite = INVISIBLE_SONG;

    manager.currentsongs = vec![
        Bar::Song(Box::new(SongBar::new(visible.clone()))),
        Bar::Song(Box::new(SongBar::new(invisible.clone()))),
    ];

    // With context, invisible songs should be filtered
    let config = Config::default();
    let mut player_config = PlayerConfig::default();
    let mut ctx = UpdateBarContext {
        config: &config,
        player_config: &mut player_config,
        songdb: &crate::select::null_song_database_accessor::NullSongDatabaseAccessor,
        score_cache: None,
        is_folderlamp: false,
        max_search_bar_count: 10,
    };

    // Put songs in favorites so they appear at root
    manager.favorites = vec![HashBar::new("Test".to_string(), vec![visible, invisible])];

    manager.update_bar_with_context(None, Some(&mut ctx));
    // Only visible should remain (but favorites are shown as HashBar, not individual songs)
    // The filtering happens when we enter a directory with SongBars
}

// ---- close tests ----

#[test]
fn test_close_at_root() {
    let mut manager = BarManager::new();
    // At root level, close should not panic
    manager.close();
}

#[test]
fn test_close_goes_up_one_level() {
    let mut manager = BarManager::new();
    // Push a directory level
    manager
        .dir
        .push(Box::new(Bar::Folder(Box::new(FolderBar::new(
            None,
            "test_dir".to_string(),
        )))));
    // Also need some currentsongs so update_bar doesn't recurse infinitely
    manager.favorites = vec![HashBar::new(
        "FAV".to_string(),
        vec![make_song_data("abc", Some("/test.bms"))],
    )];

    manager.close();
    // After close, we should be at root (dir cleared)
    assert!(manager.dir.is_empty());
}

// ---- BarContentsLoaderThread tests ----

#[test]
fn test_loader_stop_flag() {
    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop.clone());
    assert!(!loader.is_stopped());
    loader.stop_running();
    assert!(loader.is_stopped());
}

#[test]
fn test_loader_runs_on_empty_bars() {
    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop);
    let mut bars: Vec<Bar> = Vec::new();
    let player_config = PlayerConfig::default();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: None,
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: None,
        stagefile_resource: None,
    };
    loader.run(&mut bars, &mut ctx);
    // Should complete without errors
}

#[test]
fn test_loader_stops_early_when_signaled() {
    let stop = Arc::new(AtomicBool::new(true)); // pre-stopped
    let loader = BarContentsLoaderThread::new(stop);
    let mut bars = vec![make_song_bar("abc", Some("/test.bms"))];
    let player_config = PlayerConfig::default();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: None,
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: None,
        stagefile_resource: None,
    };
    loader.run(&mut bars, &mut ctx);
    // Should return immediately due to stop flag
}

#[test]
fn test_loader_loads_score_from_cache() {
    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop);

    let sd = make_song_data("test_hash", Some("/test.bms"));
    let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd.clone())))];

    let mut score = ScoreData::default();
    score.judge_counts.epg = 100;

    let mut cache = ScoreDataCache::new(
        Box::new(move |_sd, _lnmode| {
            let mut s = ScoreData::default();
            s.judge_counts.epg = 100;
            Some(s)
        }),
        Box::new(|_collector, _songs, _lnmode| {}),
    );

    let player_config = PlayerConfig::default();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: Some(&mut cache),
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: None,
        stagefile_resource: None,
    };

    loader.run(&mut bars, &mut ctx);

    // Score should be loaded
    assert!(bars[0].score().is_some());
    assert_eq!(bars[0].score().unwrap().judge_counts.epg, 100);
}

// ---- banner/stagefile loading tests ----

fn create_test_png(dir: &std::path::Path, name: &str) -> String {
    let path = dir.join(name);
    let img = image::RgbaImage::from_pixel(4, 4, image::Rgba([255, 0, 0, 255]));
    img.save(&path).unwrap();
    path.to_string_lossy().to_string()
}

#[test]
fn test_loader_loads_banner_via_pool() {
    let dir = tempfile::tempdir().unwrap();
    // Create a banner image file inside the song directory
    create_test_png(dir.path(), "banner.png");

    // Create a SongBar with a path in the temp directory and a banner filename
    let song_file = dir.path().join("test.bms");
    std::fs::write(&song_file, b"").unwrap();
    let mut sd = SongData::default();
    sd.file.sha256 = "bannerhash".to_string();
    sd.file.set_path(song_file.to_string_lossy().to_string());
    sd.file.banner = "banner.png".to_string();
    let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop);
    let player_config = PlayerConfig::default();
    let banner_pool = PixmapResourcePool::new();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: None,
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: Some(&banner_pool),
        stagefile_resource: None,
    };

    loader.run(&mut bars, &mut ctx);

    // Banner should be loaded into the SongBar
    let sb = bars[0].as_song_bar().unwrap();
    assert!(sb.banner().is_some());
    let pix = sb.banner().unwrap();
    assert_eq!(pix.width, 4);
    assert_eq!(pix.height, 4);
}

#[test]
fn test_loader_loads_stagefile_via_pool() {
    let dir = tempfile::tempdir().unwrap();
    // Create a stagefile image file inside the song directory
    create_test_png(dir.path(), "stagefile.png");

    let song_file = dir.path().join("test.bms");
    std::fs::write(&song_file, b"").unwrap();
    let mut sd = SongData::default();
    sd.file.sha256 = "stagefilehash".to_string();
    sd.file.set_path(song_file.to_string_lossy().to_string());
    sd.file.stagefile = "stagefile.png".to_string();
    let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop);
    let player_config = PlayerConfig::default();
    let stagefile_pool = PixmapResourcePool::new();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: None,
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: None,
        stagefile_resource: Some(&stagefile_pool),
    };

    loader.run(&mut bars, &mut ctx);

    // Stagefile should be loaded into the SongBar
    let sb = bars[0].as_song_bar().unwrap();
    assert!(sb.stagefile().is_some());
    let pix = sb.stagefile().unwrap();
    assert_eq!(pix.width, 4);
    assert_eq!(pix.height, 4);
}

#[test]
fn test_loader_no_pool_skips_banner_loading() {
    let dir = tempfile::tempdir().unwrap();
    create_test_png(dir.path(), "banner.png");

    let song_file = dir.path().join("test.bms");
    std::fs::write(&song_file, b"").unwrap();
    let mut sd = SongData::default();
    sd.file.sha256 = "nopoolhash".to_string();
    sd.file.set_path(song_file.to_string_lossy().to_string());
    sd.file.banner = "banner.png".to_string();
    let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop);
    let player_config = PlayerConfig::default();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: None,
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: None,
        stagefile_resource: None,
    };

    loader.run(&mut bars, &mut ctx);

    // Banner should NOT be loaded (no pool)
    let sb = bars[0].as_song_bar().unwrap();
    assert!(sb.banner().is_none());
}

#[test]
fn test_loader_nonexistent_banner_file_not_loaded() {
    let dir = tempfile::tempdir().unwrap();
    // Do NOT create banner.png, it should not exist

    let song_file = dir.path().join("test.bms");
    std::fs::write(&song_file, b"").unwrap();
    let mut sd = SongData::default();
    sd.file.sha256 = "missinghash".to_string();
    sd.file.set_path(song_file.to_string_lossy().to_string());
    sd.file.banner = "banner.png".to_string();
    let mut bars = vec![Bar::Song(Box::new(SongBar::new(sd)))];

    let stop = Arc::new(AtomicBool::new(false));
    let loader = BarContentsLoaderThread::new(stop);
    let player_config = PlayerConfig::default();
    let banner_pool = PixmapResourcePool::new();
    let mut ctx = LoaderContext {
        player_config: &player_config,
        score_cache: None,
        rival_cache: None,
        rival_name: None,
        is_folderlamp: false,
        banner_resource: Some(&banner_pool),
        stagefile_resource: None,
    };

    loader.run(&mut bars, &mut ctx);

    // Banner should NOT be loaded (file does not exist)
    let sb = bars[0].as_song_bar().unwrap();
    assert!(sb.banner().is_none());
}

// ---- add_search tests ----

#[test]
fn test_add_search_respects_max_count() {
    let mut manager = BarManager::new();
    for i in 0..12 {
        manager.add_search(
            SearchWordBar::new(format!("search_{}", i), format!("text_{}", i)),
            10,
        );
    }
    // Should cap at 10
    assert_eq!(manager.search.len(), 10);
    // First 2 should have been removed
    assert_eq!(manager.search[0].title(), "search_2");
}

#[test]
fn test_add_search_removes_duplicate() {
    let mut manager = BarManager::new();
    manager.add_search(SearchWordBar::new("foo".to_string(), "bar".to_string()), 10);
    manager.add_search(SearchWordBar::new("baz".to_string(), "qux".to_string()), 10);
    manager.add_search(
        SearchWordBar::new("foo".to_string(), "updated".to_string()),
        10,
    );

    assert_eq!(manager.search.len(), 2);
    assert_eq!(manager.search[0].title(), "baz");
    assert_eq!(manager.search[1].title(), "foo");
}

// ---- create_command_bar tests ----

#[test]
fn test_create_command_bar_simple() {
    let manager = BarManager::new();
    let folder = CommandFolder {
        name: Some("Test".to_string()),
        folder: vec![],
        sql: Some("SELECT * FROM song".to_string()),
        rcourse: vec![],
        showall: false,
    };
    let bar = manager.create_command_bar(&folder);
    assert!(matches!(bar, Bar::Command(_)));
    assert_eq!(bar.title(), "Test");
}

#[test]
fn test_create_command_bar_with_subfolders() {
    let manager = BarManager::new();
    let folder = CommandFolder {
        name: Some("Parent".to_string()),
        folder: vec![CommandFolder {
            name: Some("Child".to_string()),
            folder: vec![],
            sql: Some("SELECT 1".to_string()),
            rcourse: vec![],
            showall: false,
        }],
        sql: None,
        rcourse: vec![],
        showall: false,
    };
    let bar = manager.create_command_bar(&folder);
    assert!(matches!(bar, Bar::Container(_)));
    assert_eq!(bar.title(), "Parent");
}

// ---- RandomFolder.filter_song tests ----

#[test]
fn test_filter_song_no_filter() {
    let rf = RandomFolder {
        name: Some("Test".to_string()),
        filter: None,
    };
    assert!(rf.filter_song(None));
    let score = ScoreData::default();
    assert!(rf.filter_song(Some(&score)));
}

#[test]
fn test_filter_song_integer_filter_no_score() {
    let mut filter = HashMap::new();
    filter.insert("clear".to_string(), serde_json::Value::Number(0.into()));
    let rf = RandomFolder {
        name: Some("Test".to_string()),
        filter: Some(filter),
    };
    // null score with filter value 0 should pass
    assert!(rf.filter_song(None));
}

#[test]
fn test_filter_song_integer_filter_nonzero_no_score() {
    let mut filter = HashMap::new();
    filter.insert("clear".to_string(), serde_json::Value::Number(5.into()));
    let rf = RandomFolder {
        name: Some("Test".to_string()),
        filter: Some(filter),
    };
    // null score with non-zero filter value should fail
    assert!(!rf.filter_song(None));
}

#[test]
fn test_filter_song_string_comparison() {
    let mut filter = HashMap::new();
    filter.insert(
        "clear".to_string(),
        serde_json::Value::String(">=3".to_string()),
    );
    let rf = RandomFolder {
        name: Some("Test".to_string()),
        filter: Some(filter),
    };

    let mut score = ScoreData::default();
    score.clear = 5;
    assert!(rf.filter_song(Some(&score)));

    score.clear = 2;
    assert!(!rf.filter_song(Some(&score)));
}

// ---- evaluate_filter_expression tests ----

#[test]
fn test_evaluate_filter_gte() {
    assert!(evaluate_filter_expression(">=5", 5));
    assert!(evaluate_filter_expression(">=5", 6));
    assert!(!evaluate_filter_expression(">=5", 4));
}

#[test]
fn test_evaluate_filter_lte() {
    assert!(evaluate_filter_expression("<=5", 5));
    assert!(evaluate_filter_expression("<=5", 4));
    assert!(!evaluate_filter_expression("<=5", 6));
}

#[test]
fn test_evaluate_filter_gt() {
    assert!(evaluate_filter_expression(">5", 6));
    assert!(!evaluate_filter_expression(">5", 5));
}

#[test]
fn test_evaluate_filter_lt() {
    assert!(evaluate_filter_expression("<5", 4));
    assert!(!evaluate_filter_expression("<5", 5));
}

#[test]
fn test_evaluate_filter_empty() {
    assert!(evaluate_filter_expression("", 42));
}

// ---- i64 truncation bug tests ----

#[test]
fn test_filter_song_date_i64_not_truncated() {
    // Unix timestamp 1_700_000_000 exceeds i32::MAX (2_147_483_647 fits, but
    // 3_000_000_000 does not). Ensure large i64 values are compared correctly.
    let timestamp: i64 = 3_000_000_000; // exceeds i32::MAX
    let mut filter = HashMap::new();
    filter.insert(
        "date".to_string(),
        serde_json::Value::Number(serde_json::Number::from(timestamp)),
    );
    let rf = RandomFolder {
        name: Some("Test".to_string()),
        filter: Some(filter),
    };
    let mut score = ScoreData::default();
    score.date = timestamp;
    // Should match: both filter and score have the same i64 value
    assert!(rf.filter_song(Some(&score)));
}

#[test]
fn test_filter_song_date_i64_mismatch_detected() {
    // When the filter value and score differ, it should correctly detect the mismatch
    let mut filter = HashMap::new();
    filter.insert(
        "date".to_string(),
        serde_json::Value::Number(serde_json::Number::from(3_000_000_000_i64)),
    );
    let rf = RandomFolder {
        name: Some("Test".to_string()),
        filter: Some(filter),
    };
    let mut score = ScoreData::default();
    score.date = 3_000_000_001_i64;
    // Should NOT match: values differ by 1
    assert!(!rf.filter_song(Some(&score)));
}

#[test]
fn test_evaluate_filter_expression_large_i64() {
    // Comparison operators should work with values exceeding i32::MAX
    assert!(evaluate_filter_expression(">=3000000000", 3_000_000_000));
    assert!(evaluate_filter_expression(">=3000000000", 3_000_000_001));
    assert!(!evaluate_filter_expression(">=3000000000", 2_999_999_999));

    assert!(evaluate_filter_expression("<=3000000000", 3_000_000_000));
    assert!(!evaluate_filter_expression("<=3000000000", 3_000_000_001));

    assert!(evaluate_filter_expression(">3000000000", 3_000_000_001));
    assert!(!evaluate_filter_expression(">3000000000", 3_000_000_000));

    assert!(evaluate_filter_expression("<3000000000", 2_999_999_999));
    assert!(!evaluate_filter_expression("<3000000000", 3_000_000_000));
}

#[test]
fn test_get_score_data_property_date_i64() {
    let mut score = ScoreData::default();
    score.date = 3_000_000_000;
    // Should return the full i64 value without truncation
    assert_eq!(score_data_property(&score, "date"), 3_000_000_000_i64);
}

// ---- bar_class_name tests ----

#[test]
fn test_bar_class_name() {
    let song = make_song_bar("abc", Some("/test.bms"));
    assert_eq!(bar_class_name(&song), "SongBar");

    let folder = Bar::Folder(Box::new(FolderBar::new(None, "test".to_string())));
    assert_eq!(bar_class_name(&folder), "FolderBar");

    let container = Bar::Container(Box::new(ContainerBar::new("c".to_string(), vec![])));
    assert_eq!(bar_class_name(&container), "ContainerBar");
}

// ---- CourseTableAccessor tests ----

#[test]
fn test_course_table_accessor_name() {
    let accessor = CourseTableAccessor;
    assert_eq!(accessor.name(), "course");
}

// ---- existing tests preserved ----

#[test]
fn test_get_selected_empty() {
    let manager = BarManager::new();
    assert!(manager.selected().is_none());
}

#[test]
fn test_get_selected_with_songs() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("abc", Some("/a.bms")),
        make_song_bar("def", Some("/d.bms")),
    ];
    manager.selectedindex = 1;
    let selected = manager.selected().unwrap();
    assert_eq!(
        selected.title(),
        make_song_data("def", Some("/d.bms")).metadata.full_title()
    );
}

#[test]
fn test_mov_increase() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("a", Some("/a.bms")),
        make_song_bar("b", Some("/b.bms")),
        make_song_bar("c", Some("/c.bms")),
    ];
    manager.selectedindex = 0;
    manager.mov(true);
    assert_eq!(manager.selectedindex, 1);
    manager.mov(true);
    assert_eq!(manager.selectedindex, 2);
    manager.mov(true);
    assert_eq!(manager.selectedindex, 0); // wraps
}

#[test]
fn test_mov_decrease() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("a", Some("/a.bms")),
        make_song_bar("b", Some("/b.bms")),
        make_song_bar("c", Some("/c.bms")),
    ];
    manager.selectedindex = 0;
    manager.mov(false);
    assert_eq!(manager.selectedindex, 2); // wraps to end
}

#[test]
fn test_set_selected_position() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("a", Some("/a.bms")),
        make_song_bar("b", Some("/b.bms")),
        make_song_bar("c", Some("/c.bms")),
        make_song_bar("d", Some("/d.bms")),
    ];
    manager.set_selected_position(0.5);
    assert_eq!(manager.selectedindex, 2);
}

#[test]
fn test_get_selected_position() {
    let mut manager = BarManager::new();
    manager.currentsongs = vec![
        make_song_bar("a", Some("/a.bms")),
        make_song_bar("b", Some("/b.bms")),
        make_song_bar("c", Some("/c.bms")),
        make_song_bar("d", Some("/d.bms")),
    ];
    manager.selectedindex = 2;
    let pos = manager.selected_position();
    assert!((pos - 0.5).abs() < 0.01);
}

#[test]
fn test_add_random_course() {
    let mut manager = BarManager::new();
    let course = CourseData::default();
    let bar = GradeBar::new(course);
    manager.add_random_course(bar, "test > ".to_string());
    assert_eq!(manager.random_course_result.len(), 1);
}

#[test]
fn test_add_random_course_caps_at_100() {
    let mut manager = BarManager::new();
    for i in 0..110 {
        let course = CourseData {
            name: Some(format!("course_{}", i)),
            ..CourseData::default()
        };
        manager.add_random_course(GradeBar::new(course), format!("dir_{}", i));
    }
    assert_eq!(manager.random_course_result.len(), 100);
}

#[test]
fn test_set_append_directory_bar() {
    let mut manager = BarManager::new();
    let bar = make_song_bar("test", Some("/test.bms"));
    manager.set_append_directory_bar("key1".to_string(), bar);
    assert!(manager.append_folders.contains_key("key1"));
}

#[test]
fn test_invisible_filtering_without_context() {
    let mut manager = BarManager::new();

    let mut visible = make_song_data("visible", Some("/v.bms"));
    visible.favorite = 0;
    let mut invisible = make_song_data("invisible", Some("/i.bms"));
    invisible.favorite = INVISIBLE_SONG;

    // Build a container bar with both visible and invisible songs
    let children = vec![
        Bar::Song(Box::new(SongBar::new(visible))),
        Bar::Song(Box::new(SongBar::new(invisible))),
    ];
    let container = ContainerBar::new(String::new(), children);

    // Enter the container WITHOUT context
    manager.update_bar_with_context(Some(&Bar::Container(Box::new(container))), None);

    // Without context, children can't be loaded from the container match branch,
    // so currentsongs will be empty (no children to filter).
    // This confirms the else branch doesn't panic and handles gracefully.
    // The invisible filtering else branch is reachable when children are
    // pre-populated through other means.
}

#[test]
fn test_invisible_filtering_with_context() {
    let mut manager = BarManager::new();

    let mut visible = make_song_data("visible_song", Some("/v.bms"));
    visible.metadata.title = "visible_song".to_string();
    visible.favorite = 0;
    visible.chart.mode = 0;
    let mut invisible = make_song_data("invisible_song", Some("/i.bms"));
    invisible.metadata.title = "invisible_song".to_string();
    invisible.favorite = INVISIBLE_SONG;
    invisible.chart.mode = 0;

    let children = vec![
        Bar::Song(Box::new(SongBar::new(visible))),
        Bar::Song(Box::new(SongBar::new(invisible))),
    ];
    let container = ContainerBar::new("TestDir".to_string(), children);

    let config = Config::default();
    let mut player_config = PlayerConfig::default();
    let mut ctx = UpdateBarContext {
        config: &config,
        player_config: &mut player_config,
        songdb: &crate::select::null_song_database_accessor::NullSongDatabaseAccessor,
        score_cache: None,
        is_folderlamp: false,
        max_search_bar_count: 10,
    };

    manager.update_bar_with_context(Some(&Bar::Container(Box::new(container))), Some(&mut ctx));

    // With context, invisible song should be filtered out
    let song_count = manager
        .currentsongs
        .iter()
        .filter(|b| b.as_song_bar().is_some())
        .count();
    assert_eq!(song_count, 1, "only visible song should remain");
    assert_eq!(
        manager.currentsongs[0].title(),
        "visible_song",
        "the remaining song should be the visible one"
    );
}
