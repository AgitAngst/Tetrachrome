//! Встроенные пресеты и хранилище пользовательских.
//!
//! Пользовательские пресеты лежат в `presets.json`. Если такой файл есть рядом
//! с exe — работаем с ним (переносная установка на флешке или в репозитории
//! проекта). Иначе — `%APPDATA%\Tetrachrome\presets.json`.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::{Engine, Preset, SlotConfig};

const FILE_NAME: &str = "presets.json";

const METALLIC: &[&str] = &["_Metallic", "_Metalness", "_Metal", "_MTL"];
const SMOOTHNESS: &[&str] = &["_Smoothness", "_Smooth", "_Glossiness", "_Gloss"];
const ROUGHNESS: &[&str] = &["_Roughness", "_Rough", "_RGH"];
const OCCLUSION: &[&str] = &["_AmbientOcclusion", "_Occlusion", "_AO", "_Occ"];
const DETAIL: &[&str] = &["_DetailMask", "_Detail"];

/// Карта, из которой роль получается инверсией: гладкость — это перевёрнутая
/// шероховатость и наоборот. Нужна при переходе между Unity и Unreal/Godot.
pub fn inverse_source(role: &str) -> Option<&'static [&'static str]> {
    match role.trim().to_lowercase().as_str() {
        "smoothness" | "glossiness" => Some(ROUGHNESS),
        "roughness" => Some(SMOOTHNESS),
        _ => None,
    }
}

fn slot(role: &str, suffixes: &[&str], fill: u8) -> SlotConfig {
    SlotConfig {
        fill,
        ..SlotConfig::role(role, suffixes)
    }
}

pub fn builtin() -> Vec<Preset> {
    let preset = |name: &str, engine, label: &str, alpha, slots| Preset {
        name: name.to_owned(),
        engine,
        label: label.to_owned(),
        alpha,
        slots,
        builtin: true,
    };
    vec![
        preset(
            "Unity URP — Metallic",
            Engine::Unity,
            "_MetallicSmoothness",
            true,
            [
                slot("Metallic", METALLIC, 0),
                SlotConfig::empty(0),
                SlotConfig::empty(0),
                slot("Smoothness", SMOOTHNESS, 128),
            ],
        ),
        preset(
            "Unity HDRP — Mask Map",
            Engine::Unity,
            "_MaskMap",
            true,
            [
                slot("Metallic", METALLIC, 0),
                slot("Ambient Occlusion", OCCLUSION, 255),
                slot("Detail Mask", DETAIL, 0),
                slot("Smoothness", SMOOTHNESS, 128),
            ],
        ),
        preset(
            "Unreal — ORM",
            Engine::Unreal,
            "_ORM",
            false,
            [
                SlotConfig::empty(255),
                slot("Roughness", ROUGHNESS, 128),
                slot("Metallic", METALLIC, 0),
                SlotConfig::empty(255),
            ],
        ),
        preset(
            "Unreal — ORM Packed",
            Engine::Unreal,
            "_ORM",
            true,
            [
                slot("Ambient Occlusion", OCCLUSION, 255),
                slot("Roughness", ROUGHNESS, 128),
                slot("Metallic", METALLIC, 0),
                SlotConfig::empty(0),
            ],
        ),
        preset(
            "Godot — ORM",
            Engine::Godot,
            "_ORM",
            false,
            [
                slot("Ambient Occlusion", OCCLUSION, 255),
                slot("Roughness", ROUGHNESS, 128),
                slot("Metallic", METALLIC, 0),
                SlotConfig::empty(255),
            ],
        ),
        preset(
            "Custom",
            Engine::Other,
            "_Packed",
            true,
            [
                SlotConfig::empty(0),
                SlotConfig::empty(0),
                SlotConfig::empty(0),
                SlotConfig::empty(255),
            ],
        ),
    ]
}

#[derive(Serialize, Deserialize)]
struct PresetFile {
    version: u32,
    presets: Vec<Preset>,
}

/// Где лежит файл пресетов.
pub fn store_path() -> PathBuf {
    if let Some(dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) {
        let portable = dir.join(FILE_NAME);
        if portable.exists() {
            return portable;
        }
    }
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Tetrachrome").join(FILE_NAME)
}

/// Пользовательские пресеты. Файла нет — пустой список; файл испорчен — ошибка,
/// чтобы не затереть его при следующем сохранении.
pub fn load(path: &Path) -> Result<Vec<Preset>, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
    };
    let file: PresetFile =
        serde_json::from_str(&text).map_err(|e| format!("Cannot parse {}: {e}", path.display()))?;
    Ok(file.presets)
}

pub fn save(path: &Path, presets: &[Preset]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    }
    let file = PresetFile {
        version: 1,
        presets: presets.iter().filter(|p| !p.builtin).cloned().collect(),
    };
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    // Сначала во временный файл: оборвавшаяся запись не должна съесть пресеты.
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Cannot write {}: {e}", path.display()))
}

/// Имя, которого ещё нет в списке: «Name», «Name 2», «Name 3»…
pub fn unique_name(presets: &[Preset], wanted: &str) -> String {
    let taken = |name: &str| presets.iter().any(|p| p.name.eq_ignore_ascii_case(name));
    if !taken(wanted) {
        return wanted.to_owned();
    }
    (2..)
        .map(|n| format!("{wanted} {n}"))
        .find(|name| !taken(name))
        .expect("infinite range")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_roles_have_suffixes() {
        for preset in builtin() {
            for slot in &preset.slots {
                assert_eq!(slot.role.is_empty(), slot.suffixes.is_empty(), "{}", preset.name);
            }
        }
    }

    #[test]
    fn roundtrip_skips_builtin() {
        let dir = std::env::temp_dir().join(format!("tetrachrome-test-{}", std::process::id()));
        let path = dir.join(FILE_NAME);
        let mut presets = builtin();
        let mut mine = presets[1].clone();
        mine.builtin = false;
        mine.name = "Mine".into();
        presets.push(mine.clone());
        save(&path, &presets).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded, vec![mine]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn unique_names() {
        let presets = builtin();
        assert_eq!(unique_name(&presets, "Custom"), "Custom 2");
        assert_eq!(unique_name(&presets, "Fresh"), "Fresh");
    }
}
