/// Folder data
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FolderData {
    pub title: String,
    pub subtitle: String,
    pub command: String,
    pub path: String,
    pub banner: String,
    pub parent: String,
    pub date: i32,
    pub max: i32,
    pub adddate: i32,
    #[serde(rename = "type")]
    pub folder_type: i32,
}

impl FolderData {
    pub fn get_title(&self) -> &str {
        &self.title
    }

    pub fn get_path(&self) -> &str {
        &self.path
    }

    pub fn get_adddate(&self) -> i32 {
        self.adddate
    }
}
