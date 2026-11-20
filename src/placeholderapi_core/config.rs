//! PlaceholderAPI's `config.yml` (`plugins/PlaceholderAPI/config.yml` in the original, same path here).
//!
//! The keys are the original's: `check_updates`, `cloud_enabled`, `cloud_sorting`, `boolean.true`,
//! `boolean.false`, `date_format`, `detect_malicious_expansions`, `use_adventure_provided_replacer`, `debug`.
//! Only `boolean.*`, `date_format` and `debug` change anything here (expansions read the first two through
//! the public API); the others are read and kept so the file round-trips, because there is no eCloud, no
//! jar loading and no adventure replacer on Horizon.

/// The file written when none exists: the original's `config.yml`, without the version placeholder.
pub const DEFAULT_FILE: &str = "# PlaceholderAPI\n\
# Expansions: https://placeholderapi.com/ecloud\n\
# Wiki: https://wiki.placeholderapi.com/\n\
# No placeholders are provided with this plugin by default.\n\
check_updates: true\n\
cloud_enabled: true\n\
cloud_sorting: \"name\"\n\
boolean:\n\
\x20 'true': 'yes'\n\
\x20 'false': 'no'\n\
date_format: MM/dd/yy HH:mm:ss\n\
detect_malicious_expansions: true\n\
use_adventure_provided_replacer: false\n\
debug: false\n";

/// The values of the file. The defaults are the ones the original's `PlaceholderAPIConfig` falls back to
/// when a key is missing (`boolean.true` is `true` then, not `yes`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub check_updates: bool,
    pub cloud_enabled: bool,
    pub cloud_sorting: String,
    pub boolean_true: String,
    pub boolean_false: String,
    pub date_format: String,
    pub detect_malicious_expansions: bool,
    pub use_adventure_provided_replacer: bool,
    pub debug: bool,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            check_updates: true,
            cloud_enabled: true,
            cloud_sorting: "name".to_owned(),
            boolean_true: "true".to_owned(),
            boolean_false: "false".to_owned(),
            date_format: "MM/dd/yy HH:mm:ss".to_owned(),
            detect_malicious_expansions: true,
            use_adventure_provided_replacer: false,
            debug: false,
        }
    }
}

/// Drops a trailing comment (a `#` after whitespace, outside quotes) and the quotes around a scalar.
fn scalar(raw: &str) -> String {
    let raw = raw.trim();
    let mut quote: Option<char> = None;
    let mut cut = raw.len();
    let mut prev_space = true;
    for (i, c) in raw.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None if c == '#' && prev_space => {
                cut = i;
                break;
            }
            None => {}
        }
        prev_space = c.is_whitespace();
    }
    let value = raw[..cut].trim();
    for q in ['\'', '"'] {
        if value.len() >= 2 && value.starts_with(q) && value.ends_with(q) {
            return value[1..value.len() - 1].to_owned();
        }
    }
    value.to_owned()
}

fn flag(value: &str, default: bool) -> bool {
    match value.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => true,
        "false" | "no" | "off" => false,
        _ => default,
    }
}

impl Config {
    /// Parses the subset of YAML the file uses: `key: value` lines and one level of nesting.
    /// Unknown keys are ignored, a bad value keeps the default.
    pub fn parse(text: &str) -> Config {
        let mut config = Config::default();
        let mut section = String::new();
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let nested = line.starts_with(' ') || line.starts_with('\t');
            let Some((key, value)) = trimmed.split_once(':') else {
                continue;
            };
            let key = scalar(key);
            let value = scalar(value);
            if !nested {
                section.clear();
                if value.is_empty() {
                    section = key;
                    continue;
                }
                match key.as_str() {
                    "check_updates" => config.check_updates = flag(&value, config.check_updates),
                    "cloud_enabled" => config.cloud_enabled = flag(&value, config.cloud_enabled),
                    "cloud_sorting" => config.cloud_sorting = value,
                    "date_format" => config.date_format = value,
                    "detect_malicious_expansions" => {
                        config.detect_malicious_expansions = flag(&value, config.detect_malicious_expansions)
                    }
                    "use_adventure_provided_replacer" => {
                        config.use_adventure_provided_replacer = flag(&value, config.use_adventure_provided_replacer)
                    }
                    "debug" => config.debug = flag(&value, config.debug),
                    _ => {}
                }
            } else if section == "boolean" {
                match key.as_str() {
                    "true" => config.boolean_true = value,
                    "false" => config.boolean_false = value,
                    _ => {}
                }
            }
        }
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_file_reads_back_with_the_original_values() {
        let config = Config::parse(DEFAULT_FILE);
        assert_eq!(config.boolean_true, "yes");
        assert_eq!(config.boolean_false, "no");
        assert_eq!(config.date_format, "MM/dd/yy HH:mm:ss");
        assert_eq!(config.cloud_sorting, "name");
        assert!(config.check_updates && config.cloud_enabled && config.detect_malicious_expansions);
        assert!(!config.debug && !config.use_adventure_provided_replacer);
    }

    #[test]
    fn a_missing_key_falls_back_like_the_original() {
        let config = Config::parse("debug: true\n");
        assert!(config.debug);
        assert_eq!(config.boolean_true, "true");
        assert_eq!(config.boolean_false, "false");
    }

    #[test]
    fn comments_and_quotes_are_dropped() {
        let config = Config::parse("date_format: 'dd.MM.yyyy' # day first\nboolean:\n  'true': \"on\" # yes\n");
        assert_eq!(config.date_format, "dd.MM.yyyy");
        assert_eq!(config.boolean_true, "on");
    }

    #[test]
    fn a_bad_flag_keeps_the_default() {
        assert!(Config::parse("check_updates: maybe\n").check_updates);
        assert!(!Config::parse("check_updates: no\n").check_updates);
    }
}
