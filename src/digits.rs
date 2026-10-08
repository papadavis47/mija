use std::ops::Range;

/// Every glyph is five pixel rows tall.
pub const GLYPH_H: usize = 5;

/// A scaled bitmap of rendered text, measured in terminal cells.
pub struct Mask {
    pub width: usize,
    pub height: usize,
    bits: Vec<bool>,
    colon_cols: Vec<Range<usize>>,
}

/// Three-by-five bitmaps, one row per byte, most significant bit leftmost.
/// Scaling this single set beats hand-drawing a font per size: the clock grows
/// to fill whatever pane it is given.
const DIGITS: [[u8; GLYPH_H]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b001, 0b001], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

const COLON: [u8; GLYPH_H] = [0b0, 0b1, 0b0, 0b1, 0b0];

/// Ceiling on glyph scale. Left to grow freely the clock swallows a maximized
/// terminal; capped, it stays a focal point with room to breathe around it.
const MAX_SCALE_Y: usize = 4;
const MAX_SCALE_X: usize = 5;

fn glyph(ch: char) -> Option<[u8; GLYPH_H]> {
    match ch {
        '0'..='9' => Some(DIGITS[ch as usize - '0' as usize]),
        ':' => Some(COLON),
        _ => None,
    }
}

impl Mask {
    pub fn get(&self, x: usize, y: usize) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.bits[y * self.width + x]
    }

    /// True when the column belongs to a colon separator, which breathes
    /// independently of the digits.
    pub fn is_colon(&self, x: usize) -> bool {
        self.colon_cols.iter().any(|range| range.contains(&x))
    }
}

/// Width of a glyph in pixels. Colons are a single column so the clock reads
/// as two number pairs rather than three evenly spaced groups.
pub fn glyph_width(ch: char) -> usize {
    match ch {
        ':' => 1,
        _ => 3,
    }
}

fn pixel_width(text: &str) -> usize {
    let glyphs = text.chars().count();
    if glyphs == 0 {
        return 0;
    }
    text.chars().map(glyph_width).sum::<usize>() + glyphs - 1
}

/// Width of rendered text in terminal cells, including one pixel of gap
/// between glyphs.
pub fn text_cells(text: &str, scale_x: usize) -> usize {
    pixel_width(text) * scale_x
}

pub fn render(text: &str, scale_x: usize, scale_y: usize) -> Mask {
    let width = text_cells(text, scale_x);
    let height = GLYPH_H * scale_y;
    let mut bits = vec![false; width * height];
    let mut colon_cols = Vec::new();

    let mut pixel_x = 0usize;
    for (i, ch) in text.chars().enumerate() {
        if i > 0 {
            pixel_x += 1;
        }
        let glyph_w = glyph_width(ch);
        if ch == ':' {
            colon_cols.push(pixel_x * scale_x..(pixel_x + glyph_w) * scale_x);
        }
        if let Some(rows) = glyph(ch) {
            for (row_index, row) in rows.iter().enumerate() {
                for col in 0..glyph_w {
                    if (row >> (glyph_w - 1 - col)) & 1 == 0 {
                        continue;
                    }
                    for dy in 0..scale_y {
                        let y = row_index * scale_y + dy;
                        for dx in 0..scale_x {
                            let x = (pixel_x + col) * scale_x + dx;
                            bits[y * width + x] = true;
                        }
                    }
                }
            }
        }
        pixel_x += glyph_w;
    }

    Mask {
        width,
        height,
        bits,
        colon_cols,
    }
}

/// Largest scale that fits the given cell budget, biased towards square
/// pixels (terminal cells are roughly twice as tall as they are wide).
pub fn fit_scale(text: &str, max_w: usize, max_h: usize) -> Option<(usize, usize)> {
    let pixels = pixel_width(text);
    if pixels == 0 {
        return None;
    }
    let max_scale_y = (max_h / GLYPH_H).min(MAX_SCALE_Y);
    for scale_y in (1..=max_scale_y).rev() {
        let scale_x = (max_w / pixels).min(2 * scale_y).min(MAX_SCALE_X);
        if scale_x >= scale_y {
            return Some((scale_x, scale_y));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digits_are_three_pixels_wide_and_colons_one() {
        assert_eq!(glyph_width('5'), 3);
        assert_eq!(glyph_width(':'), 1);
    }

    #[test]
    fn unscaled_glyph_is_three_by_five() {
        let mask = render("8", 1, 1);
        assert_eq!((mask.width, mask.height), (3, GLYPH_H));
    }

    #[test]
    fn scaling_multiplies_both_axes() {
        let mask = render("8", 2, 3);
        assert_eq!((mask.width, mask.height), (6, 15));
    }

    #[test]
    fn clock_text_packs_glyphs_with_single_pixel_gaps() {
        // 3 + 1 + 3 + 1 + 1 + 1 + 3 + 1 + 3
        assert_eq!(render("18:42", 1, 1).width, 17);
        assert_eq!(text_cells("18:42", 2), 34);
    }

    #[test]
    fn eight_has_the_expected_pixels() {
        let mask = render("8", 1, 1);
        assert!(mask.get(0, 0) && mask.get(1, 0) && mask.get(2, 0));
        assert!(mask.get(0, 1) && !mask.get(1, 1) && mask.get(2, 1));
        assert!(mask.get(0, 2) && mask.get(1, 2) && mask.get(2, 2));
    }

    #[test]
    fn one_has_the_expected_pixels() {
        let mask = render("1", 1, 1);
        assert!(!mask.get(0, 0) && mask.get(1, 0) && !mask.get(2, 0));
        assert!(mask.get(0, 4) && mask.get(1, 4) && mask.get(2, 4));
    }

    #[test]
    fn scaling_replicates_each_pixel() {
        let mask = render("8", 2, 2);
        assert!(mask.get(0, 0) && mask.get(1, 0) && mask.get(0, 1) && mask.get(1, 1));
        // The hollow centre of the 8 stays hollow when scaled.
        assert!(!mask.get(2, 2) && !mask.get(3, 2) && !mask.get(2, 3));
    }

    #[test]
    fn gaps_between_glyphs_are_empty() {
        let mask = render("88", 1, 1);
        assert_eq!(mask.width, 7);
        for y in 0..GLYPH_H {
            assert!(!mask.get(3, y), "gap column should be empty at row {y}");
        }
    }

    #[test]
    fn colon_columns_are_flagged() {
        let mask = render("1:1", 1, 1);
        assert_eq!(mask.width, 9);
        assert!(mask.is_colon(4));
        assert!(!mask.is_colon(0));
        assert!(!mask.is_colon(6));
    }

    #[test]
    fn colon_columns_scale_with_the_glyphs() {
        let mask = render("1:1", 2, 1);
        assert!(mask.is_colon(8) && mask.is_colon(9));
        assert!(!mask.is_colon(7) && !mask.is_colon(10));
    }

    #[test]
    fn fit_scale_returns_the_largest_scale_that_fits() {
        assert_eq!(fit_scale("18:42", 34, 5), Some((2, 1)));
        assert_eq!(fit_scale("18:42", 60, 20), Some((3, 3)));
    }

    #[test]
    fn fit_scale_gives_up_when_there_is_no_room() {
        assert_eq!(fit_scale("18:42", 10, 5), None);
        assert_eq!(fit_scale("18:42", 40, 4), None);
    }

    #[test]
    fn fit_scale_stops_growing_on_a_maximized_terminal() {
        // Budget from a 200x52 terminal, and from one far larger.
        assert_eq!(fit_scale("18:42", 194, 41), Some((5, 4)));
        assert_eq!(fit_scale("18:42", 500, 200), Some((5, 4)));
    }

    #[test]
    fn fit_scale_leaves_smaller_panes_alone() {
        // Budgets from 80x30 and 56x26, both already under the cap.
        assert_eq!(fit_scale("18:42", 74, 19), Some((4, 3)));
        assert_eq!(fit_scale("18:42", 50, 15), Some((2, 2)));
    }

    #[test]
    fn fit_scale_never_stretches_pixels_wider_than_two_cells_per_row() {
        let (sx, sy) = fit_scale("18:42", 500, 500).unwrap();
        assert!(sx <= 2 * sy, "pixels must not exceed a 2:1 cell ratio");
        assert!(sx >= sy, "pixels must not become taller than they are wide");
    }
}
