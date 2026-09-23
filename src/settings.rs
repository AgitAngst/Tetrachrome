//! Настройки программы: общие для семьи Anvil (тема, язык, обновления).
//!
//! Лежат в `settings.json` рядом с файлом пресетов — портативная копия держит их при себе.

use std::path::{Path, PathBuf};

use anvil_ui::CommonSettings;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub common: CommonSettings,
}

pub fn path(presets_store: &Path) -> PathBuf {
    presets_store.with_file_name("settings.json")
}

/// Нет файла или он испорчен — настройки по умолчанию: из-за настроек программа не должна не открыться.
pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default()
}

pub fn save(path: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}
