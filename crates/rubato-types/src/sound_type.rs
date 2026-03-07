/// SoundType - system sound types
///
/// Moved from beatoraja-core to beatoraja-types to allow use in MainControllerAccess trait.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundType {
    Scratch,
    FolderOpen,
    FolderClose,
    OptionChange,
    OptionOpen,
    OptionClose,
    PlayReady,
    PlayStop,
    ResultClear,
    ResultFail,
    ResultClose,
    CourseClear,
    CourseFail,
    CourseClose,
    GuidesePg,
    GuideseGr,
    GuideseGd,
    GuideseBd,
    GuidesePr,
    GuideseMs,
    Select,
    Decide,
}

impl SoundType {
    pub fn is_bgm(&self) -> bool {
        matches!(self, SoundType::Select | SoundType::Decide)
    }

    pub fn path(&self) -> &str {
        match self {
            SoundType::Scratch => "scratch.wav",
            SoundType::FolderOpen => "f-open.wav",
            SoundType::FolderClose => "f-close.wav",
            SoundType::OptionChange => "o-change.wav",
            SoundType::OptionOpen => "o-open.wav",
            SoundType::OptionClose => "o-close.wav",
            SoundType::PlayReady => "playready.wav",
            SoundType::PlayStop => "playstop.wav",
            SoundType::ResultClear => "clear.wav",
            SoundType::ResultFail => "fail.wav",
            SoundType::ResultClose => "resultclose.wav",
            SoundType::CourseClear => "course_clear.wav",
            SoundType::CourseFail => "course_fail.wav",
            SoundType::CourseClose => "course_close.wav",
            SoundType::GuidesePg => "guide-pg.wav",
            SoundType::GuideseGr => "guide-gr.wav",
            SoundType::GuideseGd => "guide-gd.wav",
            SoundType::GuideseBd => "guide-bd.wav",
            SoundType::GuidesePr => "guide-pr.wav",
            SoundType::GuideseMs => "guide-ms.wav",
            SoundType::Select => "select.wav",
            SoundType::Decide => "decide.wav",
        }
    }

    pub fn values() -> &'static [SoundType] {
        &[
            SoundType::Scratch,
            SoundType::FolderOpen,
            SoundType::FolderClose,
            SoundType::OptionChange,
            SoundType::OptionOpen,
            SoundType::OptionClose,
            SoundType::PlayReady,
            SoundType::PlayStop,
            SoundType::ResultClear,
            SoundType::ResultFail,
            SoundType::ResultClose,
            SoundType::CourseClear,
            SoundType::CourseFail,
            SoundType::CourseClose,
            SoundType::GuidesePg,
            SoundType::GuideseGr,
            SoundType::GuideseGd,
            SoundType::GuideseBd,
            SoundType::GuidesePr,
            SoundType::GuideseMs,
            SoundType::Select,
            SoundType::Decide,
        ]
    }
}
