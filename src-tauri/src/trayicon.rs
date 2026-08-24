//! Tray icon rendering: draws the memory percent as a bitmap number with the
//! configured background / foreground colours, plus optional transparency,
//! rounded corners and border. Mirrors the original Mem Reduct tray icon.

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

    // Background
    for y in 0..SIZE {
        for x in 0..SIZE {
            let inside = if style.round {
                in_rounded_rect(x, y)
            } else {
                true
            };
            let border_px = style.border && is_border(x, y);
            let idx = (y * SIZE + x) * 4;
            if inside {
                let [r, g, b] = if border_px && style.border {
                    [255u8, 255, 255]
                } else {
                    style.bg
                };
                buf[idx] = r;
                buf[idx + 1] = g;
                buf[idx + 2] = b;
                buf[idx + 3] = if style.transparent && !border_px && !style.round {
                    // fully transparent background
                    0
                } else {
                    255
                };
            }
            // outside rounded corners → transparent
        }
    }

    // Digits
    let digits: Vec<u8> = percent
        .to_string()
        .bytes()
        .map(|b| b - b'0')
        .collect();
    // Each glyph: 3x5 cells at scale. Choose scale so text fits.
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
        for row in 0..5 {
            let bits = DIGITS[*digit as usize][row];
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    // fill scale x scale block
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

fn in_rounded_rect(x: usize, y: usize) -> bool {
    let r = 7;
    let cx = x as isize;
    let cy = y as isize;
    if cx < 0 || cy < 0 || cx >= SIZE as isize || cy >= SIZE as isize {
        return false;
    }
    // corner check
    let left = cx < r;
    let right = cx >= (SIZE as isize - r);
    let top = cy < r;
    let bottom = cy >= (SIZE as isize - r);
    if (left && top) || (right && top) || (left && bottom) || (right && bottom) {
        // distance to corner center
        let corner_x = if left { r as isize } else { SIZE as isize - 1 - r as isize };
        let corner_y = if top { r as isize } else { SIZE as isize - 1 - r as isize };
        let dx = cx - corner_x;
        let dy = cy - corner_y;
        dx * dx + dy * dy <= (r as isize) * (r as isize)
    } else {
        true
    }
}

fn is_border(x: usize, y: usize) -> bool {
    x == 0 || y == 0 || x == SIZE - 1 || y == SIZE - 1
}
