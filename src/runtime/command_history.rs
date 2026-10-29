use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use crate::commands::history::{CommandHistory, MAX_COMMAND_BYTES, MAX_HISTORY_ENTRIES};

const HEADER: &[u8; 8] = b"CWHIST1\n";
const MAX_BYTES: usize = HEADER.len() + MAX_HISTORY_ENTRIES * (4 + MAX_COMMAND_BYTES);

pub(super) struct HistoryStore {
    path: PathBuf,
    enabled: bool,
}

impl HistoryStore {
    pub fn new(state_directory: PathBuf, enabled: bool) -> Self {
        Self {
            path: state_directory.join("command_history.dat"),
            enabled,
        }
    }

    pub fn load(&self) -> Result<CommandHistory, String> {
        if !self.enabled {
            return Ok(CommandHistory::default());
        }

        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CommandHistory::default());
            }
            Err(error) => {
                return Err(format!(
                    "Cannot read command history '{}': {error}",
                    self.path.display()
                ));
            }
        };

        let mut bytes = Vec::new();
        file.take(MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                format!(
                    "Cannot read command history '{}': {error}",
                    self.path.display()
                )
            })?;

        decode(&bytes)
            .map_err(|error| format!("Invalid command history '{}': {error}", self.path.display()))
    }

    pub fn save(&self, history: &CommandHistory) -> Result<(), String> {
        if !self.enabled || history.entries().is_empty() {
            return Ok(());
        }

        let mut bytes = HEADER.to_vec();
        for line in history.entries() {
            bytes.extend_from_slice(&(line.len() as u32).to_le_bytes());
            bytes.extend_from_slice(line.as_bytes());
        }

        super::state_file::write(&self.path, &bytes).map_err(|error| {
            format!(
                "Cannot save command history '{}': {error}",
                self.path.display()
            )
        })
    }
}

fn decode(bytes: &[u8]) -> Result<CommandHistory, String> {
    if bytes.len() > MAX_BYTES || !bytes.starts_with(HEADER) {
        return Err("unsupported format or oversized state".into());
    }

    let mut history = CommandHistory::default();
    let mut bytes = &bytes[HEADER.len()..];
    let mut count = 0;
    while !bytes.is_empty() {
        count += 1;
        if count > MAX_HISTORY_ENTRIES {
            return Err("too many entries".into());
        }

        let length = bytes.get(..4).ok_or("missing command length")?;
        let length = u32::from_le_bytes(length.try_into().unwrap()) as usize;
        if length > MAX_COMMAND_BYTES {
            return Err("command exceeds size limit".into());
        }

        bytes = &bytes[4..];
        let line = std::str::from_utf8(bytes.get(..length).ok_or("truncated command")?)
            .map_err(|_| "command is not UTF-8")?;
        if !line.starts_with(':') {
            return Err("missing command prefix".into());
        }

        history.record(line);
        bytes = &bytes[length..];
    }

    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "wolf-history-{}-{stamp}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn history_survives_restarts_without_saving_browsing_or_draft_state() {
        let root = TestDirectory::new();
        let store = HistoryStore::new(root.0.join("State"), true);
        let mut history = store.load().unwrap();
        store.save(&history).unwrap();
        assert!(!store.path.exists());

        history.record(":cd \"Русский 狼\"");
        history.record(":style compact");
        history.previous(":unsent draft");
        store.save(&history).unwrap();

        let mut loaded = HistoryStore::new(root.0.join("State"), true)
            .load()
            .unwrap();
        assert_eq!(loaded.entries(), history.entries());
        assert_eq!(loaded.next_entry(), None);
        assert_eq!(loaded.previous(":new draft"), Some(":style compact".into()));
        assert_eq!(loaded.next_entry(), Some(":new draft".into()));

        loaded.record(":q");
        store.save(&loaded).unwrap();
        assert_eq!(store.load().unwrap().entries().last().unwrap(), ":q");
        assert_eq!(
            fs::read_dir(store.path.parent().unwrap()).unwrap().count(),
            1
        );
    }

    #[test]
    fn clean_mode_does_not_read_or_overwrite_history() {
        let root = TestDirectory::new();
        let store = HistoryStore::new(root.0.clone(), false);
        fs::write(&store.path, b"corrupt existing history").unwrap();
        let mut history = store.load().unwrap();
        assert!(history.entries().is_empty());
        history.record(":q");
        store.save(&history).unwrap();
        assert_eq!(fs::read(&store.path).unwrap(), b"corrupt existing history");
    }

    #[test]
    fn corrupt_and_oversized_history_is_rejected_without_execution() {
        for data in [
            b"not history".to_vec(),
            vec![0; MAX_BYTES + 1],
            [HEADER.as_slice(), &[255, 255, 255, 255]].concat(),
            [HEADER.as_slice(), &3u32.to_le_bytes(), &[b':', 255, 254]].concat(),
        ] {
            assert!(decode(&data).is_err());
        }

        let root = TestDirectory::new();
        let store = HistoryStore::new(root.0.clone(), true);
        fs::write(&store.path, b"return os.execute('anything')").unwrap();
        assert!(
            store
                .load()
                .unwrap_err()
                .contains("Invalid command history")
        );

        let mut history = CommandHistory::default();
        history.record(":delete");
        store.save(&history).unwrap();
        let bytes = fs::read(&store.path).unwrap();
        for end in HEADER.len() + 1..bytes.len() {
            assert!(decode(&bytes[..end]).is_err());
        }
    }

    #[test]
    fn failed_history_save_preserves_the_existing_target() {
        let root = TestDirectory::new();
        let store = HistoryStore::new(root.0.clone(), true);
        fs::create_dir(&store.path).unwrap();
        fs::write(store.path.join("keep"), b"keep").unwrap();

        let mut history = CommandHistory::default();
        history.record(":q");
        assert!(
            store
                .save(&history)
                .unwrap_err()
                .contains("Cannot save command history")
        );
        assert_eq!(fs::read(store.path.join("keep")).unwrap(), b"keep");
        assert_eq!(fs::read_dir(&root.0).unwrap().count(), 1);
    }
}
