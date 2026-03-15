// WiringCheck trait: test-only framework for detecting integration wiring bugs.
//
// Components may compile and pass unit tests individually, but if they're not
// connected correctly (missing textures, empty event queues, wrong timer IDs),
// the result is invisible failures at runtime. This trait provides a structured
// way to check that skin objects are properly wired before rendering.

/// Severity of a wiring issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Likely to cause invisible rendering (e.g., missing texture).
    Error,
    /// May cause degraded rendering (e.g., default fallback used).
    Warning,
}

/// A detected wiring issue.
#[derive(Debug, Clone)]
pub struct WiringIssue {
    pub severity: Severity,
    pub component: &'static str,
    pub field: String,
    pub message: String,
}

/// Trait for checking that a skin object is properly wired for rendering.
///
/// Implementations should check all fields that must be non-default/non-empty
/// for the object to render correctly. Returns a list of issues found.
pub trait WiringCheck {
    fn check_wiring(&self) -> Vec<WiringIssue>;
}

// --- SkinNoteObject ---

impl WiringCheck for super::skin_note_object::SkinNoteObject {
    fn check_wiring(&self) -> Vec<WiringIssue> {
        let mut issues = Vec::new();
        let lane_count = self.note_images.len();

        // Check: at least one note texture should be wired
        let any_note = self.note_images.iter().any(|img| img.is_some());
        if !any_note {
            issues.push(WiringIssue {
                severity: Severity::Error,
                component: "SkinNoteObject",
                field: "note_images".to_string(),
                message: format!(
                    "all {lane_count} lanes have None note textures - notes will be invisible"
                ),
            });
        }

        // Check individual lanes: warn about holes
        for (i, img) in self.note_images.iter().enumerate() {
            if img.is_none() && any_note {
                issues.push(WiringIssue {
                    severity: Severity::Warning,
                    component: "SkinNoteObject",
                    field: format!("note_images[{i}]"),
                    message: format!("lane {i} has no note texture (other lanes do)"),
                });
            }
        }

        issues
    }
}

// --- SkinJudgeObject ---

impl WiringCheck for super::skin_judge_object::SkinJudgeObject {
    fn check_wiring(&self) -> Vec<WiringIssue> {
        let mut issues = Vec::new();

        // judge_images is [Option<SkinImage>; 7] — at least one should be Some
        let any_judge = self.judge_images().iter().any(|img| img.is_some());
        if !any_judge {
            issues.push(WiringIssue {
                severity: Severity::Error,
                component: "SkinJudgeObject",
                field: "judge_images".to_string(),
                message: "no judge images wired - judge display will be invisible".to_string(),
            });
        }

        issues
    }
}

// --- SkinImage ---

impl WiringCheck for super::skin_image::SkinImage {
    fn check_wiring(&self) -> Vec<WiringIssue> {
        let mut issues = Vec::new();

        if self.source_count() == 0 {
            issues.push(WiringIssue {
                severity: Severity::Warning,
                component: "SkinImage",
                field: "image".to_string(),
                message: "no image sources set - SkinImage will not render".to_string(),
            });
        }

        issues
    }
}

// --- SkinGauge ---

impl WiringCheck for super::skin_gauge::SkinGauge {
    fn check_wiring(&self) -> Vec<WiringIssue> {
        let mut issues = Vec::new();

        if self.parts <= 0 {
            issues.push(WiringIssue {
                severity: Severity::Error,
                component: "SkinGauge",
                field: "parts".to_string(),
                message: format!("parts={} - gauge will not render (needs > 0)", self.parts),
            });
        }

        issues
    }
}

// --- SkinBgaObject ---

impl WiringCheck for super::skin_bga_object::SkinBgaObject {
    fn check_wiring(&self) -> Vec<WiringIssue> {
        let mut issues = Vec::new();

        if !self.has_bga_draw() {
            issues.push(WiringIssue {
                severity: Severity::Warning,
                component: "SkinBgaObject",
                field: "bga_draw".to_string(),
                message: "bga_draw not set - BGA will not render until set_bga_draw() is called"
                    .to_string(),
            });
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::TextureRegion;
    use crate::skin_note_object::SkinNoteObject;

    #[test]
    fn note_object_with_no_textures_reports_error() {
        let note = SkinNoteObject::new(8);
        let issues = note.check_wiring();
        assert!(
            issues.iter().any(|i| i.severity == Severity::Error),
            "SkinNoteObject with no textures should report an Error"
        );
    }

    #[test]
    fn note_object_with_all_textures_reports_no_errors() {
        let mut note = SkinNoteObject::new(8);
        for img in &mut note.note_images {
            *img = Some(TextureRegion {
                region_width: 64,
                region_height: 16,
                ..Default::default()
            });
        }
        let issues = note.check_wiring();
        assert!(
            !issues.iter().any(|i| i.severity == Severity::Error),
            "SkinNoteObject with all textures wired should have no errors"
        );
    }

    #[test]
    fn note_object_with_partial_textures_reports_warnings() {
        let mut note = SkinNoteObject::new(8);
        // Only wire lane 0
        note.note_images[0] = Some(TextureRegion {
            region_width: 64,
            region_height: 16,
            ..Default::default()
        });
        let issues = note.check_wiring();
        // Should have warnings for lanes 1-7 but no errors
        assert!(
            !issues.iter().any(|i| i.severity == Severity::Error),
            "partial wiring should not be an error"
        );
        let warnings: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .collect();
        assert_eq!(warnings.len(), 7, "should warn about 7 unwired lanes");
    }

    // --- SkinGauge ---

    #[test]
    fn gauge_with_valid_parts_reports_no_errors() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let gauge = crate::objects::skin_gauge::SkinGauge::new(images, 0, 0, 10, 0, 2, 100);
        let issues = gauge.check_wiring();
        assert!(
            !issues.iter().any(|i| i.severity == Severity::Error),
            "SkinGauge with parts=10 should have no errors"
        );
    }

    #[test]
    fn gauge_with_zero_parts_reports_error() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let gauge = crate::objects::skin_gauge::SkinGauge::new(images, 0, 0, 0, 0, 2, 100);
        let issues = gauge.check_wiring();
        assert!(
            issues.iter().any(|i| i.severity == Severity::Error),
            "SkinGauge with parts=0 should report an Error"
        );
    }

    // --- SkinBgaObject ---

    #[test]
    fn bga_without_draw_reports_warning() {
        let bga = crate::objects::skin_bga_object::SkinBgaObject::new(0);
        let issues = bga.check_wiring();
        assert!(
            issues
                .iter()
                .any(|i| i.severity == Severity::Warning && i.component == "SkinBgaObject"),
            "SkinBgaObject without bga_draw should report a Warning"
        );
    }
}
