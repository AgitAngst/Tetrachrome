# Tetrachrome

Упаковщик текстурных каналов для игровых художников: собирает до четырёх
grayscale-карт (Metallic, AO, Roughness, Smoothness, Detail Mask…) в RGBA-каналы
одной текстуры по соглашениям Unity (Built-in/URP, HDRP), Unreal и Godot.

- Windows 10+ x64, один самодостаточный `.exe`
- Rust + egui (eframe), обработка на CPU в линейном пространстве
- Пресеты движков, пакетный режим с автоназначением по суффиксам, экспорт PNG/TGA/TIFF

Статус: проект только начат. Полное ТЗ — в [SPEC.md](SPEC.md).
