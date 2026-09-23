//! Что упаковывается: каналы, настройки слотов, пресеты.

use serde::{Deserialize, Serialize};

/// Канал выходной текстуры.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Channel {
    R,
    G,
    B,
    A,
}

impl Channel {
    pub const ALL: [Channel; 4] = [Channel::R, Channel::G, Channel::B, Channel::A];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn letter(self) -> &'static str {
        match self {
            Channel::R => "R",
            Channel::G => "G",
            Channel::B => "B",
            Channel::A => "A",
        }
    }
}

/// Откуда брать значение в исходной картинке.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SourceChannel {
    R,
    G,
    B,
    A,
    /// Яркость по Rec. 709, считается в линейном пространстве.
    #[default]
    Lum,
}

impl SourceChannel {
    pub const ALL: [SourceChannel; 5] =
        [SourceChannel::R, SourceChannel::G, SourceChannel::B, SourceChannel::A, SourceChannel::Lum];

    pub fn label(self) -> &'static str {
        match self {
            SourceChannel::R => "R",
            SourceChannel::G => "G",
            SourceChannel::B => "B",
            SourceChannel::A => "A",
            SourceChannel::Lum => "Lum",
        }
    }
}

/// Движок, под который сделан пресет. Нужен для группировки и цветной метки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Engine {
    Unity,
    Unreal,
    Godot,
    #[default]
    Other,
}

impl Engine {
    pub const ALL: [Engine; 4] = [Engine::Unity, Engine::Unreal, Engine::Godot, Engine::Other];

    pub fn label(self) -> &'static str {
        match self {
            Engine::Unity => "Unity",
            Engine::Unreal => "Unreal",
            Engine::Godot => "Godot",
            Engine::Other => crate::lang::t("Other"),
        }
    }
}

/// Настройка одного выходного канала.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SlotConfig {
    /// Что лежит в канале: «Metallic», «Ambient Occlusion». Пусто — канал
    /// заливается постоянным значением.
    pub role: String,
    /// Какой канал исходника читать.
    pub source: SourceChannel,
    pub invert: bool,
    /// Значение, которым канал заливается без картинки.
    pub fill: u8,
    /// Исходник в sRGB: перед упаковкой перевести в линейное пространство.
    pub srgb: bool,
    /// Части имени файла, по которым картинка сама попадает в этот канал:
    /// `_Metallic`, `_Metal`. Регистр не важен, `*` по краям допускается.
    pub suffixes: Vec<String>,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            role: String::new(),
            source: SourceChannel::Lum,
            invert: false,
            fill: 0,
            srgb: false,
            suffixes: Vec::new(),
        }
    }
}

impl SlotConfig {
    pub fn role(role: &str, suffixes: &[&str]) -> Self {
        Self { role: role.to_owned(), suffixes: suffixes.iter().map(|s| (*s).to_owned()).collect(), ..Self::default() }
    }

    pub fn empty(fill: u8) -> Self {
        Self { fill, ..Self::default() }
    }
}

/// Пресет: как разложить карты по каналам для конкретного движка.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preset {
    pub name: String,
    pub engine: Engine,
    /// Метка в имени выходного файла: `_MaskMap`, `_ORM`.
    pub label: String,
    /// Писать ли альфа-канал. Без него на выходе RGB.
    pub alpha: bool,
    /// Каналы по порядку R, G, B, A.
    pub slots: [SlotConfig; 4],
    /// Встроенный пресет: не удаляется и не перезаписывается.
    #[serde(skip)]
    pub builtin: bool,
}

impl Default for Preset {
    fn default() -> Self {
        Self {
            name: crate::lang::t("New preset").to_owned(),
            engine: Engine::Other,
            label: "_Packed".to_owned(),
            alpha: true,
            slots: [SlotConfig::empty(0), SlotConfig::empty(0), SlotConfig::empty(0), SlotConfig::empty(255)],
            builtin: false,
        }
    }
}

impl Preset {
    /// Совпадают ли настройки — без учёта того, встроенный ли пресет.
    pub fn same_config(&self, other: &Preset) -> bool {
        self.name == other.name
            && self.engine == other.engine
            && self.label == other.label
            && self.alpha == other.alpha
            && self.slots == other.slots
    }

    /// Каналы, которые попадут в файл.
    pub fn channels(&self) -> &'static [Channel] {
        if self.alpha { &Channel::ALL } else { &Channel::ALL[..3] }
    }
}

/// Формат выходного файла — только без потерь.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    #[default]
    Png,
    Tga,
    Tiff,
}

impl OutputFormat {
    pub const ALL: [OutputFormat; 3] = [OutputFormat::Png, OutputFormat::Tga, OutputFormat::Tiff];

    pub fn label(self) -> &'static str {
        match self {
            OutputFormat::Png => "PNG",
            OutputFormat::Tga => "TGA",
            OutputFormat::Tiff => "TIFF",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Png => "png",
            OutputFormat::Tga => "tga",
            OutputFormat::Tiff => "tif",
        }
    }

    /// TGA бывает только 8-битным.
    pub fn supports_16bit(self) -> bool {
        !matches!(self, OutputFormat::Tga)
    }

    pub fn image_format(self) -> image::ImageFormat {
        match self {
            OutputFormat::Png => image::ImageFormat::Png,
            OutputFormat::Tga => image::ImageFormat::Tga,
            OutputFormat::Tiff => image::ImageFormat::Tiff,
        }
    }
}

/// Как быть, если исходники разного размера.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SizePolicy {
    #[default]
    Largest,
    Smallest,
    Custom,
}

pub const MAX_SIZE: u32 = 8192;
