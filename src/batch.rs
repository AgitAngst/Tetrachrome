//! Пакетный режим: разложить кучу файлов по материалам и упаковать все сразу.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;

use image::{DynamicImage, GenericImageView};

use crate::model::{Channel, OutputFormat, Preset, SizePolicy};
use crate::naming::{self, NameParts};
use crate::pack::{self, SlotInput};
use crate::source;

/// Материал: карты с общим именем.
#[derive(Debug, Clone)]
pub struct Group {
    pub base: String,
    pub files: [Option<PathBuf>; 4],
}

/// Файлы, разложенные по пресету.
#[derive(Debug, Default, Clone)]
pub struct Plan {
    pub groups: Vec<Group>,
    /// Файлы, для которых не нашлось канала.
    pub unmatched: Vec<PathBuf>,
    /// Второй файл на тот же канал того же материала.
    pub conflicts: Vec<(PathBuf, String)>,
}

/// Разложить файлы по материалам. Порядок материалов — по имени.
pub fn plan(files: &[PathBuf], preset: &Preset) -> Plan {
    let mut plan = Plan::default();
    let mut index: HashMap<String, usize> = HashMap::new();
    for path in files {
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let Some(m) = naming::match_suffix(&stem, preset) else {
            plan.unmatched.push(path.clone());
            continue;
        };
        let base = if m.base.is_empty() { stem.clone() } else { m.base };
        let key = base.to_lowercase();
        let at = *index.entry(key).or_insert_with(|| {
            plan.groups.push(Group {
                base: base.clone(),
                files: Default::default(),
            });
            plan.groups.len() - 1
        });
        let slot = &mut plan.groups[at].files[m.channel.index()];
        if slot.is_some() {
            plan.conflicts.push((path.clone(), format!("{} {}", base, m.channel.letter())));
        } else {
            *slot = Some(path.clone());
        }
    }
    plan.groups.sort_by_key(|g| g.base.to_lowercase());
    plan
}

/// Все картинки из списка путей; папки раскрываются на один уровень вглубь
/// и глубже — рекурсивно.
pub fn expand(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        collect(&path, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                collect(&entry.path(), out);
            }
        }
    } else if source::is_image(path) {
        out.push(path.to_path_buf());
    }
}

/// Всё, что нужно фоновому потоку.
pub struct Job {
    pub groups: Vec<Group>,
    pub preset: Preset,
    pub template: String,
    pub format: OutputFormat,
    pub sixteen: bool,
    pub policy: SizePolicy,
    pub custom: (u32, u32),
    pub out_dir: PathBuf,
    pub overwrite: bool,
}

pub enum Event {
    Log(LogLevel, String),
    /// Материал обработан: номер и удалось ли.
    Done(usize, bool),
    Finished,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Ok,
    Warn,
    Error,
}

/// Общий прогресс: сколько материалов готово и не нажата ли «Стоп».
#[derive(Default)]
pub struct Progress {
    pub done: AtomicUsize,
    pub cancel: AtomicBool,
}

pub fn run(job: Job, progress: Arc<Progress>, events: Sender<Event>, repaint: impl Fn()) {
    let log = |level, text: String| {
        let _ = events.send(Event::Log(level, text));
        repaint();
    };
    if let Err(e) = std::fs::create_dir_all(&job.out_dir) {
        log(LogLevel::Error, format!("Cannot create {}: {e}", job.out_dir.display()));
        let _ = events.send(Event::Finished);
        repaint();
        return;
    }
    for (i, group) in job.groups.iter().enumerate() {
        if progress.cancel.load(Ordering::Relaxed) {
            log(LogLevel::Warn, "Cancelled".to_owned());
            break;
        }
        let ok = match process(&job, group) {
            Ok(Outcome::Written(path, (w, h))) => {
                log(LogLevel::Ok, format!("{} → {} ({w}×{h})", group.base, source::display_name(&path)));
                true
            }
            Ok(Outcome::Skipped(path)) => {
                log(LogLevel::Warn, format!("{}: {} exists, skipped", group.base, source::display_name(&path)));
                true
            }
            Err(e) => {
                log(LogLevel::Error, format!("{}: {e}", group.base));
                false
            }
        };
        progress.done.fetch_add(1, Ordering::Relaxed);
        let _ = events.send(Event::Done(i, ok));
        repaint();
    }
    let _ = events.send(Event::Finished);
    repaint();
}

enum Outcome {
    Written(PathBuf, (u32, u32)),
    Skipped(PathBuf),
}

fn process(job: &Job, group: &Group) -> Result<Outcome, String> {
    let name = naming::render(
        &job.template,
        &NameParts {
            basename: &group.base,
            preset: &job.preset.name,
            label: &job.preset.label,
        },
    );
    let path = job.out_dir.join(format!("{name}.{}", job.format.extension()));
    if path.exists() && !job.overwrite {
        return Ok(Outcome::Skipped(path));
    }
    // Одна и та же картинка может идти в два канала — читаем её один раз.
    let mut loaded: HashMap<&Path, DynamicImage> = HashMap::new();
    for file in group.files.iter().flatten() {
        if !loaded.contains_key(file.as_path()) {
            loaded.insert(file, source::decode(file)?);
        }
    }
    let sizes: Vec<(u32, u32)> = loaded.values().map(GenericImageView::dimensions).collect();
    let size = pack::target_size(&sizes, job.policy, job.custom).ok_or("no source images")?;
    let slots: [SlotInput; 4] = std::array::from_fn(|i| {
        let config = &job.preset.slots[i];
        SlotInput {
            image: group.files[i].as_ref().and_then(|f| loaded.get(f.as_path())),
            source: config.source,
            srgb: config.srgb,
            invert: config.invert,
            fill: config.fill,
        }
    });
    let image = pack::pack(&slots, job.preset.alpha, size, job.sixteen && job.format.supports_16bit());
    image
        .save_with_format(&path, job.format.image_format())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(Outcome::Written(path, size))
}

/// Сколько каналов материала заполнено — для таблицы.
pub fn filled(group: &Group, preset: &Preset) -> usize {
    preset.channels().iter().filter(|c: &&Channel| group.files[c.index()].is_some()).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presets::builtin;

    #[test]
    fn groups_by_base_name() {
        let preset = builtin().into_iter().find(|p| p.label == "_MaskMap").unwrap();
        let files: Vec<PathBuf> = [
            "a/Rock_Metallic.png",
            "a/Rock_AO.png",
            "a/Rock_Smoothness.png",
            "a/rock_ao.tga",
            "a/Wood_Metal.png",
            "a/Wood_Albedo.png",
        ]
        .iter()
        .map(PathBuf::from)
        .collect();
        let plan = plan(&files, &preset);
        assert_eq!(plan.groups.len(), 2);
        assert_eq!(plan.groups[0].base, "Rock");
        assert!(plan.groups[0].files[0].is_some());
        assert!(plan.groups[0].files[1].is_some());
        assert!(plan.groups[0].files[2].is_none());
        assert!(plan.groups[0].files[3].is_some());
        assert_eq!(plan.groups[1].base, "Wood");
        assert_eq!(plan.unmatched, vec![PathBuf::from("a/Wood_Albedo.png")]);
        assert_eq!(plan.conflicts.len(), 1);
    }
}
