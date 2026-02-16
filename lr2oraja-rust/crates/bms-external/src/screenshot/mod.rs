pub mod file_exporter;
pub mod social_exporter;

/// Information about a score for screenshot metadata.
#[derive(Debug, Clone)]
pub struct ScreenshotScoreInfo {
    pub clear_type_id: u8,
    pub exscore: i32,
    pub max_notes: i32,
}

/// Convert clear type ID to display name.
pub fn clear_type_name(clear_type_id: u8) -> &'static str {
    match clear_type_id {
        0 => "NO PLAY",
        1 => "FAILED",
        2 => "ASSIST EASY",
        3 => "LIGHT ASSIST EASY",
        4 => "EASY",
        5 => "NORMAL",
        6 => "HARD",
        7 => "EX HARD",
        8 => "FULL COMBO",
        9 => "PERFECT",
        10 => "MAX",
        _ => "UNKNOWN",
    }
}

/// Determine rank name from EX score and max notes.
///
/// Rank thresholds based on EX score ratio to max EX score (notes * 2):
/// AAA: >= 8/9, AA: >= 7/9, A: >= 6/9, B: >= 5/9,
/// C: >= 4/9, D: >= 3/9, E: >= 2/9, F: < 2/9
pub fn rank_name(exscore: i32, max_notes: i32) -> &'static str {
    if max_notes <= 0 {
        return "F";
    }
    let max_ex = max_notes * 2;
    // Use integer arithmetic: exscore * 9 vs max_ex * threshold
    let scaled = exscore * 9;
    if scaled >= max_ex * 8 {
        "AAA"
    } else if scaled >= max_ex * 7 {
        "AA"
    } else if scaled >= max_ex * 6 {
        "A"
    } else if scaled >= max_ex * 5 {
        "B"
    } else if scaled >= max_ex * 4 {
        "C"
    } else if scaled >= max_ex * 3 {
        "D"
    } else if scaled >= max_ex * 2 {
        "E"
    } else {
        "F"
    }
}

/// Trait for exporting screenshots.
pub trait ScreenshotExporter: Send + Sync {
    fn send(
        &self,
        image_data: &[u8],
        state_name: &str,
        score_info: Option<&ScreenshotScoreInfo>,
    ) -> anyhow::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_type_name_all_variants() {
        assert_eq!(clear_type_name(0), "NO PLAY");
        assert_eq!(clear_type_name(1), "FAILED");
        assert_eq!(clear_type_name(2), "ASSIST EASY");
        assert_eq!(clear_type_name(3), "LIGHT ASSIST EASY");
        assert_eq!(clear_type_name(4), "EASY");
        assert_eq!(clear_type_name(5), "NORMAL");
        assert_eq!(clear_type_name(6), "HARD");
        assert_eq!(clear_type_name(7), "EX HARD");
        assert_eq!(clear_type_name(8), "FULL COMBO");
        assert_eq!(clear_type_name(9), "PERFECT");
        assert_eq!(clear_type_name(10), "MAX");
        assert_eq!(clear_type_name(11), "UNKNOWN");
        assert_eq!(clear_type_name(255), "UNKNOWN");
    }

    #[test]
    fn rank_name_thresholds() {
        // max_notes=100 -> max_ex=200
        assert_eq!(rank_name(200, 100), "AAA"); // 200/200 = 1.0 >= 8/9
        assert_eq!(rank_name(178, 100), "AAA"); // 178*9=1602 >= 200*8=1600
        assert_eq!(rank_name(177, 100), "AA"); // 177*9=1593 < 1600
        assert_eq!(rank_name(156, 100), "AA"); // 156*9=1404 >= 200*7=1400
        assert_eq!(rank_name(155, 100), "A"); // 155*9=1395 < 1400
        assert_eq!(rank_name(134, 100), "A"); // 134*9=1206 >= 200*6=1200
        assert_eq!(rank_name(133, 100), "B"); // 133*9=1197 < 1200
        assert_eq!(rank_name(112, 100), "B"); // 112*9=1008 >= 200*5=1000
        assert_eq!(rank_name(111, 100), "C"); // 111*9=999 < 1000
        assert_eq!(rank_name(89, 100), "C"); // 89*9=801 >= 200*4=800
        assert_eq!(rank_name(88, 100), "D"); // 88*9=792 < 800
        assert_eq!(rank_name(67, 100), "D"); // 67*9=603 >= 200*3=600
        assert_eq!(rank_name(66, 100), "E"); // 66*9=594 < 600
        assert_eq!(rank_name(45, 100), "E"); // 45*9=405 >= 200*2=400
        assert_eq!(rank_name(44, 100), "F"); // 44*9=396 < 400
        assert_eq!(rank_name(0, 100), "F");
    }

    #[test]
    fn rank_name_zero_notes() {
        assert_eq!(rank_name(0, 0), "F");
        assert_eq!(rank_name(100, 0), "F");
        assert_eq!(rank_name(0, -1), "F");
    }
}
