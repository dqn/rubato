use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::external::screen_shot_exporter;
use crate::external::{
    AbstractResultAccess, ImGuiNotify, IntegerPropertyFactory, MainState, Mode, NUMBER_MAXSCORE,
    ReplayData, STRING_FULLTITLE, STRING_TABLE_LEVEL, STRING_TABLE_NAME, ScoreData, ScreenType,
    StringPropertyFactory,
};

static HTTP_CLIENT: std::sync::OnceLock<reqwest::blocking::Client> = std::sync::OnceLock::new();

fn get_http_client() -> &'static reqwest::blocking::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
    })
}

/// WebhookHandler - handles webhook creation and sending for Discord webhooks.
/// Translated from Java: WebhookHandler
pub struct WebhookHandler;

impl WebhookHandler {
    pub fn new() -> Self {
        Self
    }

    fn write_multipart_field(
        os: &mut dyn Write,
        boundary: &str,
        name: &str,
        value: &str,
    ) -> std::io::Result<()> {
        os.write_all(format!("--{}\r\n", boundary).as_bytes())?;
        os.write_all(format!("Content-Disposition: form-data; name=\"{}\"\r\n", name).as_bytes())?;
        os.write_all(b"\r\n")?;
        os.write_all(value.as_bytes())?;
        os.write_all(b"\r\n")?;
        Ok(())
    }

    fn write_multipart_file(
        os: &mut dyn Write,
        boundary: &str,
        name: &str,
        file_path: &Path,
    ) -> std::io::Result<()> {
        os.write_all(format!("--{}\r\n", boundary).as_bytes())?;
        os.write_all(
            format!(
                "Content-Disposition: form-data; name=\"{}\"; filename=\"screenshot.png\"\r\n",
                name
            )
            .as_bytes(),
        )?;
        os.write_all(b"Content-Type: image/png\r\n")?;
        os.write_all(b"\r\n")?;

        // Write file content
        let file_content = std::fs::read(file_path)?;
        os.write_all(&file_content)?;
        os.write_all(b"\r\n")?;
        Ok(())
    }

    pub fn send_webhook_with_image(&self, payload: &str, image_path: &str, webhook_url: &str) {
        let path = Path::new(image_path);
        if !path.is_file() {
            log::warn!(
                "Webhook screenshot file does not exist or is not a file: {}",
                image_path
            );
            return;
        }

        let result: Result<(), Box<dyn std::error::Error>> = (|| {
            let boundary = format!(
                "----WebKitFormBoundary{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );

            let mut body: Vec<u8> = Vec::new();
            Self::write_multipart_field(&mut body, &boundary, "payload_json", payload)?;
            Self::write_multipart_file(&mut body, &boundary, "files[0]", Path::new(image_path))?;
            body.write_all(format!("--{}--\r\n", boundary).as_bytes())?;

            let client = get_http_client();
            let response = client
                .post(webhook_url)
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={}", boundary),
                )
                .body(body)
                .send()?;

            let response_code = response.status().as_u16();
            if response_code != 200 && response_code != 204 {
                ImGuiNotify::warning(&format!(
                    "Unexpected http response code when sending webhook: {}",
                    response_code
                ));
            }
            Ok(())
        })();

        if let Err(e) = result {
            log::error!("Webhook error: {}", e);
        }
    }

    fn _create_field(name: &str, value: &str) -> HashMap<String, String> {
        let mut field: HashMap<String, String> = HashMap::new();
        field.insert("name".to_string(), name.to_string());
        field.insert("value".to_string(), value.to_string());
        field
    }

    fn create_title(current_state: &MainState) -> String {
        let mut title_string = String::new();

        let table_level =
            StringPropertyFactory::string_property(STRING_TABLE_LEVEL).get(current_state);
        let full_title =
            StringPropertyFactory::string_property(STRING_FULLTITLE).get(current_state);
        let rank = screen_shot_exporter::rank_type_name(current_state);
        let clear_type = screen_shot_exporter::clear_type_name(current_state);

        if !table_level.is_empty() {
            title_string += &table_level;
            title_string += " ";
        }

        if !full_title.is_empty() {
            title_string += &full_title;
            title_string += " ";
        }

        if !rank.is_empty() {
            title_string += &rank;
            title_string += " ";
        }

        if !clear_type.is_empty() {
            title_string += &clear_type;
        }

        title_string
    }

    pub fn create_webhook_payload(
        &self,
        current_state: &MainState,
    ) -> HashMap<String, serde_json::Value> {
        let mut payload: HashMap<String, serde_json::Value> = HashMap::new();

        let webhook_name = current_state
            .resource
            .config()
            .integration
            .webhook_name
            .as_str();
        payload.insert(
            "username".to_string(),
            serde_json::Value::String(if webhook_name.is_empty() {
                "Endless Dream".to_string()
            } else {
                webhook_name.to_string()
            }),
        );
        let webhook_avatar = current_state
            .resource
            .config()
            .integration
            .webhook_avatar
            .as_str();
        payload.insert(
            "avatar_url".to_string(),
            serde_json::Value::String(if webhook_avatar.is_empty() {
                String::new()
            } else {
                webhook_avatar.to_string()
            }),
        );

        if current_state.resource.config().integration.webhook_option == 2 {
            let mut embed: HashMap<String, serde_json::Value> = HashMap::new();
            let mut author: HashMap<String, String> = HashMap::new();

            let mut image: HashMap<String, String> = HashMap::new();
            image.insert("url".to_string(), "attachment://screenshot.png".to_string());
            embed.insert(
                "image".to_string(),
                serde_json::to_value(&image).expect("HashMap<String, _> is always valid JSON"),
            );

            let screen_type = get_screen_type(current_state);

            // Score specific
            if screen_type == ScreenType::MusicResult || screen_type == ScreenType::CourseResult {
                if let Some(result_state) = get_abstract_result(current_state) {
                    let new_score = result_state.new_score();
                    let old_score = result_state.old_score();
                    let max_score = IntegerPropertyFactory::integer_property(NUMBER_MAXSCORE)
                        .get(current_state);

                    let mut description = String::new();
                    description += &format!(
                        "**DJ LEVEL:** {} \n",
                        Self::format_rank(current_state, new_score, max_score)
                    );
                    description += &format!(
                        "**EX SCORE: {}** {}\n",
                        new_score.exscore(),
                        Self::format_diff(new_score.exscore(), old_score.exscore())
                    );
                    description += &format!(
                        "**BAD/POOR: {}** {}\n",
                        Self::get_bp_count(new_score),
                        Self::format_diff(
                            Self::get_bp_count(new_score),
                            Self::get_bp_count(old_score)
                        )
                    );
                    if result_state.ir_rank() != 0 {
                        description += &format!(
                            "**IR RANK: {}/{}** {}\n",
                            result_state.ir_rank(),
                            result_state.ir_total_player(),
                            Self::format_diff(result_state.ir_rank(), result_state.old_ir_rank())
                        );
                    }
                    if *current_state.resource.original_mode() == Mode::BEAT_7K
                        && let Some(rd) = current_state.resource.replay_data()
                    {
                        description += &format!("**PATTERN: {}** \n", Self::format_random(rd));
                    }
                    description += &Self::format_links(current_state);

                    let mut footer: HashMap<String, String> = HashMap::new();
                    embed.insert(
                        "title".to_string(),
                        serde_json::Value::String(Self::create_title(current_state)),
                    );
                    embed.insert(
                        "color".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(
                            screen_shot_exporter::clear_type_colour(current_state),
                        )),
                    );
                    author.insert(
                        "name".to_string(),
                        StringPropertyFactory::string_property(STRING_TABLE_NAME)
                            .get(current_state),
                    );
                    embed.insert(
                        "author".to_string(),
                        serde_json::to_value(&author)
                            .expect("HashMap<String, _> is always valid JSON"),
                    );
                    embed.insert(
                        "description".to_string(),
                        serde_json::Value::String(description),
                    );
                    footer.insert(
                        "text".to_string(),
                        "LR2oraja ~Endless Dream~ Scorecard".to_string(),
                    );
                    embed.insert(
                        "footer".to_string(),
                        serde_json::to_value(&footer)
                            .expect("HashMap<String, _> is always valid JSON"),
                    );
                }
            } else {
                author.insert("name".to_string(), "LR2oraja ~Endless Dream~".to_string());
                embed.insert(
                    "author".to_string(),
                    serde_json::to_value(&author).expect("HashMap<String, _> is always valid JSON"),
                );
            }

            payload.insert(
                "embeds".to_string(),
                serde_json::to_value(vec![embed]).expect("HashMap<String, _> is always valid JSON"),
            );
        }

        payload
    }

    // BAD + POOR + EPOOR
    fn get_bp_count(score: &ScoreData) -> i32 {
        score.judge_count_total(3) + score.judge_count_total(4) + score.judge_count_total(5)
    }

    // Calculates the number used in rank deltas. e.g. AA+76 MAX-133
    fn rank_relative_ex_diff(ex: i32, max: i32, rank_numerator: f32) -> i32 {
        if max == 0 {
            return 0;
        }
        // Even numerators produce [GRADE]+ and odd produces [GRADE]-
        if rank_numerator as i32 % 2 == 0 {
            let grade_ex_target = (max as f32 * rank_numerator / 18.0f32).ceil();
            (ex as f32 - grade_ex_target) as i32
        } else {
            let grade_ex_target = (max as f32 * (rank_numerator + 1.0f32) / 18.0f32).ceil();
            (grade_ex_target - ex as f32) as i32
        }
    }

    // Integer difference + emoji
    fn format_diff(new_score: i32, old_score: i32) -> String {
        let improvement = new_score - old_score;
        if improvement > 0 {
            format!("(+{}) :arrow_up:", improvement)
        } else if improvement < 0 {
            format!("({}) :arrow_down:", improvement)
        } else {
            "(±0) :arrow_right:".to_string()
        }
    }

    fn format_links(current_state: &MainState) -> String {
        let Some(song) = current_state.resource.songdata() else {
            return String::new();
        };
        let mut ss = String::new();
        let md5 = &song.file.md5;
        let lr2ir = "http://www.dream-pro.info/~lavalse/LR2IR/search.cgi?mode=ranking&bmsmd5=";
        if !md5.is_empty() {
            ss += &format!(" [LR2IR]({}{})", lr2ir, md5);
        }
        let charturl = "https://bms-score-viewer.pages.dev/view?md5=";
        if !md5.is_empty() {
            ss += " |";
        }
        ss += &format!(" [Chart]({}{})", charturl, md5);

        let levels = current_state.resource.reverse_lookup_levels();
        for level in levels {
            ss += &format!(" | {}", level);
        }
        ss
    }

    fn format_percent(new_score: &ScoreData, max_score: i32) -> String {
        if max_score == 0 {
            return "(0.00%)".to_string();
        }
        let percent = 100.0f32 * (new_score.exscore() as f32 / max_score as f32);
        format!("({:.2}%)", percent)
    }

    // Makes rank string in "[GRADE][+/-][Relative diff] ([percent]) [emoji]" format.
    // e.g "AAA-53 (86.53%) :arrow_up:"
    fn format_rank(current_state: &MainState, new_score: &ScoreData, max_score: i32) -> String {
        let ex = new_score.exscore();
        let percent = if max_score == 0 {
            0.0f32
        } else {
            100.0f32 * (ex as f32 / max_score as f32)
        };
        let mut sb = String::new();
        let mut current_rank: i32 = 0;
        let mut old_rank: i32 = 0;

        for rank in &GRADE_RANKS {
            if percent > rank.percent() {
                current_rank = ((rank.numerator() / 2.0f32).floor() * 2.0f32) as i32;
                sb += &format!(
                    "**{}{}**",
                    rank.text(),
                    Self::rank_relative_ex_diff(ex, max_score, rank.numerator())
                );
                break;
            }
        }

        if let Some(result_state) = get_abstract_result(current_state) {
            let old_score = result_state.old_score();
            let old_percent = if max_score == 0 {
                0.0f32
            } else {
                100.0f32 * old_score.exscore() as f32 / max_score as f32
            };
            for rank in &GRADE_RANKS {
                if old_percent > rank.percent() {
                    old_rank = ((rank.numerator() / 2.0f32).floor() * 2.0f32) as i32;
                    break;
                }
            }
        }
        sb += &format!(" {}", Self::format_percent(new_score, max_score));

        if current_rank > old_rank {
            sb += " :arrow_up:";
        } else if current_rank < old_rank {
            sb += " :arrow_down:";
        } else {
            sb += " :arrow_right:";
        }
        sb
    }

    // magic numbers identified in Random.java
    fn format_random(rd: &ReplayData) -> String {
        let mut sb = String::new();

        match rd.randomoption {
            0 => {
                // IDENTITY
                sb += "1234567";
            }
            1 => {
                // MIRROR
                sb += "7654321";
            }
            2 | 3 => {
                // RANDOM, R-RAN
                if let Some(ref pattern) = rd.lane_shuffle_pattern
                    && !pattern.is_empty()
                {
                    for &val in pattern[0].iter().take(7) {
                        sb += &format!("{}", val + 1);
                    }
                }
            }
            4 => {
                sb += "SRAN";
            }
            5 => {
                sb += "SPIRAL";
            }
            6 => {
                sb += "HRAN";
            }
            7 => {
                sb += "ALLSCR";
            }
            _ => {
                sb += "N/A";
            }
        }
        sb
    }
}

impl Default for WebhookHandler {
    fn default() -> Self {
        Self::new()
    }
}

// 7/9 == 14/18 == 77.77% == AA
// exact ex matches on the grade boundary are by convention "[GRADE]-0"
/// GradeRank enum.
/// Translated from Java: WebhookHandler.GradeRank
#[derive(Clone, Debug)]
pub struct GradeRank {
    numerator: f32,
    text: &'static str,
}

impl GradeRank {
    const fn new(numerator: f32, text: &'static str) -> Self {
        Self { numerator, text }
    }

    pub fn percent(&self) -> f32 {
        (self.numerator / 18.0f32) * 100.0f32
    }

    pub fn text(&self) -> &str {
        self.text
    }

    pub fn numerator(&self) -> f32 {
        self.numerator
    }
}

// NOTE: The gaps (no B-, C-, D-, E-, F- entries) match the Java source intentionally.
// Java WebhookHandler.GradeRank enum has exactly these 12 entries with the same numerators.
static GRADE_RANKS: [GradeRank; 12] = [
    GradeRank::new(17.0, "MAX-"),
    GradeRank::new(16.0, "AAA+"),
    GradeRank::new(15.0, "AAA-"),
    GradeRank::new(14.0, "AA+"),
    GradeRank::new(13.0, "AA-"),
    GradeRank::new(12.0, "A+"),
    GradeRank::new(11.0, "A-"),
    GradeRank::new(10.0, "B+"),
    GradeRank::new(8.0, "C+"),
    GradeRank::new(6.0, "D+"),
    GradeRank::new(4.0, "E+"),
    GradeRank::new(0.0, "F+"),
];

/// Determine the screen type from state.
/// In Java this was done via instanceof checks; in Rust the MainState carries
/// its screen type and exposes it via MainStateAccess::get_screen_type().
fn get_screen_type(state: &MainState) -> ScreenType {
    use crate::main_state_access::MainStateAccess;
    state.screen_type()
}

/// Get the AbstractResult from the current state.
/// In Java this was done via cast: ((AbstractResult) currentState).
/// In Rust the MainState carries an optional AbstractResultAccess field,
/// populated when the current screen is MusicResult or CourseResult.
fn get_abstract_result(state: &MainState) -> Option<&dyn AbstractResultAccess> {
    state
        .abstract_result
        .as_deref()
        .map(|r| r as &dyn AbstractResultAccess)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_state(screen_type: ScreenType) -> MainState {
        MainState {
            resource: Default::default(),
            screen_type,
            abstract_result: None,
        }
    }

    #[test]
    fn rank_relative_ex_diff_returns_zero_when_max_score_is_zero() {
        // When max_score=0, there is no meaningful grade boundary, so delta must be 0.
        assert_eq!(WebhookHandler::rank_relative_ex_diff(100, 0, 17.0), 0);
        assert_eq!(WebhookHandler::rank_relative_ex_diff(100, 0, 16.0), 0);
        assert_eq!(WebhookHandler::rank_relative_ex_diff(0, 0, 14.0), 0);
        assert_eq!(WebhookHandler::rank_relative_ex_diff(50, 0, 13.0), 0);
    }

    #[test]
    fn rank_relative_ex_diff_normal_case() {
        // max=1800, numerator=14.0 (AA+): grade_ex_target = ceil(1800 * 14 / 18) = ceil(1400) = 1400
        // ex=1500 => 1500 - 1400 = 100
        assert_eq!(WebhookHandler::rank_relative_ex_diff(1500, 1800, 14.0), 100);
        // max=1800, numerator=13.0 (AA-): grade_ex_target = ceil(1800 * 14 / 18) = 1400
        // result = 1400 - 1300 = 100
        assert_eq!(WebhookHandler::rank_relative_ex_diff(1300, 1800, 13.0), 100);
    }

    #[test]
    fn send_webhook_with_image_returns_early_on_missing_file() {
        // Calling with a non-existent file should return early (log warning)
        // without panicking or attempting a network request.
        let handler = WebhookHandler::new();
        handler.send_webhook_with_image(
            r#"{"content":"test"}"#,
            "/nonexistent/path/screenshot.png",
            "https://invalid.webhook.url/not-called",
        );
        // No panic = success. The function returns before any HTTP call.
    }

    #[test]
    fn get_screen_type_delegates_to_main_state_access() {
        assert_eq!(
            get_screen_type(&make_state(ScreenType::MusicResult)),
            ScreenType::MusicResult
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::CourseResult)),
            ScreenType::CourseResult
        );
        assert_eq!(
            get_screen_type(&make_state(ScreenType::Other)),
            ScreenType::Other
        );
    }
}
