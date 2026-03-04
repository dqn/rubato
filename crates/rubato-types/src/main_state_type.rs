/// MainStateType - enum for each state in the application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainStateType {
    MusicSelect,
    Decide,
    Play,
    Result,
    CourseResult,
    Config,
    SkinConfig,
}
