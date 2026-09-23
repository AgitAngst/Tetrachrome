//! Состояние приложения и всё, что происходит вне отрисовки: загрузка
//! картинок, отмена, пресеты, экспорт, пакетная обработка.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, Key, KeyboardShortcut, Modifiers, TextureHandle, TextureOptions};

use crate::batch::{self, LogLevel};
use crate::model::{Channel, OutputFormat, Preset, SizePolicy, SourceChannel};
use crate::naming::{self, NameParts};
use crate::pack::{self, SlotInput};
use crate::presets;
use crate::source::{self, Source};

/// Сколько шагов помнит отмена.
const UNDO_LIMIT: usize = 64;
/// Сколько картинок держим в библиотеке. Старые неиспользуемые выпадают.
const LIBRARY_LIMIT: usize = 32;
const TOAST_TIME: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Pack,
    Batch,
}

/// Что показывать в большом предпросмотре.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Rgba,
    Rgb,
    Channel(Channel),
}

/// Куда пойдёт загруженная картинка.
#[derive(Clone, Copy, Debug)]
pub enum LoadTarget {
    Slot(Channel),
    /// По суффиксу имени. `single` — файл один: без совпадения он займёт
    /// первый пустой канал.
    Auto { single: bool },
    /// Перечитать с диска картинку с этим id.
    Replace(u64),
    /// Только в библиотеку: карта другого материала.
    Library,
}

struct Loaded {
    target: LoadTarget,
    result: Result<Source, String>,
}

/// Снимок для отмены: пресет, его рабочая копия и картинки в каналах.
#[derive(Clone)]
pub struct Snapshot {
    selected: usize,
    work: Preset,
    slots: [Option<Arc<Source>>; 4],
}

impl Snapshot {
    fn same(&self, other: &Snapshot) -> bool {
        self.selected == other.selected
            && self.work.same_config(&other.work)
            && self.work.alpha == other.work.alpha
            && ids(&self.slots) == ids(&other.slots)
    }
}

fn ids(slots: &[Option<Arc<Source>>; 4]) -> [Option<u64>; 4] {
    std::array::from_fn(|i| slots[i].as_ref().map(|s| s.id))
}

/// По чему видно, что предпросмотр устарел.
#[derive(Clone, PartialEq)]
struct PreviewKey {
    ids: [Option<u64>; 4],
    configs: [(SourceChannel, bool, bool, u8); 4],
    alpha: bool,
    target: Option<(u32, u32)>,
}

#[derive(Default)]
pub struct Preview {
    key: Option<PreviewKey>,
    /// Размер плоскостей предпросмотра.
    pub size: (u32, u32),
    planes: [Vec<f32>; 4],
    pub main: Option<TextureHandle>,
    main_mode: Option<ViewMode>,
    pub strips: [Option<TextureHandle>; 4],
}

pub struct View {
    /// Точек экрана на пиксель результата. `None` — вписать в окно.
    pub zoom: Option<f32>,
    pub pan: egui::Vec2,
    pub mode: ViewMode,
    /// Масштаб, которым картинка вписана сейчас: от него считаются +/−.
    pub fit_zoom: f32,
}

pub struct OutputSettings {
    pub template: String,
    pub format: OutputFormat,
    pub sixteen: bool,
    /// Папка результата. `None` — рядом с первым исходником.
    pub folder: Option<PathBuf>,
    pub policy: SizePolicy,
    pub custom: (u32, u32),
    /// Пакетный режим: перезаписывать ли существующие файлы.
    pub overwrite: bool,
}

struct ExportJob {
    rx: Receiver<Result<PathBuf, String>>,
    name: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Toast {
    pub kind: ToastKind,
    pub text: String,
    /// Показать в Проводнике.
    pub reveal: Option<PathBuf>,
    pub born: Instant,
}

pub enum Dialog {
    DeletePreset(usize),
    /// Переход на другой пресет, а в текущем несохранённые правки.
    SwitchPreset(usize),
    Overwrite(PathBuf),
    About,
    Shortcuts,
}

pub struct BatchRun {
    pub progress: Arc<batch::Progress>,
    rx: Receiver<batch::Event>,
    pub total: usize,
    /// Номер в задании → имя материала.
    bases: Vec<String>,
}

#[derive(Default)]
pub struct BatchState {
    pub files: Vec<PathBuf>,
    pub plan: batch::Plan,
    plan_key: Option<(Vec<Vec<String>>, bool, usize)>,
    /// Материалы, снятые галочкой (по имени в нижнем регистре).
    pub disabled: HashSet<String>,
    pub run: Option<BatchRun>,
    pub log: Vec<(LogLevel, String)>,
    /// Итог по материалам последнего запуска.
    pub status: HashMap<String, bool>,
    pub finished_dir: Option<PathBuf>,
}

pub struct App {
    pub presets: Vec<Preset>,
    pub store: PathBuf,
    pub selected: usize,
    /// Рабочая копия выбранного пресета: всё, что меняется в интерфейсе.
    pub work: Preset,
    pub slots: [Option<Arc<Source>>; 4],
    /// Все загруженные картинки, свежие первыми.
    pub library: Vec<Arc<Source>>,
    thumbs: HashMap<u64, TextureHandle>,

    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    settled: Snapshot,

    loads: Vec<Receiver<Loaded>>,
    pub loading: usize,

    pub mode: Mode,
    pub preview: Preview,
    pub view: View,
    pub output: OutputSettings,
    export: Option<ExportJob>,
    pub last_export: Option<PathBuf>,
    pub batch: BatchState,

    pub toasts: Vec<Toast>,
    pub dialog: Option<Dialog>,
    pub last_open_dir: Option<PathBuf>,
    /// Карточка канала, над которой сейчас несут файлы.
    pub drop_target: Option<Channel>,
    /// Прямоугольники карточек с прошлого кадра — чтобы понять, куда бросили.
    pub card_rects: [Option<egui::Rect>; 4],
    /// Пиксель результата под курсором в предпросмотре.
    pub hovered_pixel: Option<(u32, u32)>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::theme::install(&cc.egui_ctx);

        let store = presets::store_path();
        let mut list = presets::builtin();
        let mut toasts = Vec::new();
        match presets::load(&store) {
            Ok(user) => list.extend(user),
            Err(e) => toasts.push(Toast {
                kind: ToastKind::Error,
                text: e,
                reveal: None,
                born: Instant::now(),
            }),
        }
        // По умолчанию — HDRP Mask Map: самый полный из встроенных.
        let selected = list.iter().position(|p| p.label == "_MaskMap").unwrap_or(0);
        let work = list[selected].clone();
        let settled = Snapshot {
            selected,
            work: work.clone(),
            slots: Default::default(),
        };
        Self {
            presets: list,
            store,
            selected,
            work,
            slots: Default::default(),
            library: Vec::new(),
            thumbs: HashMap::new(),
            undo: Vec::new(),
            redo: Vec::new(),
            settled,
            loads: Vec::new(),
            loading: 0,
            mode: Mode::Pack,
            preview: Preview::default(),
            view: View {
                zoom: None,
                pan: egui::Vec2::ZERO,
                mode: ViewMode::Rgba,
                fit_zoom: 1.0,
            },
            output: OutputSettings {
                template: "{basename}{label}".to_owned(),
                format: OutputFormat::Png,
                sixteen: false,
                folder: None,
                policy: SizePolicy::Largest,
                custom: (2048, 2048),
                overwrite: false,
            },
            export: None,
            last_export: None,
            batch: BatchState::default(),
            toasts,
            dialog: None,
            last_open_dir: None,
            drop_target: None,
            card_rects: [None; 4],
            hovered_pixel: None,
        }
    }

    // ---------------------------------------------------------------- пресеты

    pub fn current(&self) -> &Preset {
        &self.presets[self.selected]
    }

    /// Рабочая копия разошлась с сохранённым пресетом.
    pub fn modified(&self) -> bool {
        !self.work.same_config(self.current())
    }

    pub fn select_preset(&mut self, index: usize) {
        if index == self.selected || index >= self.presets.len() {
            return;
        }
        if self.modified() {
            self.dialog = Some(Dialog::SwitchPreset(index));
        } else {
            self.apply_preset(index);
        }
    }

    /// Перейти на пресет. Картинки переезжают вслед за ролями: Metallic из
    /// канала R в URP окажется в канале B в Unreal ORM.
    pub fn apply_preset(&mut self, index: usize) {
        let old_work = std::mem::replace(&mut self.work, self.presets[index].clone());
        let old_slots = std::mem::take(&mut self.slots);
        self.selected = index;
        let base = basename_of(&old_slots, &old_work);
        let mut notes = Vec::new();
        for &channel in self.work.clone().channels() {
            let i = channel.index();
            let role = self.work.slots[i].role.trim().to_lowercase();
            if role.is_empty() {
                continue;
            }
            // 1. Та же роль была в прошлом пресете.
            let carried = old_work
                .slots
                .iter()
                .zip(&old_slots)
                .find(|(config, image)| image.is_some() && config.role.trim().to_lowercase() == role)
                .and_then(|(_, image)| image.clone());
            if carried.is_some() {
                self.slots[i] = carried;
                continue;
            }
            let Some(base) = &base else { continue };
            // 2. В библиотеке есть карта того же материала с нужным суффиксом.
            let suffixes = self.work.slots[i].suffixes.clone();
            if let Some(found) = self.find_in_library(base, &suffixes) {
                self.slots[i] = Some(found);
                continue;
            }
            // 3. Роль получается инверсией другой карты: гладкость из шероховатости.
            if let Some(opposite) = presets::inverse_source(&role) {
                let found = self.find_in_library(base, opposite).or_else(|| {
                    old_slots.iter().flatten().find(|s| naming::match_patterns(&s.stem(), opposite).is_some()).cloned()
                });
                if let Some(found) = found {
                    notes.push(format!("{} ← inverted {}", self.work.slots[i].role, found.name));
                    self.slots[i] = Some(found);
                    self.work.slots[i].invert = !self.work.slots[i].invert;
                }
            }
        }
        for note in notes {
            self.toast(ToastKind::Info, note);
        }
    }

    fn find_in_library<S: AsRef<str>>(&self, base: &str, suffixes: &[S]) -> Option<Arc<Source>> {
        self.library
            .iter()
            .find(|s| {
                naming::match_patterns(&s.stem(), suffixes)
                    .is_some_and(|(_, b)| b.eq_ignore_ascii_case(base))
            })
            .cloned()
    }

    /// Сохранить рабочую копию. Встроенный пресет не трогаем — появляется новый.
    pub fn save_preset(&mut self) {
        if self.current().builtin {
            self.save_preset_as_new();
            return;
        }
        let mut preset = self.work.clone();
        preset.builtin = false;
        let taken = self
            .presets
            .iter()
            .enumerate()
            .any(|(i, p)| i != self.selected && p.name.eq_ignore_ascii_case(&preset.name));
        if taken || preset.name.trim().is_empty() {
            self.toast(ToastKind::Warning, "A preset with this name already exists".to_owned());
            return;
        }
        self.presets[self.selected] = preset;
        self.persist();
        self.toast(ToastKind::Success, format!("Saved preset “{}”", self.work.name));
    }

    pub fn save_preset_as_new(&mut self) {
        let mut preset = self.work.clone();
        preset.builtin = false;
        let wanted = if preset.name == self.current().name {
            format!("{} copy", preset.name)
        } else {
            preset.name.clone()
        };
        preset.name = presets::unique_name(&self.presets, wanted.trim());
        self.work.name = preset.name.clone();
        self.presets.push(preset);
        self.selected = self.presets.len() - 1;
        self.persist();
        self.toast(ToastKind::Success, format!("Created preset “{}”", self.work.name));
    }

    pub fn revert_preset(&mut self) {
        self.work = self.current().clone();
    }

    pub fn new_preset(&mut self) {
        let preset = Preset {
            name: presets::unique_name(&self.presets, "New preset"),
            ..Preset::default()
        };
        self.presets.push(preset);
        self.persist();
        let index = self.presets.len() - 1;
        self.force_select(index);
    }

    /// Дубликат выбранного пресета — таким, каким он виден сейчас.
    pub fn duplicate_preset(&mut self) {
        let mut preset = self.work.clone();
        preset.builtin = false;
        preset.name = presets::unique_name(&self.presets, &format!("{} copy", self.current().name));
        self.presets.push(preset.clone());
        self.selected = self.presets.len() - 1;
        self.work = preset;
        self.persist();
    }

    pub fn delete_preset(&mut self, index: usize) {
        if self.presets.get(index).is_none_or(|p| p.builtin) {
            return;
        }
        let name = self.presets.remove(index).name;
        self.persist();
        if self.selected == index {
            let next = index.min(self.presets.len() - 1);
            self.selected = usize::MAX;
            self.apply_preset(next);
        } else if self.selected > index {
            self.selected -= 1;
        }
        self.toast(ToastKind::Info, format!("Deleted preset “{name}”"));
    }

    /// Сдвинуть пользовательский пресет вверх или вниз среди пользовательских.
    pub fn move_preset(&mut self, index: usize, up: bool) {
        let target = if up { index.checked_sub(1) } else { Some(index + 1) };
        let Some(target) = target else { return };
        if target >= self.presets.len() || self.presets[target].builtin || self.presets[index].builtin {
            return;
        }
        self.presets.swap(index, target);
        if self.selected == index {
            self.selected = target;
        } else if self.selected == target {
            self.selected = index;
        }
        self.persist();
    }

    /// Выбрать пресет, отбросив правки.
    pub fn force_select(&mut self, index: usize) {
        self.selected = usize::MAX;
        self.apply_preset(index);
    }

    fn persist(&mut self) {
        if let Err(e) = presets::save(&self.store, &self.presets) {
            self.toast(ToastKind::Error, e);
        }
    }

    // ---------------------------------------------------------------- картинки

    /// Загрузить файлы в фоне. Уже загруженные берутся из библиотеки.
    pub fn open_files(&mut self, paths: Vec<PathBuf>, target: LoadTarget) {
        let paths: Vec<PathBuf> = paths.into_iter().filter(|p| source::is_image(p)).collect();
        if paths.is_empty() {
            self.toast(ToastKind::Warning, "No supported images (PNG, TGA, TIFF, JPG, BMP, EXR)".to_owned());
            return;
        }
        if let Some(dir) = paths[0].parent() {
            self.last_open_dir = Some(dir.to_path_buf());
        }
        let mut fresh = Vec::new();
        for path in paths {
            let known = self.library.iter().find(|s| same_path(&s.path, &path)).cloned();
            match (known, target) {
                (Some(src), LoadTarget::Slot(_) | LoadTarget::Auto { .. }) => self.place(src, target),
                (Some(_), LoadTarget::Library) => {}
                _ => fresh.push(path),
            }
        }
        if fresh.is_empty() {
            return;
        }
        let (tx, rx) = mpsc::channel();
        self.loading += fresh.len();
        self.loads.push(rx);
        std::thread::spawn(move || {
            for path in fresh {
                let result = source::load(&path);
                if tx.send(Loaded { target, result }).is_err() {
                    break;
                }
            }
        });
    }

    /// Файлы и папки — по суффиксам, одиночный файл — в первый пустой канал.
    ///
    /// Если в куче карты нескольких материалов, в каналы идёт один — тот, у
    /// которого карт больше, — а остальные ложатся в библиотеку: смешать Rock
    /// и Wood в одной текстуре никто не хочет.
    pub fn open_paths(&mut self, paths: Vec<PathBuf>) {
        let files = batch::expand(paths);
        if files.len() <= 1 {
            self.open_files(files, LoadTarget::Auto { single: true });
            return;
        }
        let plan = batch::plan(&files, &self.work);
        let best = plan
            .groups
            .iter()
            .enumerate()
            .max_by_key(|(i, g)| (batch::filled(g, &self.work), std::cmp::Reverse(*i)))
            .map(|(_, g)| g.clone());
        let Some(best) = best else {
            self.open_files(files, LoadTarget::Library);
            self.toast(
                ToastKind::Warning,
                "No file names match this preset's suffixes — pick maps from the library on each card".to_owned(),
            );
            return;
        };
        let chosen: Vec<PathBuf> = best.files.iter().flatten().cloned().collect();
        let rest: Vec<PathBuf> = files.into_iter().filter(|f| !chosen.contains(f)).collect();
        let others = plan.groups.len() - 1;
        self.open_files(chosen, LoadTarget::Auto { single: false });
        if !rest.is_empty() {
            self.open_files(rest, LoadTarget::Library);
        }
        if others > 0 {
            self.toast(
                ToastKind::Info,
                format!(
                    "Loaded “{}”. {others} more material{} went to the library — Batch mode packs them all at once.",
                    best.base,
                    if others == 1 { "" } else { "s" }
                ),
            );
        }
    }

    fn poll_loads(&mut self, ctx: &egui::Context) {
        let mut done = Vec::new();
        for rx in &self.loads {
            while let Ok(loaded) = rx.try_recv() {
                done.push(loaded);
            }
        }
        if done.is_empty() {
            if self.loading > 0 {
                ctx.request_repaint_after(Duration::from_millis(100));
            }
            return;
        }
        for Loaded { target, result } in done {
            self.loading = self.loading.saturating_sub(1);
            match result {
                Ok(src) => {
                    let src = Arc::new(src);
                    self.add_to_library(src.clone());
                    self.place(src, target);
                }
                Err(e) => self.toast(ToastKind::Error, e),
            }
        }
        if self.loading == 0 {
            self.loads.clear();
        }
    }

    fn add_to_library(&mut self, src: Arc<Source>) {
        self.library.retain(|s| !same_path(&s.path, &src.path));
        self.library.insert(0, src);
        while self.library.len() > LIBRARY_LIMIT {
            let used = ids(&self.slots);
            match self.library.iter().rposition(|s| !used.contains(&Some(s.id))) {
                Some(i) => {
                    self.library.remove(i);
                }
                None => break,
            }
        }
    }

    fn place(&mut self, src: Arc<Source>, target: LoadTarget) {
        match target {
            LoadTarget::Slot(channel) => {
                if channel == Channel::A && !self.work.alpha {
                    self.work.alpha = true;
                }
                self.slots[channel.index()] = Some(src);
            }
            LoadTarget::Auto { single } => {
                if let Some(m) = naming::match_suffix(&src.stem(), &self.work) {
                    self.slots[m.channel.index()] = Some(src);
                } else if single {
                    let empty = self.work.channels().iter().find(|c| self.slots[c.index()].is_none()).copied();
                    match empty {
                        Some(channel) => self.slots[channel.index()] = Some(src),
                        None => self.toast(
                            ToastKind::Info,
                            format!("{} added to the library — all channels are busy", src.name),
                        ),
                    }
                }
            }
            LoadTarget::Library => {}
            LoadTarget::Replace(old) => {
                for slot in self.slots.iter_mut() {
                    if slot.as_ref().is_some_and(|s| s.id == old) {
                        *slot = Some(src.clone());
                    }
                }
            }
        }
    }

    /// Перечитать с диска все картинки в каналах: художник поправил карту в
    /// Painter и хочет увидеть результат.
    pub fn reload_sources(&mut self) {
        let mut seen = HashSet::new();
        let sources: Vec<Arc<Source>> = self.slots.iter().flatten().filter(|s| seen.insert(s.id)).cloned().collect();
        if sources.is_empty() {
            return;
        }
        for src in sources {
            self.library.retain(|s| s.id != src.id);
            self.open_files(vec![src.path.clone()], LoadTarget::Replace(src.id));
        }
        self.toast(ToastKind::Info, "Reloading sources from disk…".to_owned());
    }

    pub fn clear_slot(&mut self, channel: Channel) {
        self.slots[channel.index()] = None;
    }

    pub fn clear_all(&mut self) {
        self.slots = Default::default();
    }

    /// Миниатюра картинки; текстура создаётся при первом показе.
    pub fn thumb(&mut self, ctx: &egui::Context, src: &Source) -> egui::TextureId {
        self.thumbs
            .entry(src.id)
            .or_insert_with(|| ctx.load_texture(format!("thumb-{}", src.id), src.thumb.clone(), TextureOptions::LINEAR))
            .id()
    }

    // ---------------------------------------------------------------- размер

    /// Размеры картинок в каналах, которые попадут в файл.
    pub fn source_sizes(&self) -> Vec<(u32, u32)> {
        self.work
            .channels()
            .iter()
            .filter_map(|c| self.slots[c.index()].as_ref().map(|s| s.size()))
            .collect()
    }

    pub fn sizes_differ(&self) -> bool {
        let sizes = self.source_sizes();
        sizes.windows(2).any(|w| w[0] != w[1])
    }

    pub fn target_size(&self) -> Option<(u32, u32)> {
        pack::target_size(&self.source_sizes(), self.output.policy, self.output.custom)
    }

    fn slot_inputs<'a>(&'a self, preview: bool) -> [SlotInput<'a>; 4] {
        std::array::from_fn(|i| {
            let config = &self.work.slots[i];
            SlotInput {
                image: self.slots[i].as_ref().map(|s| if preview { &s.preview } else { &s.image }),
                source: config.source,
                srgb: config.srgb,
                invert: config.invert,
                fill: config.fill,
            }
        })
    }

    // ---------------------------------------------------------------- предпросмотр

    fn preview_key(&self) -> PreviewKey {
        PreviewKey {
            ids: ids(&self.slots),
            configs: std::array::from_fn(|i| {
                let c = &self.work.slots[i];
                (c.source, c.srgb, c.invert, c.fill)
            }),
            alpha: self.alpha_visible(),
            target: self.target_size(),
        }
    }

    /// Пересобрать предпросмотр, если что-то поменялось.
    fn refresh_preview(&mut self, ctx: &egui::Context) {
        let key = self.preview_key();
        if self.preview.key.as_ref() != Some(&key) {
            let size = pack::preview_size(key.target.unwrap_or((512, 512)));
            let planes = pack::compose(&self.slot_inputs(true), size);
            for (i, plane) in planes.iter().enumerate() {
                let image = ColorImage::from_gray([size.0 as usize, size.1 as usize], &to_bytes(plane));
                set_texture(ctx, &mut self.preview.strips[i], &format!("strip-{i}"), image);
            }
            self.preview.planes = planes;
            self.preview.size = size;
            self.preview.key = Some(key);
            self.preview.main_mode = None;
        }
        if self.preview.main_mode != Some(self.view.mode) {
            let image = self.main_image();
            set_texture(ctx, &mut self.preview.main, "preview", image);
            self.preview.main_mode = Some(self.view.mode);
        }
    }

    fn main_image(&self) -> ColorImage {
        let (w, h) = self.preview.size;
        let planes = &self.preview.planes;
        let n = w as usize * h as usize;
        let mut rgba = vec![255u8; n * 4];
        // Альфа без картинки — постоянное число; прозрачная целиком картинка
        // ничего не показала бы, поэтому такую альфу в предпросмотре не
        // применяем (в файл она, конечно, пишется).
        let alpha = self.alpha_visible();
        match self.view.mode {
            ViewMode::Rgba | ViewMode::Rgb => {
                for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
                    px[0] = pack::to_u8(planes[0][i]);
                    px[1] = pack::to_u8(planes[1][i]);
                    px[2] = pack::to_u8(planes[2][i]);
                    if alpha && self.view.mode == ViewMode::Rgba {
                        px[3] = pack::to_u8(planes[3][i]);
                    }
                }
            }
            ViewMode::Channel(c) => {
                for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
                    let v = pack::to_u8(planes[c.index()][i]);
                    px[0] = v;
                    px[1] = v;
                    px[2] = v;
                }
            }
        }
        ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba)
    }

    /// Показывать ли альфу в режиме RGBA.
    pub fn alpha_visible(&self) -> bool {
        self.work.alpha && self.slots[Channel::A.index()].is_some()
    }

    /// Точные значения пикселя результата — из полноразмерных исходников.
    pub fn pixel_value(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        let target = self.target_size()?;
        let inputs = self.slot_inputs(false);
        let mut out = [255u8; 4];
        for &c in self.work.channels() {
            out[c.index()] = pack::to_u8(pack::sample(&inputs[c.index()], x, y, target));
        }
        Some(out)
    }

    // ---------------------------------------------------------------- экспорт

    /// Общее имя материала для `{basename}`.
    pub fn basename(&self) -> String {
        basename_of(&self.slots, &self.work).unwrap_or_else(|| "Untitled".to_owned())
    }

    pub fn output_name(&self) -> String {
        let name = naming::render(
            &self.output.template,
            &NameParts {
                basename: &self.basename(),
                preset: &self.work.name,
                label: &self.work.label,
            },
        );
        format!("{name}.{}", self.output.format.extension())
    }

    /// Папка результата: выбранная или рядом с первым исходником.
    pub fn output_dir(&self) -> Option<PathBuf> {
        self.output.folder.clone().or_else(|| {
            self.slots
                .iter()
                .flatten()
                .next()
                .and_then(|s| s.path.parent().map(Path::to_path_buf))
        })
    }

    pub fn can_export(&self) -> Result<(), &'static str> {
        if self.export.is_some() {
            return Err("Export in progress");
        }
        if self.target_size().is_none() {
            return Err("Assign at least one image, or set a custom size");
        }
        if self.output_dir().is_none() {
            return Err("Choose an output folder");
        }
        Ok(())
    }

    pub fn exporting(&self) -> bool {
        self.export.is_some()
    }

    pub fn export(&mut self, ctx: &egui::Context, confirmed: bool) {
        if let Err(why) = self.can_export() {
            self.toast(ToastKind::Warning, why.to_owned());
            return;
        }
        let (Some(dir), Some(size)) = (self.output_dir(), self.target_size()) else {
            return;
        };
        let name = self.output_name();
        let path = dir.join(&name);
        if path.exists() && !confirmed {
            self.dialog = Some(Dialog::Overwrite(path));
            return;
        }
        let images: [Option<Arc<Source>>; 4] = self.slots.clone();
        let configs = self.work.slots.clone();
        let alpha = self.work.alpha;
        let format = self.output.format;
        let sixteen = self.output.sixteen && format.supports_16bit();
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let slots: [SlotInput; 4] = std::array::from_fn(|i| SlotInput {
                image: images[i].as_ref().map(|s| &s.image),
                source: configs[i].source,
                srgb: configs[i].srgb,
                invert: configs[i].invert,
                fill: configs[i].fill,
            });
            let result = std::fs::create_dir_all(&dir)
                .map_err(|e| format!("Cannot create {}: {e}", dir.display()))
                .and_then(|()| {
                    pack::pack(&slots, alpha, size, sixteen)
                        .save_with_format(&path, format.image_format())
                        .map_err(|e| format!("Cannot write {}: {e}", path.display()))
                })
                .map(|()| path);
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.export = Some(ExportJob { rx, name });
    }

    fn poll_export(&mut self) {
        let Some(job) = &self.export else { return };
        let Ok(result) = job.rx.try_recv() else { return };
        let name = job.name.clone();
        self.export = None;
        match result {
            Ok(path) => {
                self.toasts.push(Toast {
                    kind: ToastKind::Success,
                    text: format!("Exported {name}"),
                    reveal: Some(path.clone()),
                    born: Instant::now(),
                });
                self.last_export = Some(path);
            }
            Err(e) => self.toast(ToastKind::Error, e),
        }
    }

    // ---------------------------------------------------------------- пакет

    pub fn batch_add(&mut self, paths: Vec<PathBuf>) {
        let before = self.batch.files.len();
        let mut files = std::mem::take(&mut self.batch.files);
        files.extend(batch::expand(paths));
        files.sort();
        files.dedup();
        self.batch.files = files;
        self.batch.plan_key = None;
        let added = self.batch.files.len() - before;
        if added == 0 {
            self.toast(ToastKind::Info, "No new images found".to_owned());
        }
    }

    /// Разложить файлы заново, если поменялись файлы или суффиксы.
    pub fn refresh_plan(&mut self) {
        let key = (
            self.work.slots.iter().map(|s| s.suffixes.clone()).collect::<Vec<_>>(),
            self.work.alpha,
            self.batch.files.len(),
        );
        if self.batch.plan_key.as_ref() != Some(&key) {
            self.batch.plan = batch::plan(&self.batch.files, &self.work);
            self.batch.plan_key = Some(key);
        }
    }

    pub fn batch_enabled(&self) -> Vec<batch::Group> {
        self.batch
            .plan
            .groups
            .iter()
            .filter(|g| !self.batch.disabled.contains(&g.base.to_lowercase()))
            .filter(|g| batch::filled(g, &self.work) > 0)
            .cloned()
            .collect()
    }

    pub fn batch_out_dir(&self) -> Option<PathBuf> {
        self.output
            .folder
            .clone()
            .or_else(|| self.batch.files.first().and_then(|f| f.parent()).map(|d| d.join("packed")))
    }

    pub fn start_batch(&mut self, ctx: &egui::Context) {
        if self.batch.run.is_some() {
            return;
        }
        let groups = self.batch_enabled();
        let Some(out_dir) = self.batch_out_dir() else {
            self.toast(ToastKind::Warning, "Choose an output folder".to_owned());
            return;
        };
        if groups.is_empty() {
            self.toast(ToastKind::Warning, "Nothing to process".to_owned());
            return;
        }
        let bases: Vec<String> = groups.iter().map(|g| g.base.clone()).collect();
        let job = batch::Job {
            groups,
            preset: self.work.clone(),
            template: self.output.template.clone(),
            format: self.output.format,
            sixteen: self.output.sixteen,
            policy: self.output.policy,
            custom: self.output.custom,
            out_dir: out_dir.clone(),
            overwrite: self.output.overwrite,
        };
        let progress = Arc::new(batch::Progress::default());
        let (tx, rx) = mpsc::channel();
        let ctx2 = ctx.clone();
        let p2 = progress.clone();
        std::thread::spawn(move || batch::run(job, p2, tx, move || ctx2.request_repaint()));
        self.batch.log.clear();
        self.batch.status.clear();
        self.batch.finished_dir = None;
        self.batch.log.push((
            LogLevel::Info,
            format!("Packing {} materials into {}", bases.len(), out_dir.display()),
        ));
        self.batch.run = Some(BatchRun {
            progress,
            rx,
            total: bases.len(),
            bases,
        });
        self.last_export = Some(out_dir);
    }

    fn poll_batch(&mut self) {
        let Some(run) = &self.batch.run else { return };
        let mut finished = false;
        let mut events = Vec::new();
        while let Ok(event) = run.rx.try_recv() {
            events.push(event);
        }
        for event in events {
            match event {
                batch::Event::Log(level, text) => self.batch.log.push((level, text)),
                batch::Event::Done(i, ok) => {
                    if let Some(base) = self.batch.run.as_ref().and_then(|r| r.bases.get(i)) {
                        self.batch.status.insert(base.to_lowercase(), ok);
                    }
                }
                batch::Event::Finished => finished = true,
            }
        }
        if finished {
            let failed = self.batch.status.values().filter(|ok| !**ok).count();
            let done = self.batch.status.len();
            self.batch.run = None;
            self.batch.finished_dir = self.last_export.clone();
            let (kind, text) = if failed == 0 {
                (ToastKind::Success, format!("Batch finished: {done} materials"))
            } else {
                (ToastKind::Warning, format!("Batch finished: {failed} of {done} failed"))
            };
            self.toasts.push(Toast {
                kind,
                text,
                reveal: self.batch.finished_dir.clone(),
                born: Instant::now(),
            });
        }
    }

    // ---------------------------------------------------------------- отмена

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            selected: self.selected,
            work: self.work.clone(),
            slots: self.slots.clone(),
        }
    }

    fn restore(&mut self, snap: &Snapshot) {
        self.selected = snap.selected.min(self.presets.len() - 1);
        self.work = snap.work.clone();
        self.slots = snap.slots.clone();
    }

    /// Запомнить шаг отмены, когда правка закончилась: перетаскивание числа
    /// или набор текста становятся одним шагом, а не сотней.
    fn settle(&mut self, ctx: &egui::Context) {
        let now = self.snapshot();
        if now.same(&self.settled) {
            return;
        }
        let busy = ctx.input(|i| i.pointer.any_down()) || ctx.egui_wants_keyboard_input();
        if busy {
            return;
        }
        let prev = std::mem::replace(&mut self.settled, now);
        self.undo.push(prev);
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        let Some(prev) = self.undo.pop() else { return };
        self.redo.push(self.snapshot());
        self.restore(&prev);
        self.settled = prev;
    }

    pub fn redo(&mut self) {
        let Some(next) = self.redo.pop() else { return };
        self.undo.push(self.snapshot());
        self.restore(&next);
        self.settled = next;
    }

    // ---------------------------------------------------------------- прочее

    pub fn toast(&mut self, kind: ToastKind, text: String) {
        self.toasts.push(Toast {
            kind,
            text,
            reveal: None,
            born: Instant::now(),
        });
    }

    fn expire_toasts(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        self.toasts.retain(|t| now.duration_since(t.born) < TOAST_TIME + extra(t));
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }

    pub fn pick_images(&mut self, target: LoadTarget) {
        let mut dialog = rfd::FileDialog::new().add_filter("Images", source::EXTENSIONS);
        if let Some(dir) = &self.last_open_dir {
            dialog = dialog.set_directory(dir);
        }
        let picked = match target {
            LoadTarget::Slot(_) => dialog.pick_file().map(|f| vec![f]),
            _ => dialog.pick_files(),
        };
        if let Some(files) = picked {
            match target {
                LoadTarget::Auto { .. } => self.open_paths(files),
                other => self.open_files(files, other),
            }
        }
    }

    pub fn pick_output_folder(&mut self) {
        let mut dialog = rfd::FileDialog::new();
        if let Some(dir) = self.output_dir().or_else(|| self.last_open_dir.clone()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(dir) = dialog.pick_folder() {
            self.output.folder = Some(dir);
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        let typing = ctx.egui_wants_keyboard_input();
        let shortcut = |m, k| KeyboardShortcut::new(m, k);
        let (export, open, undo, redo, redo2, save, reload, fit) = ctx.input_mut(|i| {
            (
                i.consume_shortcut(&shortcut(Modifiers::COMMAND, Key::E)),
                i.consume_shortcut(&shortcut(Modifiers::COMMAND, Key::O)),
                !typing && i.consume_shortcut(&shortcut(Modifiers::COMMAND, Key::Z)),
                !typing && i.consume_shortcut(&shortcut(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)),
                !typing && i.consume_shortcut(&shortcut(Modifiers::COMMAND, Key::Y)),
                i.consume_shortcut(&shortcut(Modifiers::COMMAND, Key::S)),
                i.consume_shortcut(&shortcut(Modifiers::NONE, Key::F5)),
                !typing && i.consume_shortcut(&shortcut(Modifiers::COMMAND, Key::Num0)),
            )
        });
        if self.dialog.is_some() {
            return;
        }
        if export {
            match self.mode {
                Mode::Pack => self.export(ctx, false),
                Mode::Batch => self.start_batch(ctx),
            }
        }
        if open {
            match self.mode {
                Mode::Pack => self.pick_images(LoadTarget::Auto { single: true }),
                Mode::Batch => {
                    if let Some(files) = rfd::FileDialog::new().add_filter("Images", source::EXTENSIONS).pick_files() {
                        self.batch_add(files);
                    }
                }
            }
        }
        if undo {
            self.undo();
        }
        if redo || redo2 {
            self.redo();
        }
        if save && self.modified() {
            self.save_preset();
        }
        if reload {
            self.reload_sources();
        }
        if fit {
            self.view.zoom = None;
            self.view.pan = egui::Vec2::ZERO;
        }
    }

    /// Файлы, брошенные в окно: на карточку — в её канал, иначе по суффиксам.
    fn take_dropped(&mut self, ctx: &egui::Context) {
        let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
        let pos = if hovering || ctx.input(|i| !i.raw.dropped_files.is_empty()) {
            crate::platform::cursor_pos(ctx)
        } else {
            None
        };
        self.drop_target = pos.and_then(|p| {
            Channel::ALL
                .into_iter()
                .find(|c| self.card_rects[c.index()].is_some_and(|r| r.contains(p)))
        });
        let dropped: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if dropped.is_empty() {
            return;
        }
        match self.mode {
            Mode::Batch => self.batch_add(dropped),
            Mode::Pack => {
                // Папка из Проводника — все картинки из неё, по суффиксам.
                let files = batch::expand(dropped);
                match (self.drop_target, files.len()) {
                    (Some(channel), 1) => self.open_files(files, LoadTarget::Slot(channel)),
                    _ => self.open_paths(files),
                }
            }
        }
        self.drop_target = None;
    }
}

impl eframe::App for App {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_loads(ctx);
        self.poll_export();
        self.poll_batch();
        self.take_dropped(ctx);
        self.shortcuts(ctx);
        self.expire_toasts(ctx);
        if self.mode == Mode::Batch {
            self.refresh_plan();
        }
        if self.exporting() || self.batch.run.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.mode == Mode::Pack {
            self.refresh_preview(&ctx);
        }
        crate::ui::draw(self, ui);
        self.settle(&ctx);
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = crate::theme::Palette::DARK.bg;
        egui::Rgba::from(c).to_array()
    }
}

/// Уведомление с кнопкой живёт дольше: до кнопки ещё надо дотянуться.
fn extra(toast: &Toast) -> Duration {
    if toast.reveal.is_some() || toast.kind == ToastKind::Error {
        Duration::from_secs(4)
    } else {
        Duration::ZERO
    }
}

fn basename_of(slots: &[Option<Arc<Source>>; 4], preset: &Preset) -> Option<String> {
    slots
        .iter()
        .flatten()
        .next()
        .map(|s| naming::base_name(&s.stem(), preset))
}

fn same_path(a: &Path, b: &Path) -> bool {
    a == b || a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

fn to_bytes(plane: &[f32]) -> Vec<u8> {
    plane.iter().map(|v| pack::to_u8(*v)).collect()
}

fn set_texture(ctx: &egui::Context, slot: &mut Option<TextureHandle>, name: &str, image: ColorImage) {
    // Вблизи пиксели должны быть видны как квадраты, а издали — сглажены.
    let options = TextureOptions {
        magnification: egui::TextureFilter::Nearest,
        minification: egui::TextureFilter::Linear,
        ..TextureOptions::LINEAR
    };
    match slot {
        Some(texture) => texture.set(image, options),
        None => *slot = Some(ctx.load_texture(name, image, options)),
    }
}
