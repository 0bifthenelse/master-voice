use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub language: Option<String>,
    pub rate: Option<f32>,
    pub pitch: Option<f32>,
    pub volume: Option<f32>,
    pub robotic_depth: Option<f32>,
    pub device: Option<String>,
    pub queue_limit: Option<usize>,
    pub daemon_idle_timeout_secs: Option<u64>,
    pub overrides: Option<toml::Table>,
    pub omp_auto_speech: Option<bool>,
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn config_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return PathBuf::from(dir);
    }
    home_dir().join(".config")
}

pub fn cache_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(dir);
    }
    home_dir().join(".cache")
}

pub fn data_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(dir);
    }
    home_dir().join(".local").join("share")
}

pub fn config_path() -> PathBuf {
    config_dir().join("master-voice").join("config.toml")
}

pub fn runtime_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            return path;
        }
    }
    std::env::temp_dir()
}

pub fn socket_path() -> PathBuf {
    runtime_dir().join("master-voice.sock")
}

pub fn load_config() -> Result<Config, String> {
    let path = config_path();
    if !path.exists() {
        return Ok(Config::default());
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    toml::from_str(&content).map_err(|e| format!("invalid config {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_config_is_default() {
        assert!(load_config().is_ok());
    }

    #[test]
    fn parses_known_fields() {
        let config: Config = toml::from_str(
            r#"
language = "fr-FR"
rate = 1.2
queue_limit = 8
omp_auto_speech = false

[overrides]
linux = "L IH N UX K S"
"#,
        )
        .unwrap();
        assert_eq!(config.language.as_deref(), Some("fr-FR"));
        assert_eq!(config.rate, Some(1.2));
        assert_eq!(config.queue_limit, Some(8));
        assert_eq!(config.omp_auto_speech, Some(false));
        assert!(config.overrides.is_some());
    }

    #[test]
    fn rejects_garbage() {
        assert!(toml::from_str::<Config>("not toml {{{").is_err());
    }
}
