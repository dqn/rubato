use std::sync::Arc;

use anyhow::Result;
use tracing::info;

use super::{ScreenshotExporter, ScreenshotScoreInfo, clear_type_name, rank_name};
use crate::webhook::WebhookHandler;

/// Composes human-readable score text for social sharing (webhook, clipboard, etc.).
///
/// Text format follows the Java `ScreenShotTwitterExporter` pattern:
/// - Song title and artist
/// - Clear type, rank, and EX score (when score info is available)
/// - Hashtag for discoverability
pub struct ScoreTextComposer;

impl ScoreTextComposer {
    /// Compose a score text suitable for social sharing.
    ///
    /// When `score_info` is `None`, only the title/artist line and hashtag are included.
    pub fn compose(title: &str, artist: &str, score_info: Option<&ScreenshotScoreInfo>) -> String {
        let mut lines: Vec<String> = Vec::new();

        // Title / Artist line
        if !title.is_empty() && !artist.is_empty() {
            lines.push(format!("{} / {}", title, artist));
        } else if !title.is_empty() {
            lines.push(title.to_string());
        } else if !artist.is_empty() {
            lines.push(artist.to_string());
        }

        // Score details
        if let Some(info) = score_info {
            let clear = clear_type_name(info.clear_type_id);
            let rank = rank_name(info.exscore, info.max_notes);
            let max_ex = info.max_notes * 2;
            lines.push(format!(
                "Clear: {} | Rank: {} | EX: {}/{}",
                clear, rank, info.exscore, max_ex
            ));
        }

        // Hashtag
        lines.push("#beatoraja".to_string());

        lines.join("\n")
    }
}

/// Screenshot exporter that sends screenshots with score text via webhook.
///
/// Uses `WebhookHandler` to post multipart/form-data (Discord-compatible)
/// with the score text as the message body and the screenshot as an attachment.
pub struct WebhookScreenshotExporter {
    handler: Arc<WebhookHandler>,
    webhook_name: String,
    webhook_avatar: String,
}

impl WebhookScreenshotExporter {
    pub fn new(handler: Arc<WebhookHandler>, webhook_name: String, webhook_avatar: String) -> Self {
        Self {
            handler,
            webhook_name,
            webhook_avatar,
        }
    }
}

impl ScreenshotExporter for WebhookScreenshotExporter {
    fn send(
        &self,
        image_data: &[u8],
        state_name: &str,
        score_info: Option<&ScreenshotScoreInfo>,
    ) -> Result<()> {
        // Build score info for webhook embed; use defaults when no score_info provided
        let info = score_info.cloned().unwrap_or(ScreenshotScoreInfo {
            clear_type_id: 0,
            exscore: 0,
            max_notes: 0,
        });

        let song_title = state_name.to_string();

        let handler = Arc::clone(&self.handler);
        let webhook_name = self.webhook_name.clone();
        let webhook_avatar = self.webhook_avatar.clone();
        let image = image_data.to_vec();

        // Spawn async send on the tokio runtime
        tokio::task::spawn(async move {
            if let Err(e) = handler
                .send_webhook(
                    &info,
                    &song_title,
                    Some(&image),
                    &webhook_name,
                    &webhook_avatar,
                )
                .await
            {
                tracing::error!("webhook screenshot export failed: {}", e);
            } else {
                info!("webhook screenshot exported for {}", song_title);
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_with_score_info() {
        let info = ScreenshotScoreInfo {
            clear_type_id: 5,
            exscore: 1500,
            max_notes: 1000,
        };
        let text = ScoreTextComposer::compose("Test Song", "Test Artist", Some(&info));
        // 1500*9=13500 vs 2000*6=12000 -> A, vs 2000*7=14000 -> not AA
        assert_eq!(
            text,
            "Test Song / Test Artist\nClear: NORMAL | Rank: A | EX: 1500/2000\n#beatoraja"
        );
    }

    #[test]
    fn compose_without_score_info() {
        let text = ScoreTextComposer::compose("Test Song", "Test Artist", None);
        assert_eq!(text, "Test Song / Test Artist\n#beatoraja");
    }

    #[test]
    fn compose_title_only() {
        let text = ScoreTextComposer::compose("Title Only", "", None);
        assert_eq!(text, "Title Only\n#beatoraja");
    }

    #[test]
    fn compose_artist_only() {
        let text = ScoreTextComposer::compose("", "Artist Only", None);
        assert_eq!(text, "Artist Only\n#beatoraja");
    }

    #[test]
    fn compose_empty_title_and_artist() {
        let text = ScoreTextComposer::compose("", "", None);
        assert_eq!(text, "#beatoraja");
    }

    #[test]
    fn compose_full_combo() {
        let info = ScreenshotScoreInfo {
            clear_type_id: 8,
            exscore: 1800,
            max_notes: 1000,
        };
        let text = ScoreTextComposer::compose("FC Song", "FC Artist", Some(&info));
        assert!(text.contains("Clear: FULL COMBO"));
        assert!(text.contains("EX: 1800/2000"));
    }

    #[test]
    fn compose_failed() {
        let info = ScreenshotScoreInfo {
            clear_type_id: 1,
            exscore: 100,
            max_notes: 1000,
        };
        let text = ScoreTextComposer::compose("Hard Song", "Hard Artist", Some(&info));
        assert!(text.contains("Clear: FAILED"));
        assert!(text.contains("Rank: F"));
    }

    #[test]
    fn compose_max() {
        let info = ScreenshotScoreInfo {
            clear_type_id: 10,
            exscore: 2000,
            max_notes: 1000,
        };
        let text = ScoreTextComposer::compose("Max Song", "Max Artist", Some(&info));
        assert!(text.contains("Clear: MAX"));
        assert!(text.contains("Rank: AAA"));
        assert!(text.contains("EX: 2000/2000"));
    }

    #[test]
    fn compose_all_clear_types() {
        for id in 0..=10 {
            let info = ScreenshotScoreInfo {
                clear_type_id: id,
                exscore: 1000,
                max_notes: 1000,
            };
            let text = ScoreTextComposer::compose("Song", "Artist", Some(&info));
            let expected_clear = clear_type_name(id);
            assert!(
                text.contains(&format!("Clear: {}", expected_clear)),
                "clear_type_id {} should produce '{}'",
                id,
                expected_clear
            );
        }
    }

    #[test]
    fn compose_all_ranks() {
        // max_notes=1000 -> max_ex=2000
        // Rank thresholds: exscore*9 vs max_ex*N
        // AAA: >= 16000 (ex >= 1778), AA: >= 14000 (ex >= 1556)
        // A: >= 12000 (ex >= 1334), B: >= 10000 (ex >= 1112)
        // C: >= 8000 (ex >= 889), D: >= 6000 (ex >= 667)
        // E: >= 4000 (ex >= 445), F: below
        let cases = [
            (1800, 1000, "AAA"),
            (1600, 1000, "AA"),
            (1400, 1000, "A"),
            (1200, 1000, "B"),
            (900, 1000, "C"),
            (700, 1000, "D"),
            (500, 1000, "E"),
            (100, 1000, "F"),
        ];
        for (exscore, max_notes, expected_rank) in cases {
            let info = ScreenshotScoreInfo {
                clear_type_id: 5,
                exscore,
                max_notes,
            };
            let text = ScoreTextComposer::compose("Song", "Artist", Some(&info));
            assert!(
                text.contains(&format!("Rank: {}", expected_rank)),
                "exscore={}, max_notes={} should produce rank '{}'",
                exscore,
                max_notes,
                expected_rank
            );
        }
    }
}
