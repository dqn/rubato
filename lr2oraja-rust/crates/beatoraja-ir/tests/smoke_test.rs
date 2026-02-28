//! Smoke tests for beatoraja-ir crate: zero-coverage API surface.
//!
//! These tests verify that core data types can be constructed, accessed,
//! and serialized without panicking. No network calls are made.

use beatoraja_ir::ir_response::IRResponse;
use beatoraja_ir::lr2_ir_connection::LR2IRSongData;
use beatoraja_ir::ranking_data::{self, RankingData};

// ---------------------------------------------------------------------------
// RankingData
// ---------------------------------------------------------------------------

#[test]
fn ranking_data_default_construction() {
    let rd = RankingData::default();

    assert_eq!(rd.get_rank(), 0);
    assert_eq!(rd.get_previous_rank(), 0);
    assert_eq!(rd.get_local_rank(), 0);
    assert_eq!(rd.get_total_player(), 0);
    assert_eq!(rd.get_state(), ranking_data::NONE);
    assert_eq!(rd.get_last_update_time(), 0);
    assert!(rd.get_score(0).is_none());
    assert_eq!(rd.get_score_ranking(0), i32::MIN);
    assert_eq!(rd.get_clear_count(0), 0);
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
    // Japanese text is NOT url-encoded by to_url_encoded_form() because it
    // uses plain string formatting without percent-encoding. This test
    // documents that behavior.
    let song = LR2IRSongData::new(
        "\u{6771}\u{65b9}\u{30d7}\u{30ed}\u{30b8}\u{30a7}\u{30af}\u{30c8}".to_string(), // "東方プロジェクト"
        "114328".to_string(),
    );
    let form = song.to_url_encoded_form();

    // BUG: Unicode characters are passed through raw instead of being
    // percent-encoded. Servers expecting application/x-www-form-urlencoded
    // may reject or misparse this.
    assert!(
        form.contains("\u{6771}\u{65b9}"),
        "Expected raw Unicode in form body (no percent-encoding), got: {}",
        form
    );
    assert!(
        !form.contains("%E6"),
        "Unicode should NOT be percent-encoded (documenting existing behavior): {}",
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
    assert_eq!(resp.get_data(), Some(&vec![1, 2, 3]));
}

#[test]
fn ir_response_failure_has_no_data() {
    let resp: IRResponse<String> = IRResponse::failure("network error".to_string());
    assert!(!resp.is_succeeded());
    assert!(resp.get_data().is_none());
    assert_eq!(resp.get_message(), "network error");
}
