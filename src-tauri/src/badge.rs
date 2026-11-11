//! The unread badge (Messenger 1.0.2): a small red circle with the count, drawn here as raw RGBA so no image library is needed. std-only, tested with `rustc --test src/badge.rs`.
//! Windows shows it as the taskbar overlay icon; macOS and Linux use the platform's own badge count (see badge_value).

pub const SIZE: u32 = 32;

/// "1" to "9", then "9+". Zero or less shows nothing.
pub fn label(count: i64) -> Option<String> {
    match count {
        i64::MIN..=0 => None,
        1..=9 => Some(count.to_string()),
        _ => Some("9+".to_string()),
    }
}

/// The number the platform badge shows (macOS dock, Linux launcher). None clears it.
pub fn badge_value(count: i64) -> Option<i64> {
    (count > 0).then_some(count)
}

/// 3 wide by 5 high digits, one row per u8 (bits 2..0); '+' is drawn too.
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        _ => [0; 5],
    }
}

/// RGBA pixels (SIZE x SIZE) of a red circle with the label in white, or None when there is nothing to show.
pub fn overlay_rgba(count: i64) -> Option<Vec<u8>> {
    let text = label(count)?;
    let n = SIZE as i32;
    let mut px = vec![0u8; (SIZE * SIZE * 4) as usize];
    let c = (n as f32 - 1.0) / 2.0;
    let r = n as f32 / 2.0;
    for y in 0..n {
        for x in 0..n {
            let d = ((x as f32 - c).powi(2) + (y as f32 - c).powi(2)).sqrt();
            if d <= r - 0.5 {
                let i = ((y * n + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&[220, 38, 38, 255]);
            }
        }
    }
    // scale 3 for one or two glyphs: glyph 3x5 -> 9x15 (one char) or 2 chars with a 3 px gap: 21 wide
    let scale = 3;
    let chars: Vec<char> = text.chars().collect();
    let gw = 3 * scale;
    let total = chars.len() as i32 * gw + (chars.len() as i32 - 1) * scale;
    let (x0, y0) = ((n - total) / 2, (n - 5 * scale) / 2);
    for (k, ch) in chars.iter().enumerate() {
        let g = glyph(*ch);
        for (row, bits) in g.iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let (x, y) = (x0 + k as i32 * (gw + scale) + col as i32 * scale + dx, y0 + row as i32 * scale + dy);
                            if (0..n).contains(&x) && (0..n).contains(&y) {
                                let i = ((y * n + x) * 4) as usize;
                                px[i..i + 4].copy_from_slice(&[255, 255, 255, 255]);
                            }
                        }
                    }
                }
            }
        }
    }
    Some(px)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_the_count_up_to_nine_then_nine_plus_and_nothing_for_zero() {
        assert_eq!(label(0), None);
        assert_eq!(label(-4), None);
        assert_eq!(label(1).as_deref(), Some("1"));
        assert_eq!(label(9).as_deref(), Some("9"));
        assert_eq!(label(10).as_deref(), Some("9+"));
        assert_eq!(label(i64::MAX).as_deref(), Some("9+"));
    }

    #[test]
    fn the_platform_badge_value_clears_at_zero() {
        assert_eq!(badge_value(0), None);
        assert_eq!(badge_value(-1), None);
        assert_eq!(badge_value(7), Some(7));
        assert_eq!(badge_value(250), Some(250));
    }

    #[test]
    fn no_overlay_for_zero_and_a_full_size_image_otherwise() {
        assert!(overlay_rgba(0).is_none());
        for n in [1, 5, 9, 10, 99, 1000] {
            let px = overlay_rgba(n).unwrap();
            assert_eq!(px.len(), (SIZE * SIZE * 4) as usize, "{n}");
        }
    }

    #[test]
    fn the_image_is_a_red_circle_with_white_marks_and_clear_corners() {
        let px = overlay_rgba(3).unwrap();
        let at = |x: u32, y: u32| &px[((y * SIZE + x) * 4) as usize..((y * SIZE + x) * 4 + 4) as usize];
        assert_eq!(at(0, 0), [0, 0, 0, 0], "corner is transparent");
        assert_eq!(at(SIZE - 1, SIZE - 1), [0, 0, 0, 0]);
        assert_eq!(at(3, 16), [220, 38, 38, 255], "inside the circle, outside the digit, is red");
        assert!(px.chunks(4).any(|p| p == [255, 255, 255, 255]), "the digit is drawn in white");
    }

    #[test]
    fn different_counts_draw_different_pictures() {
        assert_ne!(overlay_rgba(1), overlay_rgba(2));
        assert_ne!(overlay_rgba(9), overlay_rgba(10));
        assert_eq!(overlay_rgba(10), overlay_rgba(500), "both read 9+");
    }
}
