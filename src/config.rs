use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::get_adapter;
use crate::error::SkmError;

pub mod app;

pub use app::{
    app_config_dir, app_config_path, app_store_path, read_app_config, try_read_app_config,
    write_app_config, AppConfig,
};

pub const SETUP_VERSION: u32 = 1;
pub const SETUP_FILENAME: &str = ".skm.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupFile {
    pub version: u32,
    pub placement: PlacementSection,
    #[serde(default)]
    pub profile: ProfileSection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementSection {
    pub agent: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub source_type: String,
    pub path: String,
    pub hash: String,
    pub imported_at: String,
    pub transfer: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileFile {
    /// Names of profiles whose skills this profile also includes, resolved at placement time.
    /// Declared before `skill` so TOML serialization emits it above the `[[skill]]` tables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extends: Vec<String>,
    #[serde(default)]
    pub skill: Vec<ProfileSkillEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSkillEntry {
    pub id: String,
}

pub fn default_setup(agent: &str) -> SetupFile {
    SetupFile {
        version: SETUP_VERSION,
        placement: PlacementSection {
            agent: agent.to_string(),
        },
        profile: ProfileSection::default(),
    }
}

pub fn write_setup(path: &Path, setup: &SetupFile) -> Result<(), SkmError> {
    let content = toml::to_string_pretty(setup)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn read_setup(path: &Path) -> Result<SetupFile, SkmError> {
    let setup = read_setup_raw(path)?;
    validate_setup_agent(&setup)?;
    Ok(setup)
}

pub fn read_setup_raw(path: &Path) -> Result<SetupFile, SkmError> {
    let content = fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            SkmError::SetupNotFound(path.to_path_buf())
        } else {
            SkmError::Io(e)
        }
    })?;
    let setup: SetupFile = toml::from_str(&content).map_err(|e| SkmError::InvalidSetup {
        path: path.to_path_buf(),
        message: e.to_string(),
    })?;
    Ok(setup)
}

pub fn validate_setup_agent(setup: &SetupFile) -> Result<(), SkmError> {
    get_adapter(&setup.placement.agent).map(|_| ())
}

pub fn user_setup_path() -> PathBuf {
    home_dir().join(SETUP_FILENAME)
}

pub fn project_setup_path(cwd: &Path) -> PathBuf {
    cwd.join(SETUP_FILENAME)
}

pub fn home_dir() -> PathBuf {
    directories::UserDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn default_store_root() -> PathBuf {
    home_dir().join(".skill-store")
}

pub fn resolve_store_root(cli_store: Option<&Path>) -> PathBuf {
    if let Some(s) = cli_store {
        return s.to_path_buf();
    }
    if let Ok(env) = std::env::var("SKM_STORE") {
        if !env.is_empty() {
            return PathBuf::from(env);
        }
    }
    if let Some(config) = try_read_app_config() {
        return app_store_path(&config);
    }
    default_store_root()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn cli_store_beats_env() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _env = EnvGuard::set("SKM_STORE", "/from-env");
        let cli = PathBuf::from("/from-cli");
        assert_eq!(resolve_store_root(Some(&cli)), cli);
    }

    #[test]
    fn env_beats_app_config() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let _xdg = EnvGuard::set(
            "XDG_CONFIG_HOME",
            tmp.path().join("config").to_str().unwrap(),
        );
        write_app_config(&tmp.path().join("from-config")).unwrap();
        let _env = EnvGuard::set("SKM_STORE", "/from-env");
        assert_eq!(resolve_store_root(None), PathBuf::from("/from-env"));
    }

    #[test]
    fn app_config_used_when_env_unset() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let store = tmp.path().join("from-config");
        let _xdg = EnvGuard::set(
            "XDG_CONFIG_HOME",
            tmp.path().join("config").to_str().unwrap(),
        );
        let _env = EnvGuard::remove("SKM_STORE");
        write_app_config(&store).unwrap();
        assert_eq!(resolve_store_root(None), store);
    }

    #[test]
    fn empty_skm_store_falls_through() {
        let _lock = ENV_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let store = tmp.path().join("from-config");
        let _xdg = EnvGuard::set(
            "XDG_CONFIG_HOME",
            tmp.path().join("config").to_str().unwrap(),
        );
        let _env = EnvGuard::set("SKM_STORE", "");
        write_app_config(&store).unwrap();
        assert_eq!(resolve_store_root(None), store);
    }
}
