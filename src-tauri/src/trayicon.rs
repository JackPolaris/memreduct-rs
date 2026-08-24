//! Tray icon rendering: draws the memory percent as a bitmap number on a
//! configurable background, with optional transparency, rounded corners and a
//! crisp border. Mirrors the original Mem Reduct tray icon.

const SIZE: usize = 32;

/// 3x5 bitmap font for digits 0-9.
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

/// Border thickness in pixels (1..=3).
const BORDER: usize = 2;
/// Corner radius when `round` is enabled.
const RADIUS: isize = 7;

pub struct TrayIconStyle {
    pub bg: [u8; 3],
    pub fg: [u8; 3],
    pub transparent: bool,
    pub border: bool,
    pub round: bool,
}

impl Default for TrayIconStyle {
    fn default() -> Self {
        Self {
            bg: [0x00, 0x80, 0x40],
            fg: [0xff, 0xff, 0xff],
            transparent: false,
            border: false,
            round: false,
        }
    }
}

/// Convert a %02u32 color (0x00RRGGBB) into [r, g, b].
pub fn unpack_color(rgb: u32) -> [u8; 3] {
    [
        ((rgb >> 16) & 0xff) as u8,
        ((rgb >> 8) & 0xff) as u8,
        (rgb & 0xff) as u8,
    ]
}

/// Render `percent` as a 32x32 RGBA bitmap.
pub fn render(percent: u32, style: &TrayIconStyle) -> Vec<u8> {
    let mut buf = vec![0u8; SIZE * SIZE * 4];
    let percent = percent.min(999);

    for y in 0..SIZE {
        for x in 0..SIZE {
            let (on_bg, is_border) = classify(x, y, style.round, style.border);
            let idx = (y * SIZE + x) * 4;
            if is_border {
                // Crisp single-colour border (always opaque).
                buf[idx] = style.fg[0];
                buf[idx + 1] = style.fg[1];
                buf[idx + 2] = style.fg[2];
                buf[idx + 3] = 255;
            } else if on_bg {
                // Background fill; honour transparency.
                let transparent = style.transparent;
                if !transparent {
                    buf[idx] = style.bg[0];
                    buf[idx + 1] = style.bg[1];
                    buf[idx + 2] = style.bg[2];
                }
                buf[idx + 3] = if transparent { 0 } else { 255 };
            }
            // else: outside shape → alpha 0 already
        }
    }

    // Digits
    let digits: Vec<u8> = percent.to_string().bytes().map(|b| b - b'0').collect();
    let scale = match digits.len() {
        1 => 5,
        2 => 3,
        _ => 2,
    };
    let glyph_w = 3 * scale;
    let glyph_h = 5 * scale;
    let gap = scale;
    let total_w = digits.len() * glyph_w + (digits.len().saturating_sub(1)) * gap;
    let total_h = glyph_h;
    let start_x = (SIZE as isize - total_w as isize) / 2;
    let start_y = (SIZE as isize - total_h as isize) / 2;

    for (di, digit) in digits.iter().enumerate() {
        let ox = start_x + di as isize * (glyph_w + gap) as isize;
        for (row, row_bits) in DIGITS[*digit as usize].iter().enumerate() {
            for col in 0..3 {
                if row_bits & (1 << (2 - col)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let x = ox + (col * scale + dx) as isize;
                            let y = start_y + (row * scale + dy) as isize;
                            if x >= 0 && y >= 0 && (x as usize) < SIZE && (y as usize) < SIZE {
                                let idx = ((y as usize) * SIZE + x as usize) * 4;
                                buf[idx] = style.fg[0];
                                buf[idx + 1] = style.fg[1];
                                buf[idx + 2] = style.fg[2];
                                buf[idx + 3] = 255;
                            }
                        }
                    }
                }
            }
        }
    }

    buf
}

/// Classify a pixel: inside background shape, and whether it's border.
fn classify(x: usize, y: usize, round: bool, border: bool) -> (bool, bool) {
    let inside = if round { in_rounded(x, y) } else { true };
    if !inside {
        return (false, false);
    }
    let is_border = border && is_border_pixel(x, y);
    // Background is inside minus the border ring.
    let on_bg = !is_border;
    (on_bg, is_border)
}

fn in_rounded(x: usize, y: usize) -> bool {
    let cx = x as isize;
    let cy = y as isize;
    if cx < 0 || cy < 0 || cx >= SIZE as isize || cy >= SIZE as isize {
        return false;
    }
    let r = RADIUS;
    let left = cx < r;
    let right = cx >= (SIZE as isize - r);
    let top = cy < r;
    let bottom = cy >= (SIZE as isize - r);
    if (top || bottom) && (left || right) {
        let corner_x = if left { r } else { SIZE as isize - 1 - r };
        let corner_y = if top { r } else { SIZE as isize - 1 - r };
        let dx = cx - corner_x;
        let dy = cy - corner_y;
        dx * dx + dy * dy <= r * r
    } else {
        true
    }
}

/// A pixel belongs to the border ring (contiguous thickness).
fn is_border_pixel(x: usize, y: usize) -> bool {
    x < BORDER || y < BORDER || x >= SIZE - BORDER || y >= SIZE - BORDER
}
