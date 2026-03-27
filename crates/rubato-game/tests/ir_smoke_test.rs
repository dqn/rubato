//! Smoke tests for beatoraja-ir crate: zero-coverage API surface.
//!
//! These tests verify that core data types can be constructed, accessed,
//! and serialized without panicking. No network calls are made.

use rubato_game::ir::ir_response::IRResponse;
use rubato_game::ir::lr2_ir_connection::LR2IRSongData;
use rubato_game::ir::ranking_data::{self, RankingData};

// ---------------------------------------------------------------------------
// RankingData
// ---------------------------------------------------------------------------

#[test]
fn ranking_data_default_construction() {
    let rd = RankingData::default();

    assert_eq!(rd.rank(), 0);
    assert_eq!(rd.previous_rank(), 0);
    assert_eq!(rd.local_rank(), 0);
    assert_eq!(rd.total_player(), 0);
    assert_eq!(rd.state(), ranking_data::NONE);
    assert_eq!(rd.last_update_time(), 0);
    assert!(rd.score(0).is_none());
    assert_eq!(rd.score_ranking(0), i32::MIN);
    assert_eq!(rd.clear_count(0), 0);
}

// ---------------------------------------------------------------------------
// LR2IRSongData::to_url_encoded_form
// ---------------------------------------------------------------------------

#[test]
fn lr2_ir_song_data_url_encode_empty_fields() {
    let song = LR2IRSongData::new(String::new(), String::new());
    let form = song.to_url_encoded_form();

    // Should produce a valid form string even with empty fields.
    assert_eq!(form, "songmd5=&id=&lastupdate=");
}

#[test]
fn lr2_ir_song_data_url_encode_unicode() {
    // Japanese text is now properly percent-encoded by to_url_encoded_form().
    let song = LR2IRSongData::new(
        "\u{6771}\u{65b9}\u{30d7}\u{30ed}\u{30b8}\u{30a7}\u{30af}\u{30c8}".to_string(), // "東方プロジェクト"
        "114328".to_string(),
    );
    let form = song.to_url_encoded_form();

    // Unicode characters should be percent-encoded for safe transmission.
    assert!(
        form.contains("%E6"),
        "Unicode should be percent-encoded: {}",
        form
    );
    assert!(
        !form.contains("\u{6771}"),
        "Raw Unicode should not be present in encoded form: {}",
        form
    );
}

// ---------------------------------------------------------------------------
// IRResponse
// ---------------------------------------------------------------------------

#[test]
fn ir_response_success_with_vec_data() {
    let resp = IRResponse::success("OK".to_string(), vec![1, 2, 3]);
    assert!(resp.is_succeeded());
    assert_eq!(resp.data(), Some(&vec![1, 2, 3]));
}

#[test]
fn ir_response_failure_has_no_data() {
    let resp: IRResponse<String> = IRResponse::failure("network error".to_string());
    assert!(!resp.is_succeeded());
    assert!(resp.data().is_none());
    assert_eq!(resp.message, "network error");
}
