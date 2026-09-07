use dioxus::prelude::*;

const FONT_PX: f64 = 16.0;
const COLUMNS: usize = 120;
const COLUMN_GLYPHS: usize = 96;
const GLYPHS: &str = "ｱｲｳｴｵｶｷｸｹｺｻｼｽｾｿﾀﾁﾂﾃﾄﾅﾆﾇﾈﾉﾊﾋﾌﾍﾎﾏﾐﾑﾒﾓﾔﾕﾖﾗﾘﾙﾚﾛﾜﾝ0123456789";

#[component]
pub fn MatrixRain(accent_rgb: String, words: Vec<String>) -> Element {
    let head = format!(
        "light-dark(rgb({}), {})",
        Accent::darkened(&accent_rgb, 42),
        Accent::brightened(&accent_rgb)
    );
    let trail = format!(
        "light-dark(rgb({} / 0.55), rgb({} / 0.5))",
        Accent::darkened(&accent_rgb, 55),
        accent_rgb
    );
    let words: Vec<Vec<char>> = words
        .iter()
        .filter(|word| !word.is_empty())
        .map(|word| word.chars().collect())
        .collect();

    rsx! {
        div {
            class: "absolute inset-0 overflow-hidden font-mono text-[16px] leading-[16px]",
            style: "--vmux-rain-head:{head};--vmux-rain-trail:{trail};",

            for index in 0..COLUMNS {
                {
                    let column = RainColumn::at(index, &words);
                    rsx! {
                        div {
                            key: "{index}",
                            class: "absolute top-0 whitespace-pre",
                            style: "{column.style()}",
                            div {
                                class: "absolute left-0 top-0 text-[var(--vmux-rain-trail)] [-webkit-mask-image:linear-gradient(to_bottom,transparent_0%,rgb(0_0_0/.08)_18%,rgb(0_0_0/.4)_65%,#000_100%)] [-webkit-mask-repeat:no-repeat] [-webkit-mask-size:100%_320px] [mask-image:linear-gradient(to_bottom,transparent_0%,rgb(0_0_0/.08)_18%,rgb(0_0_0/.4)_65%,#000_100%)] [mask-repeat:no-repeat] [mask-size:100%_320px] motion-reduce:!animate-none motion-reduce:opacity-[0.08] motion-reduce:[-webkit-mask-image:none] motion-reduce:[mask-image:none]",
                                style: "{column.animation_style()}",
                                "{column.glyphs}"
                            }
                            div {
                                class: "absolute left-0 top-0 text-[var(--vmux-rain-head)] [text-shadow:0_0_8px_var(--vmux-rain-head)] [-webkit-mask-image:linear-gradient(to_bottom,transparent_0%,transparent_88%,#000_96%,transparent_100%)] [-webkit-mask-repeat:no-repeat] [-webkit-mask-size:100%_320px] [mask-image:linear-gradient(to_bottom,transparent_0%,transparent_88%,#000_96%,transparent_100%)] [mask-repeat:no-repeat] [mask-size:100%_320px] motion-reduce:hidden",
                                style: "{column.animation_style()}",
                                "{column.glyphs}"
                            }
                        }
                    }
                }
            }
        }
    }
}

struct RainColumn {
    index: usize,
    glyphs: String,
    duration_seconds: f64,
    delay_seconds: f64,
}

impl RainColumn {
    fn at(index: usize, words: &[Vec<char>]) -> Self {
        let glyphs: Vec<char> = GLYPHS.chars().collect();
        let word = (!words.is_empty() && index % 7 == 3).then(|| &words[index % words.len()]);
        let mut column_glyphs = String::with_capacity(COLUMN_GLYPHS * 2);
        for row in 0..COLUMN_GLYPHS {
            if row > 0 {
                column_glyphs.push('\n');
            }
            let character = match word {
                Some(word) => word[row % word.len()],
                None => glyphs[Self::noise(index * 97 + row) as usize % glyphs.len()],
            };
            column_glyphs.push(character);
        }
        let duration_seconds = 5.0 + (Self::noise(index * 17) % 4500) as f64 / 1000.0;
        let delay_seconds =
            -((Self::noise(index * 31) % 10_000) as f64 / 10_000.0 * duration_seconds);
        Self {
            index,
            glyphs: column_glyphs,
            duration_seconds,
            delay_seconds,
        }
    }

    fn style(&self) -> String {
        format!("left:{}px;", self.index as f64 * FONT_PX)
    }

    fn animation_style(&self) -> String {
        format!(
            "animation:vmux-rain-window {:.3}s linear {:.3}s infinite both;",
            self.duration_seconds, self.delay_seconds
        )
    }

    fn noise(seed: usize) -> u64 {
        let mut x = (seed as u64)
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        x ^= x >> 33;
        x = x.wrapping_mul(0xff51afd7ed558ccd);
        x ^ (x >> 33)
    }
}

struct Accent;

impl Accent {
    fn brightened(accent_rgb: &str) -> String {
        let Some([r, g, b]) = Self::parse(accent_rgb) else {
            return "rgb(220 230 255)".to_string();
        };
        let mix = |c: u16| c + (255 - c) * 7 / 10;
        format!("rgb({} {} {})", mix(r), mix(g), mix(b))
    }

    fn darkened(accent_rgb: &str, pct: u16) -> String {
        let Some([r, g, b]) = Self::parse(accent_rgb) else {
            return "20 24 33".to_string();
        };
        let mix = |c: u16| c * pct / 100;
        format!("{} {} {}", mix(r), mix(g), mix(b))
    }

    fn parse(accent_rgb: &str) -> Option<[u16; 3]> {
        let mut parts = accent_rgb.split_whitespace();
        let mut channel = || parts.next()?.parse::<u16>().ok();
        let rgb = [channel()?, channel()?, channel()?];
        parts.next().is_none().then_some(rgb)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_woven_column_reads_as_the_word_it_was_given() {
        let words = vec!["CLAUDE".chars().collect::<Vec<_>>()];
        let column = RainColumn::at(3, &words);

        let shown: String = column
            .glyphs
            .chars()
            .filter(|glyph| *glyph != '\n')
            .collect();
        assert_eq!(shown.chars().count(), COLUMN_GLYPHS);
        assert!(shown.starts_with("CLAUDE"), "got {shown}");
    }

    #[test]
    fn adjacent_columns_do_not_share_a_fall() {
        let first = RainColumn::at(10, &[]);
        let second = RainColumn::at(11, &[]);

        assert_ne!(first.animation_style(), second.animation_style());
        assert_ne!(first.glyphs, second.glyphs);
    }

    #[test]
    fn a_malformed_accent_falls_back_rather_than_producing_broken_css() {
        assert_eq!(Accent::parse("1 2"), None);
        assert_eq!(Accent::parse("1 2 3 4"), None);
        assert_eq!(Accent::parse("no such colour"), None);
        assert_eq!(Accent::brightened("oops"), "rgb(220 230 255)");
        assert_eq!(Accent::darkened("oops", 42), "20 24 33");
    }
}
