//! Язык интерфейса. Строки в коде — английские, перевод — в `lang/ru.tsv`.
//!
//! Язык общий с набором (`anvil_ui::lang`): [`set`] меняет оба.

use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use anvil_ui::Lang;

static RUSSIAN: AtomicBool = AtomicBool::new(false);

static RU: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| parse(include_str!("../lang/ru.tsv")));

fn parse(text: &'static str) -> HashMap<&'static str, &'static str> {
    text.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).filter_map(|l| l.split_once('\t')).collect()
}

/// Сменить язык программы и набора.
pub fn set(ctx: &eframe::egui::Context, lang: Lang) {
    RUSSIAN.store(lang == Lang::Ru, Ordering::Relaxed);
    anvil_ui::lang::set_language(ctx, lang);
}

fn russian() -> bool {
    RUSSIAN.load(Ordering::Relaxed)
}

/// Перевести строку. Нет перевода — остаётся английская (тест не даёт этому случиться).
pub fn t(en: &'static str) -> &'static str {
    if russian() { RU.get(en).copied().unwrap_or(en) } else { en }
}

/// Подставить значения вместо `{}` по порядку: `fill(t("Deleted preset “{}”"), &[&name])`.
pub fn fill(template: &str, values: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    for value in values {
        let Some(at) = rest.find("{}") else { break };
        out.push_str(&rest[..at]);
        out.push_str(&value.to_string());
        rest = &rest[at + 2..];
    }
    out.push_str(rest);
    out
}

/// Число со словом в нужной форме: «1 image», «3 images» / «1 картинка», «3 картинки», «5 картинок».
pub fn count(n: usize, en: [&str; 2], ru: [&str; 3]) -> String {
    let word = if russian() {
        let (n10, n100) = (n % 10, n % 100);
        if n10 == 1 && n100 != 11 {
            ru[0]
        } else if (2..=4).contains(&n10) && !(12..=14).contains(&n100) {
            ru[1]
        } else {
            ru[2]
        }
    } else if n == 1 {
        en[0]
    } else {
        en[1]
    };
    format!("{n} {word}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Все строки из `t("…")` в исходниках есть в словаре, и в словаре нет лишних.
    #[test]
    fn dictionary_matches_sources() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut used = std::collections::BTreeSet::new();
        let mut stack = vec![dir];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    let text = std::fs::read_to_string(&path).unwrap();
                    // Тесты не в счёт (здесь же ищется сам `t(`), комментарии тоже: там `t("…")` — пример.
                    let text = text.split("#[cfg(test)]").next().unwrap_or_default();
                    let src: String =
                        text.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
                    // `t(` и сразу строка — даже если rustfmt перенёс её на следующую строку.
                    for (i, _) in src.match_indices("t(") {
                        if src[..i].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                            continue;
                        }
                        let Some(body) = src[i + 2..].trim_start().strip_prefix('"') else {
                            continue;
                        };
                        used.insert(body[..body.find('"').unwrap()].to_owned());
                    }
                }
            }
        }
        let missing: Vec<_> = used.iter().filter(|k| !RU.contains_key(k.as_str())).collect();
        assert!(missing.is_empty(), "нет перевода в lang/ru.tsv: {missing:#?}");
        let stale: Vec<_> = RU.keys().filter(|k| !used.contains(**k)).collect();
        assert!(stale.is_empty(), "лишние строки в lang/ru.tsv: {stale:#?}");
        // Подстановки `{}` должны совпадать по числу — иначе значение потеряется.
        for (en, ru) in RU.iter() {
            assert_eq!(en.matches("{}").count(), ru.matches("{}").count(), "разное число {{}}: {en}");
        }
    }

    #[test]
    fn fill_and_plurals() {
        assert_eq!(fill("{} of {} failed", &[&2, &5]), "2 of 5 failed");
        assert_eq!(fill("no values {}", &[]), "no values {}");
        RUSSIAN.store(true, Ordering::Relaxed);
        let f = |n| count(n, ["image", "images"], ["картинка", "картинки", "картинок"]);
        assert_eq!(f(1), "1 картинка");
        assert_eq!(f(3), "3 картинки");
        assert_eq!(f(5), "5 картинок");
        assert_eq!(f(11), "11 картинок");
        assert_eq!(f(22), "22 картинки");
        RUSSIAN.store(false, Ordering::Relaxed);
    }
}
