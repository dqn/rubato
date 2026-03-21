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
    pub date: i64,
    pub max: i32,
    pub adddate: i64,
    #[serde(rename = "type")]
    pub folder_type: i32,
}

impl FolderData {
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn adddate(&self) -> i64 {
        self.adddate
    }
}
