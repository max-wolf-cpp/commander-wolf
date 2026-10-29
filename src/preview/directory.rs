#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryPreviewEntry {
    pub is_hidden: bool,
    pub name: String,
    pub is_directory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryPreview {
    pub entries: Vec<DirectoryPreviewEntry>,
    pub truncated: bool,
}
