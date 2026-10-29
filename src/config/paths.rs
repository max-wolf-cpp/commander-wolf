use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StandardPaths {
    pub config: PathBuf,
    pub data: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
    pub runtime: PathBuf,
}

impl StandardPaths {
    pub fn discover() -> Result<Self, String> {
        let environment = std::env::vars_os()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect();

        Self::from_environment(&environment)
    }

    pub fn from_environment(environment: &HashMap<String, String>) -> Result<Self, String> {
        #[cfg(windows)]
        let (config, data, state, cache) = {
            let local = environment
                .get("LOCALAPPDATA")
                .map(PathBuf::from)
                .ok_or("LOCALAPPDATA is not set")?;

            (local.clone(), local.clone(), local.clone(), local)
        };

        #[cfg(not(windows))]
        let (config, data, state, cache) = {
            let home = environment
                .get("HOME")
                .map(PathBuf::from)
                .ok_or("HOME is not set")?;

            let config = environment
                .get("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".config"));
            let data = environment
                .get("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/share"));
            let state = environment
                .get("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/state"));
            let cache = environment
                .get("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".cache"));

            (config, data, state, cache)
        };

        let runtime = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("runtime")))
            .unwrap_or_else(|| PathBuf::from("runtime"));

        Ok(Self {
            config: config.join("commander-wolf"),
            data: data.join("commander-wolf"),
            state: state.join("commander-wolf"),
            cache: cache.join("commander-wolf"),
            runtime,
        })
    }

    pub fn get(&self, name: &str) -> Option<&PathBuf> {
        match name {
            "config" => Some(&self.config),
            "data" => Some(&self.data),
            "state" => Some(&self.state),
            "cache" => Some(&self.cache),
            "runtime" => Some(&self.runtime),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_directory_is_consistent() {
        let mut env = HashMap::new();
        #[cfg(windows)]
        env.insert("LOCALAPPDATA".into(), "C:/Local".into());
        #[cfg(not(windows))]
        env.insert("HOME".into(), "/home/wolf".into());

        let paths = StandardPaths::from_environment(&env).unwrap();

        assert!(paths.config.ends_with("commander-wolf"));
        assert!(paths.cache.ends_with("commander-wolf"));
    }
}
