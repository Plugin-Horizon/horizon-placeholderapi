//! `&` colour codes, as Bukkit's `ChatColor.translateAlternateColorCodes('&', ...)` reads them, which
//! PlaceholderAPI's `Msg.color` uses for its own messages. Placeholder values are not coloured by
//! PlaceholderAPI and are not here either: `set_placeholders` returns the `&` codes it was given.

/// The colour of each code, as RGB.
const COLOURS: [(char, u32); 16] = [
    ('0', 0x000000),
    ('1', 0x0000AA),
    ('2', 0x00AA00),
    ('3', 0x00AAAA),
    ('4', 0xAA0000),
    ('5', 0xAA00AA),
    ('6', 0xFFAA00),
    ('7', 0xAAAAAA),
    ('8', 0x555555),
    ('9', 0x5555FF),
    ('a', 0x55FF55),
    ('b', 0x55FFFF),
    ('c', 0xFF5555),
    ('d', 0xFF55FF),
    ('e', 0xFFFF55),
    ('f', 0xFFFFFF),
];

/// The codes Bukkit accepts after `&`: colours, formats and reset (upper or lower case).
const CODES: &str = "0123456789AaBbCcDdEeFfKkLlMmNnOoRr";

/// A run of text in one colour.
#[derive(Clone, Debug, PartialEq)]
pub struct Segment {
    pub text: String,
    pub rgb: u32,
}

/// Splits `s` at its colour codes. Formats (`&k` to `&o`) do not change the colour and are dropped;
/// `&r` returns to `default`. An `&` that does not start a code stays as text.
pub fn segments(s: &str, default: u32) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut rgb = default;
    let mut cur = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        let code = match (c, chars.peek()) {
            ('&', Some(&k)) if CODES.contains(k) => Some(k),
            _ => None,
        };
        let Some(code) = code else {
            cur.push(c);
            continue;
        };
        chars.next();
        let lower = code.to_ascii_lowercase();
        let next_rgb = if lower == 'r' {
            Some(default)
        } else {
            COLOURS.iter().find(|(k, _)| *k == lower).map(|(_, v)| *v)
        };
        if let Some(next) = next_rgb
            && next != rgb
        {
            if !cur.is_empty() {
                out.push(Segment { text: std::mem::take(&mut cur), rgb });
            }
            rgb = next;
        }
    }
    if !cur.is_empty() {
        out.push(Segment { text: cur, rgb });
    }
    out
}

/// The text with its colour codes removed.
pub fn plain(s: &str) -> String {
    segments(s, 0).into_iter().map(|seg| seg.text).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: u32 = 0xFFFFFF;

    fn seg(text: &str, rgb: u32) -> Segment {
        Segment { text: text.to_owned(), rgb }
    }

    #[test]
    fn colour_codes_split_the_text_into_runs() {
        assert_eq!(
            segments("&cHi &athere", WHITE),
            vec![seg("Hi ", 0xFF5555), seg("there", 0x55FF55)]
        );
    }

    #[test]
    fn reset_goes_back_to_the_default_colour() {
        assert_eq!(segments("&cred&rplain", WHITE), vec![seg("red", 0xFF5555), seg("plain", WHITE)]);
    }

    #[test]
    fn formats_are_dropped_and_keep_the_colour() {
        assert_eq!(segments("&c&lbold&o", WHITE), vec![seg("bold", 0xFF5555)]);
    }

    #[test]
    fn upper_case_codes_are_codes_too() {
        assert_eq!(segments("&Cx", WHITE), vec![seg("x", 0xFF5555)]);
    }

    #[test]
    fn an_ampersand_that_starts_no_code_is_text() {
        assert_eq!(segments("a & b &x c", WHITE), vec![seg("a & b &x c", WHITE)]);
        // The first `&` is text (the next character is `&`), the second starts the code `&a`.
        assert_eq!(segments("&&a", WHITE), vec![seg("&", WHITE)]);
    }

    #[test]
    fn the_section_sign_is_not_a_code_here() {
        assert_eq!(segments("§cx", WHITE), vec![seg("§cx", WHITE)]);
    }

    #[test]
    fn a_trailing_code_leaves_no_empty_run() {
        assert_eq!(segments("text&c", WHITE), vec![seg("text", WHITE)]);
        assert_eq!(segments("", WHITE), Vec::<Segment>::new());
    }

    #[test]
    fn plain_removes_the_codes_only() {
        assert_eq!(plain("&aA&rB&l&4C"), "ABC");
        assert_eq!(plain("&&a"), "&");
        assert_eq!(plain("no codes"), "no codes");
    }
}
