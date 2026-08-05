use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::rules::DomainRule;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to resolve configuration directory")]
    DirResolutionError,
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("serialization error: {0}")]
    Toml(#[from] toml::ser::Error),
    #[error("deserialization error: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub global_params: Vec<String>,
    pub domain_rules: HashMap<String, DomainRule>,
    #[serde(default)]
    pub whitelist_domains: Vec<String>,
    #[serde(default)]
    pub custom_patterns: Vec<String>,
    #[serde(default)]
    pub strip_fragments: bool,
    #[serde(default = "default_true")]
    pub normalize_urls: bool,
    #[serde(default)]
    pub aggressive_mode: bool,
}

fn default_true() -> bool {
    true
}

impl Config {
    pub fn load() -> Result<Arc<Self>, ConfigError> {
        let path = Self::resolve_path()?;
        
        let mut config = if path.exists() {
            let content = fs::read_to_string(&path)?;
            match toml::from_str::<Config>(&content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("config parse error, falling back to default: {}", e);
                    Self::default()
                }
            }
        } else {
            let config = Self::default();
            config.save_atomic(&path).ok();
            config
        };

        config.apply_env_overrides();
        Ok(Arc::new(config))
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::resolve_path()?;
        self.save_atomic(&path)
    }

    fn save_atomic(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        let tmp_path = path.with_extension("tmp");
        
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, path)?;
        
        Ok(())
    }

    fn resolve_path() -> Result<PathBuf, ConfigError> {
        if let Ok(env_path) = env::var("CLIPSCRUB_CONFIG") {
            return Ok(PathBuf::from(env_path));
        }

        if let Some(proj_dirs) = ProjectDirs::from("com", "clipscrub", "clipscrub") {
            Ok(proj_dirs.config_dir().join("config.toml"))
        } else {
            Ok(PathBuf::from("clipscrub.toml"))
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(val) = env::var("CLIPSCRUB_AGGRESSIVE") {
            self.aggressive_mode = val == "1" || val.to_lowercase() == "true";
        }
        
        if let Ok(val) = env::var("CLIPSCRUB_STRIP_FRAGMENTS") {
            self.strip_fragments = val == "1" || val.to_lowercase() == "true";
        }

        if let Ok(val) = env::var("CLIPSCRUB_DEBUG") {
             if val == "1" {
                 println!("Loaded config from: {:?}", Self::resolve_path());
             }
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        use crate::rules::Config as RulesConfig;
        let default_rules = RulesConfig::default();
        
        Self {
            global_params: default_rules.global_params,
            domain_rules: default_rules.domain_rules,
            whitelist_domains: default_rules.whitelist_domains,
            custom_patterns: default_rules.custom_patterns,
            strip_fragments: default_rules.strip_fragments,
            normalize_urls: default_rules.normalize_urls,
            aggressive_mode: default_rules.aggressive_mode,
        }
    }
}
