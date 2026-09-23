// Значок приложения: четыре плитки R, G, B, A на тёмной подложке.
//
// Файл подключается и в приложение (значок окна), и в build.rs (значок exe)
// через `include!`, поэтому здесь нет ни `use crate`, ни внешних крейтов.

/// Значок `size`×`size` в RGBA без предумножения.
pub fn render_icon(size: u32) -> Vec<u8> {
    let s = size as f32;
    let mut out = vec![0u8; (size * size * 4) as usize];
    let bg_radius = s * 0.22;
    let pad = s * 0.16;
    let gap = s * 0.07;
    let tile = (s - pad * 2.0 - gap) / 2.0;
    let tile_radius = tile * 0.24;
    let tiles: [(f32, f32, [f32; 3], [f32; 3]); 4] = [
        (pad, pad, [255.0, 95.0, 109.0], [255.0, 95.0, 109.0]),
        (pad + tile + gap, pad, [69.0, 212.0, 138.0], [69.0, 212.0, 138.0]),
        (pad, pad + tile + gap, [79.0, 157.0, 255.0], [79.0, 157.0, 255.0]),
        (pad + tile + gap, pad + tile + gap, [236.0, 239.0, 245.0], [90.0, 96.0, 110.0]),
    ];
    for y in 0..size {
        for x in 0..size {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let bg = coverage(rounded_rect(px, py, 0.0, 0.0, s, s, bg_radius));
            if bg <= 0.0 {
                continue;
            }
            // Подложка: сверху чуть светлее.
            let t = py / s;
            let mut color = [
                lerp(40.0, 20.0, t),
                lerp(44.0, 23.0, t),
                lerp(56.0, 30.0, t),
            ];
            for &(tx, ty, from, to) in &tiles {
                let c = coverage(rounded_rect(px, py, tx, ty, tile, tile, tile_radius));
                if c > 0.0 {
                    // Альфа-плитка — диагональный градиент: намёк на прозрачность.
                    let k = (((px - tx) + (py - ty)) / (tile * 2.0)).clamp(0.0, 1.0);
                    for i in 0..3 {
                        let tile_color = lerp(from[i], to[i], k);
                        color[i] = lerp(color[i], tile_color, c);
                    }
                }
            }
            let at = ((y * size + x) * 4) as usize;
            out[at] = color[0].round() as u8;
            out[at + 1] = color[1].round() as u8;
            out[at + 2] = color[2].round() as u8;
            out[at + 3] = (bg * 255.0).round() as u8;
        }
    }
    out
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Покрытие пикселя по расстоянию до края фигуры.
fn coverage(distance: f32) -> f32 {
    (0.5 - distance).clamp(0.0, 1.0)
}

/// Знаковое расстояние до скруглённого прямоугольника: меньше нуля — внутри.
fn rounded_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    let qx = (px - cx).abs() - (w / 2.0 - r);
    let qy = (py - cy).abs() - (h / 2.0 - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    outside + qx.max(qy).min(0.0) - r
}
