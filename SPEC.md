# Tetrachrome — техническое задание

Рабочее название в исходном ТЗ: "Texture Channel Packer". Язык реализации: **Rust**.

## Purpose
A GUI tool for game artists to pack multiple grayscale texture maps into
the RGBA channels of a single output image — following conventions of
different game engines and render pipelines.

---

## Core Functionality

### Channel Packing
- Load up to 4 source images (one per channel: R, G, B, A)
- Each channel slot supports:
  - Assign any loaded image
  - Choose which channel to READ from the source (R/G/B/A or Luminance)
  - Invert the channel value (checkbox)
  - Fill with constant value (0 or 255) if no image is assigned
- Preview the output image in real time (RGBA composite + per-channel grayscale strips)
- Export as PNG (default), TGA, or TIFF (lossless only — no JPEG)

### Preset System
Ship the following built-in presets (user cannot delete them, but can clone):

**Unity Built-in / URP — Metallic workflow**
- R: Metallic (grayscale)
- G: (empty → 0)
- B: (empty → 0)
- A: Smoothness (grayscale)
- Output label: "_MetallicSmoothness"

**Unity HDRP — Mask Map (MADS)**
- R: Metallic
- G: Ambient Occlusion
- B: Detail Mask
- A: Smoothness
- Output label: "_MaskMap"

**Unreal Engine — ORM**
- R: (empty → 255, Occlusion default white)
- G: Roughness
- B: Metallic
- Output label: "_ORM"  (note: Unreal reads AO from R, Roughness G, Metallic B)

**Unreal Engine — OcclusionRoughnessMetallic (packed)**
- R: Ambient Occlusion
- G: Roughness
- B: Metallic
- A: (empty → 0)
- Output label: "_ORM"

**Godot — ORM**
- R: Ambient Occlusion
- G: Roughness
- B: Metallic
- Output label: "_ORM"

**Custom** — all slots empty, user fills manually

---

## Preset Management
- Presets panel (sidebar or dropdown)
- Built-in presets: read-only, with a "Duplicate" button
- User presets: full CRUD (create, rename, delete, reorder)
- Presets stored in a local JSON file next to the executable (or %APPDATA%)
- Each preset stores: name, engine tag, per-channel config (source role, invert,
  constant fill, output channel slot)

---

## Batch Mode
- "Add files" button: load multiple source files at once
  (e.g., drop a folder of _Metallic.png, _AO.png, _Roughness.png)
- Auto-assign by filename suffix (configurable suffix→channel mapping per preset):
  e.g., "*_Metallic*" → R slot, "*_AO*" → G slot, etc.
- Process all → export all packed textures to a chosen output folder
- Progress bar + log window

---

## UI Layout
- Left panel: Preset selector + preset editor
- Center: Per-channel input slots (R / G / B / A), each with:
  - Image thumbnail preview
  - File path label
  - Source channel selector (R / G / B / A / Lum)
  - Invert toggle
  - "Clear" button
- Right panel: Output preview
  - Composite RGBA thumbnail (checkerboard background for alpha)
  - Four grayscale strip previews (one per output channel)
  - Output filename template field (supports tokens: {basename}, {preset}, {date})
  - Export button
- Top bar: menu (File / Presets / Help), zoom control for preview

---

## Technical Requirements
- Language: Rust — egui (eframe) for UI, image crate for pixel ops
- Target: Windows 10+ x64, single self-contained .exe, no installer required
- All image processing in linear color space; handle sRGB→Linear conversion
  on load when source is a color texture (flag per slot: "sRGB input")
- Max texture size: 8192×8192; warn if sources have mismatched resolutions
  (offer: scale to largest, scale to smallest, scale to custom)
- No GPU required; CPU-only processing is fine for this tool

---

## Nice-to-Have (implement if straightforward)
- Drag-and-drop image files onto channel slots
- "Open output folder" button after export
- Dark theme by default
- Keyboard shortcuts: Ctrl+E = Export, Ctrl+O = Open image, Ctrl+Z = Undo last slot change
- Remember last used folders (per-session)
- Show pixel value under cursor in preview (R/G/B/A numeric)
