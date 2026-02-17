pub mod payload;

use std::time::Duration;

use anyhow::Result;
use tracing::{error, info};

use self::payload::WebhookPayload;
use crate::screenshot::ScreenshotScoreInfo;

/// Webhook handler for sending score results to Discord webhooks.
pub struct WebhookHandler {
    client: reqwest::Client,
    urls: Vec<String>,
}

const WEBHOOK_CONNECT_TIMEOUT_SECS: u64 = 5;
const WEBHOOK_REQUEST_TIMEOUT_SECS: u64 = 15;

fn mask_webhook_url(url: &str) -> String {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return "<invalid-url>".to_string();
    };

    let Some(host) = parsed.host_str() else {
        return "<invalid-url>".to_string();
    };

    let token_hint = parsed
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .map(|token| {
            let tail_len = token.chars().count().min(4);
            let tail: String = token
                .chars()
                .rev()
                .take(tail_len)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect();
            if tail.is_empty() {
                "***".to_string()
            } else {
                format!("***{tail}")
            }
        })
        .unwrap_or_else(|| "***".to_string());

    format!("{host}/.../{token_hint}")
}

impl WebhookHandler {
    pub fn new(urls: Vec<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .connect_timeout(Duration::from_secs(WEBHOOK_CONNECT_TIMEOUT_SECS))
                .timeout(Duration::from_secs(WEBHOOK_REQUEST_TIMEOUT_SECS))
                .build()
                .unwrap_or_default(),
            urls,
        }
    }

    /// Send a webhook with score information and optional screenshot.
    pub async fn send_webhook(
        &self,
        score_info: &ScreenshotScoreInfo,
        song_title: &str,
        screenshot_data: Option<&[u8]>,
        webhook_name: &str,
        webhook_avatar: &str,
    ) -> Result<()> {
        let embed = payload::create_embed(score_info, song_title);
        let payload = WebhookPayload {
            username: if webhook_name.is_empty() {
                None
            } else {
                Some(webhook_name.to_string())
            },
            avatar_url: if webhook_avatar.is_empty() {
                None
            } else {
                Some(webhook_avatar.to_string())
            },
            embeds: vec![embed],
        };

        for url in &self.urls {
            if url.is_empty() {
                continue;
            }
            let masked_url = mask_webhook_url(url);
            if let Err(e) = self.send_to_url(url, &payload, screenshot_data).await {
                error!("webhook send failed for {}: {}", masked_url, e);
            } else {
                info!("webhook sent to {}", masked_url);
            }
        }

        Ok(())
    }

    async fn send_to_url(
        &self,
        url: &str,
        payload: &WebhookPayload,
        screenshot_data: Option<&[u8]>,
    ) -> Result<()> {
        if let Some(image) = screenshot_data {
            // Multipart: JSON payload + image attachment
            let payload_json = serde_json::to_string(payload)?;
            let form = reqwest::multipart::Form::new()
                .text("payload_json", payload_json)
                .part(
                    "file",
                    reqwest::multipart::Part::bytes(image.to_vec())
                        .file_name("screenshot.png")
                        .mime_str("image/png")?,
                );
            self.client
                .post(url)
                .multipart(form)
                .send()
                .await?
                .error_for_status()?;
        } else {
            self.client
                .post(url)
                .json(payload)
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_webhook_url_hides_secret_parts() {
        let masked = mask_webhook_url(
            "https://discord.com/api/webhooks/123456789012345678/abcdefghijklmnopqrstuvwxyz",
        );
        assert_eq!(masked, "discord.com/.../***wxyz");
        assert!(!masked.contains("123456789012345678"));
        assert!(!masked.contains("abcdefghijklmnopqrstuvwxyz"));
    }

    #[test]
    fn mask_webhook_url_handles_invalid_url() {
        let masked = mask_webhook_url("not-a-url");
        assert_eq!(masked, "<invalid-url>");
    }

    #[test]
    fn webhook_handler_new() {
        let handler = WebhookHandler::new(vec![
            "https://example.com/webhook1".to_string(),
            "https://example.com/webhook2".to_string(),
        ]);
        assert_eq!(handler.urls.len(), 2);
    }

    #[test]
    fn webhook_handler_empty_urls() {
        let handler = WebhookHandler::new(vec![]);
        assert!(handler.urls.is_empty());
    }
}
