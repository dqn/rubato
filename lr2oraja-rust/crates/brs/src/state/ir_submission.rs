// IR submission helpers for fire-and-forget score/course submission.
//
// Sends play data to the Internet Ranking server asynchronously.
// Failures are logged but never propagate to the caller.

use bms_ir::chart_data::IRChartData;
use bms_ir::connection::IRConnection;
use bms_ir::connection_manager::IRConnectionManager;
use bms_ir::course_data::IRCourseData;
use bms_ir::score_data::IRScoreData;
use bms_rule::ScoreData;

/// Submit a play score to the IR server (fire-and-forget).
///
/// Creates an LR2IR connection, converts the score to IR format,
/// and spawns an async task to send it. Failures are logged as warnings.
/// No-op if called outside a Tokio runtime context.
pub fn submit_score_to_ir(score: &ScoreData, sha256: &str, lntype: i32) {
    // Guard: only spawn task if tokio runtime is available
    let Ok(_handle) = tokio::runtime::Handle::try_current() else {
        return;
    };

    let ir_score = IRScoreData::from(score);
    let chart = IRChartData {
        sha256: sha256.to_string(),
        lntype,
        ..minimal_chart_data()
    };

    tokio::spawn(async move {
        let Some(conn) = IRConnectionManager::create("LR2IR") else {
            tracing::warn!("IR: LR2IR connection not available");
            return;
        };
        match conn.send_play_data(&chart, &ir_score).await {
            Ok(resp) if resp.succeeded => {
                tracing::info!("IR: score submitted successfully");
            }
            Ok(resp) => {
                tracing::warn!("IR: score submission rejected: {}", resp.message);
            }
            Err(e) => {
                tracing::warn!("IR: score submission failed: {e}");
            }
        }
    });
}

/// Submit a course score to the IR server (fire-and-forget).
///
/// Converts `CourseData` to `IRCourseData` and sends via `send_course_play_data`.
/// No-op if called outside a Tokio runtime context.
pub fn submit_course_score_to_ir(score: &ScoreData, course: &bms_database::CourseData) {
    // Guard: only spawn task if tokio runtime is available
    let Ok(_handle) = tokio::runtime::Handle::try_current() else {
        return;
    };

    let ir_score = IRScoreData::from(score);
    let ir_course = ir_course_data_from(course);

    tokio::spawn(async move {
        let Some(conn) = IRConnectionManager::create("LR2IR") else {
            tracing::warn!("IR: LR2IR connection not available");
            return;
        };
        match conn.send_course_play_data(&ir_course, &ir_score).await {
            Ok(resp) if resp.succeeded => {
                tracing::info!("IR: course score submitted successfully");
            }
            Ok(resp) => {
                tracing::warn!("IR: course score submission rejected: {}", resp.message);
            }
            Err(e) => {
                tracing::warn!("IR: course score submission failed: {e}");
            }
        }
    });
}

/// Create a minimal IRChartData with only sha256 and lntype populated.
fn minimal_chart_data() -> IRChartData {
    IRChartData {
        md5: String::new(),
        sha256: String::new(),
        title: String::new(),
        subtitle: String::new(),
        genre: String::new(),
        artist: String::new(),
        subartist: String::new(),
        url: String::new(),
        appendurl: String::new(),
        level: 0,
        total: 0,
        mode: 0,
        lntype: 0,
        judge: 0,
        minbpm: 0,
        maxbpm: 0,
        notes: 0,
        has_undefined_ln: false,
        has_ln: false,
        has_cn: false,
        has_hcn: false,
        has_mine: false,
        has_random: false,
        has_stop: false,
        values: std::collections::HashMap::new(),
    }
}

/// Convert bms_database::CourseDataConstraint to bms_ir::CourseDataConstraint.
fn convert_constraint(
    c: &bms_database::CourseDataConstraint,
) -> bms_ir::course_data::CourseDataConstraint {
    use bms_database::CourseDataConstraint as Db;
    use bms_ir::course_data::CourseDataConstraint as Ir;

    match c {
        Db::Class => Ir::Class,
        Db::GradeMirror => Ir::Mirror,
        Db::GradeRandom => Ir::Random,
        Db::NoSpeed => Ir::NoSpeed,
        Db::NoGood => Ir::NoGood,
        Db::NoGreat => Ir::NoGreat,
        Db::GaugeLr2 => Ir::GaugeLr2,
        Db::Gauge5Keys => Ir::Gauge5Keys,
        Db::Gauge7Keys => Ir::Gauge7Keys,
        Db::Gauge9Keys => Ir::Gauge9Keys,
        Db::Gauge24Keys => Ir::Gauge24Keys,
        Db::Ln => Ir::Ln,
        Db::Cn => Ir::Cn,
        Db::Hcn => Ir::Hcn,
    }
}

/// Convert bms_database::CourseData to bms_ir::IRCourseData.
fn ir_course_data_from(course: &bms_database::CourseData) -> IRCourseData {
    use bms_ir::course_data::IRTrophyData;

    let constraints = course.constraint.iter().map(convert_constraint).collect();

    let trophies: Vec<IRTrophyData> = course
        .trophy
        .iter()
        .map(|t| IRTrophyData {
            name: t.name.clone(),
            scorerate: t.scorerate,
            smissrate: t.missrate,
        })
        .collect();

    let charts: Vec<IRChartData> = course
        .hash
        .iter()
        .map(|h| IRChartData {
            sha256: h.sha256.clone(),
            ..minimal_chart_data()
        })
        .collect();

    IRCourseData {
        name: course.name.clone(),
        charts,
        constraint: constraints,
        trophy: trophies,
        lntype: -1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_database::course_data::CourseSongData;
    use bms_rule::ClearType;

    #[test]
    fn submit_score_does_not_panic_without_runtime() {
        let score = ScoreData {
            sha256: "test_hash".to_string(),
            mode: 0,
            clear: ClearType::Normal,
            epg: 100,
            ..Default::default()
        };

        // Guard: only call if tokio runtime is available
        if tokio::runtime::Handle::try_current().is_ok() {
            submit_score_to_ir(&score, "test_hash", 0);
        }
    }

    #[test]
    fn ir_course_data_conversion() {
        use bms_database::CourseDataConstraint as DbC;

        let course = bms_database::CourseData {
            name: "Test Course".to_string(),
            hash: vec![
                CourseSongData {
                    sha256: "hash1".to_string(),
                    ..Default::default()
                },
                CourseSongData {
                    sha256: "hash2".to_string(),
                    ..Default::default()
                },
            ],
            constraint: vec![DbC::Class, DbC::NoSpeed],
            trophy: vec![bms_database::TrophyData {
                name: "Gold".to_string(),
                missrate: 1.0,
                scorerate: 90.0,
            }],
            release: true,
        };

        let ir = ir_course_data_from(&course);
        assert_eq!(ir.name, "Test Course");
        assert_eq!(ir.charts.len(), 2);
        assert_eq!(ir.charts[0].sha256, "hash1");
        assert_eq!(ir.constraint.len(), 2);
        assert_eq!(ir.trophy.len(), 1);
        assert_eq!(ir.trophy[0].name, "Gold");
    }

    #[test]
    fn minimal_chart_data_is_empty() {
        let chart = minimal_chart_data();
        assert!(chart.sha256.is_empty());
        assert_eq!(chart.lntype, 0);
        assert_eq!(chart.notes, 0);
    }
}
