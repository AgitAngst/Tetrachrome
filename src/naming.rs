//! Имена файлов: распознать канал по суффиксу и собрать имя результата.

use crate::model::{Channel, Preset};

/// Совпадение шаблона суффикса с именем файла.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuffixMatch {
    pub channel: Channel,
    /// Имя без найденного суффикса: общее у всех карт одного материала.
    pub base: String,
    len: usize,
}

/// Найти канал пресета, к которому относится файл с таким именем (без
/// расширения). Шаблон ищется без учёта регистра и должен кончаться на границе
/// слова: `_M` найдётся в `Rock_M` и `Rock_M_2K`, но не в `Rock_Mask`. Если
/// подходят несколько, побеждает самый длинный.
pub fn match_suffix(stem: &str, preset: &Preset) -> Option<SuffixMatch> {
    let mut best: Option<SuffixMatch> = None;
    for &channel in preset.channels() {
        let Some((len, base)) = match_patterns(stem, &preset.slots[channel.index()].suffixes) else {
            continue;
        };
        if best.as_ref().is_none_or(|b| len > b.len) {
            best = Some(SuffixMatch { channel, base, len });
        }
    }
    best
}

/// Самый длинный из шаблонов, найденный в имени: его длина и имя без него.
pub fn match_patterns<S: AsRef<str>>(stem: &str, patterns: &[S]) -> Option<(usize, String)> {
    let lower = stem.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for pattern in patterns {
        let pattern = pattern.as_ref().trim().trim_matches('*').to_lowercase();
        if pattern.is_empty() || best.as_ref().is_some_and(|b| b.0 >= pattern.len()) {
            continue;
        }
        let Some(at) = find_token(&lower, &pattern) else {
            continue;
        };
        // `to_lowercase` может поменять длину не-ASCII символов; тогда
        // безопаснее не резать имя вовсе.
        let base = if lower.len() == stem.len() {
            let mut base = String::with_capacity(stem.len());
            base.push_str(&stem[..at]);
            base.push_str(&stem[at + pattern.len()..]);
            tidy(&base)
        } else {
            stem.to_owned()
        };
        best = Some((pattern.len(), base));
    }
    best
}

/// Последнее вхождение `pattern`, за которым конец имени или не буква/цифра.
fn find_token(haystack: &str, pattern: &str) -> Option<usize> {
    haystack.rmatch_indices(pattern).map(|(i, _)| i).find(|&i| {
        haystack[i + pattern.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric())
    })
}

/// Убрать повисшие разделители: `Rock__2K` → `Rock_2K`, `Rock_` → `Rock`.
fn tidy(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        let sep = matches!(c, '_' | '-' | ' ' | '.');
        if sep && out.chars().last().is_some_and(|l| matches!(l, '_' | '-' | ' ' | '.')) {
            continue;
        }
        out.push(c);
    }
    out.trim_matches(|c| matches!(c, '_' | '-' | ' ' | '.')).to_owned()
}

/// Общее имя материала по имени одной из его карт.
pub fn base_name(stem: &str, preset: &Preset) -> String {
    match match_suffix(stem, preset) {
        Some(m) if !m.base.is_empty() => m.base,
        _ => stem.to_owned(),
    }
}

/// Значения для подстановки в шаблон имени.
pub struct NameParts<'a> {
    pub basename: &'a str,
    pub preset: &'a str,
    pub label: &'a str,
}

pub const TOKENS: &[(&str, &str)] = &[
    ("{basename}", "Source name without the map suffix"),
    ("{label}", "Preset output label, e.g. _MaskMap"),
    ("{preset}", "Preset name"),
    ("{date}", "Today, YYYY-MM-DD"),
];

/// Имя файла по шаблону, без расширения. Недопустимые в Windows символы
/// заменяются на `_`.
pub fn render(template: &str, parts: &NameParts) -> String {
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let name = template
        .replace("{basename}", parts.basename)
        .replace("{label}", parts.label)
        .replace("{preset}", &slug(parts.preset))
        .replace("{date}", &date);
    let clean: String = name
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { '_' } else { c })
        .collect();
    let clean = clean.trim().trim_end_matches('.').to_owned();
    if clean.is_empty() { "packed".to_owned() } else { clean }
}

/// Имя пресета для имени файла: `Unity HDRP — Mask Map` → `Unity_HDRP_Mask_Map`.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_alphanumeric() || c == '-' {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::builtin;

    fn hdrp() -> Preset {
        builtin().into_iter().find(|p| p.label == "_MaskMap").unwrap()
    }

    #[test]
    fn matches_by_suffix() {
        let p = hdrp();
        let m = match_suffix("Rock_01_Metallic", &p).unwrap();
        assert_eq!((m.channel, m.base.as_str()), (Channel::R, "Rock_01"));
        let m = match_suffix("rock_ao_2K", &p).unwrap();
        assert_eq!((m.channel, m.base.as_str()), (Channel::G, "rock_2K"));
        let m = match_suffix("Rock_AmbientOcclusion", &p).unwrap();
        assert_eq!(m.channel, Channel::G);
        assert_eq!(match_suffix("Rock_Albedo", &p), None);
        // `_Metal` не должен найтись внутри `_Metallic_Old` как граница слова.
        let m = match_suffix("Rock_Metallic_Old", &p).unwrap();
        assert_eq!(m.base, "Rock_Old");
    }

    #[test]
    fn word_boundary() {
        let mut p = hdrp();
        p.slots[0].suffixes = vec!["*_M*".into()];
        assert_eq!(match_suffix("Rock_Mask", &p).map(|m| m.channel), None);
        assert_eq!(match_suffix("Rock_M", &p).map(|m| m.channel), Some(Channel::R));
    }

    #[test]
    fn channels_without_alpha_are_skipped() {
        let orm = builtin().into_iter().find(|p| p.name == "Godot — ORM").unwrap();
        assert_eq!(match_suffix("Rock_Roughness", &orm).map(|m| m.channel), Some(Channel::G));
    }

    #[test]
    fn renders_template() {
        let parts = NameParts {
            basename: "Rock",
            preset: "Unity HDRP — Mask Map",
            label: "_MaskMap",
        };
        assert_eq!(render("{basename}{label}", &parts), "Rock_MaskMap");
        assert_eq!(render("{preset}/{basename}", &parts), "Unity_HDRP_Mask_Map_Rock");
        assert!(render("{basename}_{date}", &parts).starts_with("Rock_20"));
        assert_eq!(render("   ", &parts), "packed");
    }
}
