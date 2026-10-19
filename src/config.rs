use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Config {
    pub gamepath: Option<String>,
}

fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|p| p.join("MapScraper").join("config.json"))
}

pub fn load_config() -> Config {
    if let Some(path) = get_config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                    return cfg;
                }
            }
        }
    }
    Config::default()
}

pub fn save_config(config: &Config) {
    if let Some(path) = get_config_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(content) = serde_json::to_string_pretty(config) {
            let _ = fs::write(path, content);
        }
    }
}

pub fn clear_config() {
    if let Some(path) = get_config_path() {
        if path.exists() {
            let _ = fs::remove_file(path);
        }
    }
}
