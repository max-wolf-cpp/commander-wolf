use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FinderEntryKind {
    File,
    Directory,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinderResult {
    pub absolute_path: PathBuf,
    pub relative_path: String,
    pub kind: FinderEntryKind,
    pub is_hidden: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinderWorkerMessage {
    ResultsAdded {
        generation: u64,
        results: Vec<FinderResult>,
    },
    Completed {
        generation: u64,
        skipped: usize,
        truncated: bool,
        cancelled: bool,
    },
    Failed {
        generation: u64,
        message: String,
    },
}
