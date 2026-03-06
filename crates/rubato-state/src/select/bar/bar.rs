use crate::select::stubs::ScoreData;

/// Dispatch a method call uniformly across all 14 Bar variants.
///
/// Only usable when every variant delegates to the same method with the same
/// arguments. Methods like `lamp` and `bar_data` have variant-specific field
/// paths and must remain as explicit match blocks.
macro_rules! bar_dispatch {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            Bar::Song(b) => b.$method($($arg),*),
            Bar::Folder(b) => b.$method($($arg),*),
            Bar::Command(b) => b.$method($($arg),*),
            Bar::Container(b) => b.$method($($arg),*),
            Bar::Hash(b) => b.$method($($arg),*),
            Bar::Table(b) => b.$method($($arg),*),
            Bar::Grade(b) => b.$method($($arg),*),
            Bar::RandomCourse(b) => b.$method($($arg),*),
            Bar::SearchWord(b) => b.$method($($arg),*),
            Bar::SameFolder(b) => b.$method($($arg),*),
            Bar::Executable(b) => b.$method($($arg),*),
            Bar::Function(b) => b.$method($($arg),*),
            Bar::ContextMenu(b) => b.$method($($arg),*),
            Bar::LeaderBoard(b) => b.$method($($arg),*),
        }
    };
}

/// Shared data for all bar types
/// Translates: bms.player.beatoraja.select.bar.Bar
#[derive(Clone, Debug, Default)]
pub struct BarData {
    /// Player score
    pub score: Option<ScoreData>,
    /// Rival score
    pub rscore: Option<ScoreData>,
}

impl BarData {
    pub fn score(&self) -> Option<&ScoreData> {
        self.score.as_ref()
    }

    pub fn set_score(&mut self, score: Option<ScoreData>) {
        self.score = score;
    }

    pub fn rival_score(&self) -> Option<&ScoreData> {
        self.rscore.as_ref()
    }

    pub fn set_rival_score(&mut self, score: Option<ScoreData>) {
        self.rscore = score;
    }
}

/// Bar enum representing all bar types in the select screen
#[derive(Clone)]
pub enum Bar {
    Song(Box<super::song_bar::SongBar>),
    Folder(Box<super::folder_bar::FolderBar>),
    Command(Box<super::command_bar::CommandBar>),
    Container(Box<super::container_bar::ContainerBar>),
    Hash(Box<super::hash_bar::HashBar>),
    Table(Box<super::table_bar::TableBar>),
    Grade(Box<super::grade_bar::GradeBar>),
    RandomCourse(Box<super::random_course_bar::RandomCourseBar>),
    SearchWord(Box<super::search_word_bar::SearchWordBar>),
    SameFolder(Box<super::same_folder_bar::SameFolderBar>),
    Executable(Box<super::executable_bar::ExecutableBar>),
    Function(Box<super::function_bar::FunctionBar>),
    ContextMenu(Box<super::context_menu_bar::ContextMenuBar>),
    LeaderBoard(Box<super::leader_board_bar::LeaderBoardBar>),
}

impl Bar {
    pub fn title(&self) -> String {
        bar_dispatch!(self, title)
    }

    pub fn score(&self) -> Option<&ScoreData> {
        self.bar_data().score()
    }

    pub fn set_score(&mut self, score: Option<ScoreData>) {
        self.bar_data_mut().set_score(score);
    }

    pub fn rival_score(&self) -> Option<&ScoreData> {
        self.bar_data().rival_score()
    }

    pub fn set_rival_score(&mut self, score: Option<ScoreData>) {
        self.bar_data_mut().set_rival_score(score);
    }

    pub fn lamp(&self, is_player: bool) -> i32 {
        match self {
            Bar::Song(b) => b.lamp(is_player),
            Bar::Folder(b) => b.directory.lamp(is_player),
            Bar::Command(b) => b.directory.lamp(is_player),
            Bar::Container(b) => b.directory.lamp(is_player),
            Bar::Hash(b) => b.directory.lamp(is_player),
            Bar::Table(b) => b.directory.lamp(is_player),
            Bar::Grade(b) => b.lamp(is_player),
            Bar::RandomCourse(b) => b.lamp(is_player),
            Bar::SearchWord(b) => b.directory.lamp(is_player),
            Bar::SameFolder(b) => b.directory.lamp(is_player),
            Bar::Executable(b) => b.lamp(is_player),
            Bar::Function(b) => b.lamp(is_player),
            Bar::ContextMenu(b) => b.lamp(is_player),
            Bar::LeaderBoard(b) => b.directory.lamp(is_player),
        }
    }

    pub fn bar_data(&self) -> &BarData {
        match self {
            Bar::Song(b) => &b.selectable.bar_data,
            Bar::Folder(b) => &b.directory.bar_data,
            Bar::Command(b) => &b.directory.bar_data,
            Bar::Container(b) => &b.directory.bar_data,
            Bar::Hash(b) => &b.directory.bar_data,
            Bar::Table(b) => &b.directory.bar_data,
            Bar::Grade(b) => &b.selectable.bar_data,
            Bar::RandomCourse(b) => &b.selectable.bar_data,
            Bar::SearchWord(b) => &b.directory.bar_data,
            Bar::SameFolder(b) => &b.directory.bar_data,
            Bar::Executable(b) => &b.selectable.bar_data,
            Bar::Function(b) => &b.selectable.bar_data,
            Bar::ContextMenu(b) => &b.directory.bar_data,
            Bar::LeaderBoard(b) => &b.directory.bar_data,
        }
    }

    pub fn bar_data_mut(&mut self) -> &mut BarData {
        match self {
            Bar::Song(b) => &mut b.selectable.bar_data,
            Bar::Folder(b) => &mut b.directory.bar_data,
            Bar::Command(b) => &mut b.directory.bar_data,
            Bar::Container(b) => &mut b.directory.bar_data,
            Bar::Hash(b) => &mut b.directory.bar_data,
            Bar::Table(b) => &mut b.directory.bar_data,
            Bar::Grade(b) => &mut b.selectable.bar_data,
            Bar::RandomCourse(b) => &mut b.selectable.bar_data,
            Bar::SearchWord(b) => &mut b.directory.bar_data,
            Bar::SameFolder(b) => &mut b.directory.bar_data,
            Bar::Executable(b) => &mut b.selectable.bar_data,
            Bar::Function(b) => &mut b.selectable.bar_data,
            Bar::ContextMenu(b) => &mut b.directory.bar_data,
            Bar::LeaderBoard(b) => &mut b.directory.bar_data,
        }
    }

    /// Check if this bar is a SongBar
    pub fn as_song_bar(&self) -> Option<&super::song_bar::SongBar> {
        if let Bar::Song(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_song_bar_mut(&mut self) -> Option<&mut super::song_bar::SongBar> {
        if let Bar::Song(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_directory_bar(&self) -> Option<&super::directory_bar::DirectoryBarData> {
        match self {
            Bar::Folder(b) => Some(&b.directory),
            Bar::Command(b) => Some(&b.directory),
            Bar::Container(b) => Some(&b.directory),
            Bar::Hash(b) => Some(&b.directory),
            Bar::Table(b) => Some(&b.directory),
            Bar::SearchWord(b) => Some(&b.directory),
            Bar::SameFolder(b) => Some(&b.directory),
            Bar::ContextMenu(b) => Some(&b.directory),
            Bar::LeaderBoard(b) => Some(&b.directory),
            _ => None,
        }
    }

    pub fn as_grade_bar(&self) -> Option<&super::grade_bar::GradeBar> {
        if let Bar::Grade(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_grade_bar_mut(&mut self) -> Option<&mut super::grade_bar::GradeBar> {
        if let Bar::Grade(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_random_course_bar(&self) -> Option<&super::random_course_bar::RandomCourseBar> {
        if let Bar::RandomCourse(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_executable_bar(&self) -> Option<&super::executable_bar::ExecutableBar> {
        if let Bar::Executable(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_function_bar(&self) -> Option<&super::function_bar::FunctionBar> {
        if let Bar::Function(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_function_bar_mut(&mut self) -> Option<&mut super::function_bar::FunctionBar> {
        if let Bar::Function(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_folder_bar(&self) -> Option<&super::folder_bar::FolderBar> {
        if let Bar::Folder(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_table_bar(&self) -> Option<&super::table_bar::TableBar> {
        if let Bar::Table(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_hash_bar(&self) -> Option<&super::hash_bar::HashBar> {
        if let Bar::Hash(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_context_menu_bar(&self) -> Option<&super::context_menu_bar::ContextMenuBar> {
        if let Bar::ContextMenu(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_search_word_bar(&self) -> Option<&super::search_word_bar::SearchWordBar> {
        if let Bar::SearchWord(b) = self {
            Some(b)
        } else {
            None
        }
    }

    pub fn as_selectable_bar(&self) -> Option<&super::selectable_bar::SelectableBarData> {
        match self {
            Bar::Song(b) => Some(&b.selectable),
            Bar::Grade(b) => Some(&b.selectable),
            Bar::RandomCourse(b) => Some(&b.selectable),
            Bar::Executable(b) => Some(&b.selectable),
            Bar::Function(b) => Some(&b.selectable),
            _ => None,
        }
    }

    pub fn as_selectable_bar_mut(
        &mut self,
    ) -> Option<&mut super::selectable_bar::SelectableBarData> {
        match self {
            Bar::Song(b) => Some(&mut b.selectable),
            Bar::Grade(b) => Some(&mut b.selectable),
            Bar::RandomCourse(b) => Some(&mut b.selectable),
            Bar::Executable(b) => Some(&mut b.selectable),
            Bar::Function(b) => Some(&mut b.selectable),
            _ => None,
        }
    }

    /// Check if this is a DirectoryBar variant
    pub fn is_directory_bar(&self) -> bool {
        matches!(
            self,
            Bar::Folder(_)
                | Bar::Command(_)
                | Bar::Container(_)
                | Bar::Hash(_)
                | Bar::Table(_)
                | Bar::SearchWord(_)
                | Bar::SameFolder(_)
                | Bar::ContextMenu(_)
                | Bar::LeaderBoard(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::select::bar::command_bar::CommandBar;
    use crate::select::bar::container_bar::ContainerBar;
    use crate::select::bar::folder_bar::FolderBar;
    use crate::select::bar::function_bar::FunctionBar;
    use crate::select::bar::grade_bar::GradeBar;
    use crate::select::bar::hash_bar::HashBar;
    use crate::select::bar::random_course_bar::RandomCourseBar;
    use crate::select::bar::same_folder_bar::SameFolderBar;
    use crate::select::bar::search_word_bar::SearchWordBar;
    use crate::select::bar::song_bar::SongBar;
    use crate::select::bar::table_bar::TableBar;
    use crate::select::stubs::*;
    use std::sync::Arc;

    /// Stub TableAccessor for testing
    struct TestTableAccessor;
    impl TableAccessor for TestTableAccessor {
        fn name(&self) -> &str {
            "test"
        }
        fn read(&self) -> Option<TableData> {
            None
        }
        fn write(&self, _td: &mut TableData) {}
    }

    #[test]
    fn bar_clone_song() {
        let bar = Bar::Song(Box::new(SongBar::new(SongData::default())));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_folder() {
        let bar = Bar::Folder(Box::new(FolderBar::new(None, "test".to_string())));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_command() {
        let bar = Bar::Command(Box::new(CommandBar::new(
            "cmd".to_string(),
            "SELECT 1".to_string(),
        )));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_container_with_children() {
        let children = vec![
            Bar::Folder(Box::new(FolderBar::new(None, "a".to_string()))),
            Bar::Folder(Box::new(FolderBar::new(None, "b".to_string()))),
        ];
        let bar = Bar::Container(Box::new(ContainerBar::new(
            "container".to_string(),
            children,
        )));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
        if let Bar::Container(ref c) = cloned {
            assert_eq!(c.children().len(), 2);
        } else {
            panic!("expected Container variant");
        }
    }

    #[test]
    fn bar_clone_hash() {
        let bar = Bar::Hash(Box::new(HashBar::new("hash".to_string(), vec![])));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_table() {
        let td = TableData::default();
        let accessor: Arc<dyn TableAccessor> = Arc::new(TestTableAccessor);
        let bar = Bar::Table(Box::new(TableBar::new(td, accessor)));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_grade() {
        let bar = Bar::Grade(Box::new(GradeBar::new(CourseData::default())));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_random_course() {
        let bar = Bar::RandomCourse(Box::new(RandomCourseBar::new(RandomCourseData::default())));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_search_word() {
        let bar = Bar::SearchWord(Box::new(SearchWordBar::new(
            "search".to_string(),
            "text".to_string(),
        )));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_same_folder() {
        let bar = Bar::SameFolder(Box::new(SameFolderBar::new(
            "same".to_string(),
            "crc".to_string(),
        )));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_function() {
        let bar = Bar::Function(Box::new(FunctionBar::new("func".to_string(), 0)));
        let cloned = bar.clone();
        assert_eq!(bar.title(), cloned.title());
    }

    #[test]
    fn bar_clone_table_shared_accessor() {
        // Verify Arc-based accessor sharing: clone shares the same accessor
        let td = TableData::default();
        let accessor: Arc<dyn TableAccessor> = Arc::new(TestTableAccessor);
        let bar = Bar::Table(Box::new(TableBar::new(td, accessor)));
        let cloned = bar.clone();
        if let (Bar::Table(orig), Bar::Table(cloned_t)) = (&bar, &cloned) {
            assert_eq!(orig.accessor().name(), cloned_t.accessor().name());
        } else {
            panic!("expected Table variants");
        }
    }
}
