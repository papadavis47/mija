use ratatui::style::Color;

use crate::timer::State;

/// Named colours in Mija's monochrome rose palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Rose,
    Blush,
    Mulberry,
    Plum,
    Mist,
    Dusk,
    Ink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Truecolor,
    Indexed,
}

pub struct Theme {
    pub mode: ColorMode,
}

/// How far a spent clock row fades towards plum. Stopping short of the full
/// plum keeps drained digits legible against the ink background — the point is
/// that spent time recedes, not that it disappears.
const DRAIN_DEPTH: f64 = 0.6;

impl Tone {
    pub fn rgb(self) -> (u8, u8, u8) {
        match self {
            Tone::Rose => (0xc8, 0x4e, 0x89),
            Tone::Blush => (0xf2, 0xa8, 0xc8),
            Tone::Mulberry => (0x8b, 0x3a, 0x6b),
            Tone::Plum => (0x5e, 0x24, 0x40),
            Tone::Mist => (0xb8, 0x9a, 0xa8),
            Tone::Dusk => (0x7a, 0x64, 0x70),
            Tone::Ink => (0x16, 0x0d, 0x13),
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Tone::Rose => 168,
            Tone::Blush => 218,
            Tone::Mulberry => 89,
            Tone::Plum => 53,
            Tone::Mist => 138,
            Tone::Dusk => 96,
            Tone::Ink => 233,
        }
    }
}

impl Theme {
    pub fn detect(colorterm: Option<&str>) -> Self {
        let truecolor = matches!(colorterm, Some("truecolor") | Some("24bit"));
        Self {
            mode: if truecolor {
                ColorMode::Truecolor
            } else {
                ColorMode::Indexed
            },
        }
    }

    pub fn from_env() -> Self {
        Self::detect(std::env::var("COLORTERM").ok().as_deref())
    }

    pub fn color(&self, tone: Tone) -> Color {
        match self.mode {
            ColorMode::Truecolor => {
                let (r, g, b) = tone.rgb();
                Color::Rgb(r, g, b)
            }
            ColorMode::Indexed => Color::Indexed(tone.index()),
        }
    }

    /// Mix two tones. Truecolor interpolates so the drain line slides
    /// smoothly; the indexed fallback snaps to whichever tone is nearer.
    pub fn blend(&self, from: Tone, to: Tone, t: f64) -> Color {
        let t = t.clamp(0.0, 1.0);
        match self.mode {
            ColorMode::Indexed => {
                if t < 0.5 {
                    self.color(from)
                } else {
                    self.color(to)
                }
            }
            ColorMode::Truecolor => {
                let (r0, g0, b0) = from.rgb();
                let (r1, g1, b1) = to.rgb();
                let mix = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round() as u8;
                Color::Rgb(mix(r0, r1), mix(g0, g1), mix(b0, b1))
            }
        }
    }

    /// Colour for a clock row that has drained by `amount`, 0.0 (full) to
    /// 1.0 (spent).
    pub fn drained(&self, tone: Tone, amount: f64) -> Color {
        let amount = amount.clamp(0.0, 1.0);
        match self.mode {
            // Indexed has no room to interpolate, so it snaps on the raw drain
            // rather than the shortened one.
            ColorMode::Indexed => {
                if amount < 0.5 {
                    self.color(tone)
                } else {
                    self.color(Tone::Plum)
                }
            }
            ColorMode::Truecolor => self.blend(tone, Tone::Plum, amount * DRAIN_DEPTH),
        }
    }
}

/// The tone that carries a state's identity. Every state is a value of the
/// same rose, so the app reads pink whatever it is doing.
pub fn accent(state: State) -> Tone {
    match state {
        State::Work => Tone::Rose,
        State::ShortBreak => Tone::Blush,
        State::LongBreak => Tone::Mulberry,
        State::Paused | State::Idle => Tone::Dusk,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rose_is_the_primary_brand_colour() {
        assert_eq!(Tone::Rose.rgb(), (0xc8, 0x4e, 0x89));
    }

    #[test]
    fn truecolor_env_selects_truecolor_mode() {
        assert_eq!(Theme::detect(Some("truecolor")).mode, ColorMode::Truecolor);
        assert_eq!(Theme::detect(Some("24bit")).mode, ColorMode::Truecolor);
    }

    #[test]
    fn missing_or_unknown_colorterm_falls_back_to_indexed() {
        assert_eq!(Theme::detect(None).mode, ColorMode::Indexed);
        assert_eq!(Theme::detect(Some("")).mode, ColorMode::Indexed);
        assert_eq!(Theme::detect(Some("256")).mode, ColorMode::Indexed);
    }

    #[test]
    fn truecolor_theme_emits_exact_rgb() {
        let theme = Theme::detect(Some("truecolor"));
        assert_eq!(theme.color(Tone::Rose), Color::Rgb(0xc8, 0x4e, 0x89));
    }

    #[test]
    fn indexed_theme_emits_palette_indices() {
        let theme = Theme::detect(None);
        assert_eq!(theme.color(Tone::Rose), Color::Indexed(168));
        assert_eq!(theme.color(Tone::Plum), Color::Indexed(53));
    }

    #[test]
    fn blend_endpoints_are_the_endpoint_tones() {
        let theme = Theme::detect(Some("truecolor"));
        assert_eq!(
            theme.blend(Tone::Rose, Tone::Plum, 0.0),
            theme.color(Tone::Rose)
        );
        assert_eq!(
            theme.blend(Tone::Rose, Tone::Plum, 1.0),
            theme.color(Tone::Plum)
        );
    }

    #[test]
    fn blend_interpolates_in_truecolor() {
        let theme = Theme::detect(Some("truecolor"));
        let (r0, g0, b0) = Tone::Rose.rgb();
        let (r1, g1, b1) = Tone::Plum.rgb();
        let expected = Color::Rgb(
            ((r0 as f64 + r1 as f64) / 2.0).round() as u8,
            ((g0 as f64 + g1 as f64) / 2.0).round() as u8,
            ((b0 as f64 + b1 as f64) / 2.0).round() as u8,
        );
        assert_eq!(theme.blend(Tone::Rose, Tone::Plum, 0.5), expected);
    }

    #[test]
    fn blend_snaps_to_nearest_tone_when_indexed() {
        let theme = Theme::detect(None);
        assert_eq!(
            theme.blend(Tone::Rose, Tone::Plum, 0.4),
            theme.color(Tone::Rose)
        );
        assert_eq!(
            theme.blend(Tone::Rose, Tone::Plum, 0.6),
            theme.color(Tone::Plum)
        );
    }

    #[test]
    fn blend_clamps_out_of_range_ratios() {
        let theme = Theme::detect(Some("truecolor"));
        assert_eq!(
            theme.blend(Tone::Rose, Tone::Plum, -1.0),
            theme.color(Tone::Rose)
        );
        assert_eq!(
            theme.blend(Tone::Rose, Tone::Plum, 2.0),
            theme.color(Tone::Plum)
        );
    }

    #[test]
    fn an_undrained_row_is_the_state_tone() {
        let theme = Theme::detect(Some("truecolor"));
        assert_eq!(theme.drained(Tone::Rose, 0.0), theme.color(Tone::Rose));
    }

    #[test]
    fn a_spent_row_stays_short_of_full_plum_so_it_stays_legible() {
        let theme = Theme::detect(Some("truecolor"));
        let spent = theme.drained(Tone::Rose, 1.0);
        assert_ne!(spent, theme.color(Tone::Plum));
        assert_eq!(spent, theme.blend(Tone::Rose, Tone::Plum, DRAIN_DEPTH));
    }

    #[test]
    fn indexed_drain_snaps_on_the_raw_fraction() {
        let theme = Theme::detect(None);
        assert_eq!(theme.drained(Tone::Rose, 0.4), theme.color(Tone::Rose));
        assert_eq!(theme.drained(Tone::Rose, 0.6), theme.color(Tone::Plum));
    }

    #[test]
    fn drain_clamps_out_of_range_amounts() {
        let theme = Theme::detect(Some("truecolor"));
        assert_eq!(theme.drained(Tone::Rose, -1.0), theme.color(Tone::Rose));
        assert_eq!(
            theme.drained(Tone::Rose, 5.0),
            theme.drained(Tone::Rose, 1.0)
        );
    }

    #[test]
    fn each_state_maps_to_a_rose_family_tone() {
        assert_eq!(accent(State::Work), Tone::Rose);
        assert_eq!(accent(State::ShortBreak), Tone::Blush);
        assert_eq!(accent(State::LongBreak), Tone::Mulberry);
        assert_eq!(accent(State::Paused), Tone::Dusk);
        assert_eq!(accent(State::Idle), Tone::Dusk);
    }
}
