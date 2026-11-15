//! The flat JSON objects of `params` columns: read keeping each value's text, so a row written
//! back with unchanged values is unchanged, and written in a fixed style.

use std::collections::BTreeMap;

/// One value of a row's `params` object.
#[derive(Clone, Debug, PartialEq)]
pub enum Param {
    /// A number.
    Number(f64),
    /// A string (colours, definition keys).
    Text(String),
}

/// A `params` object: its entries in written order, each with the text it was written as.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Params {
    /// Key, value and the value's source text.
    pub entries: Vec<(String, Param, String)>,
}

/// Reads a flat JSON object of numbers, strings and booleans (booleans read as 0 and 1).
/// Nested values and malformed text end the reading; what was read so far is kept.
pub fn parse_params(text: &str) -> BTreeMap<String, Param> {
    Params::parse(text)
        .entries
        .into_iter()
        .map(|(k, v, _)| (k, v))
        .collect()
}

impl Params {
    /// Reads `text` as [`parse_params`] does, keeping the order and source text of values.
    pub fn parse(text: &str) -> Self {
        let mut out = Self::default();
        let chars: Vec<char> = text.chars().collect();
        let mut at = 0;
        let skip = |at: &mut usize| {
            while *at < chars.len() && (chars[*at].is_whitespace() || chars[*at] == ',') {
                *at += 1;
            }
        };
        let string = |at: &mut usize| -> Option<String> {
            if chars.get(*at) != Some(&'"') {
                return None;
            }
            *at += 1;
            let mut s = String::new();
            while let Some(&c) = chars.get(*at) {
                *at += 1;
                match c {
                    '"' => return Some(s),
                    '\\' => {
                        s.push(*chars.get(*at)?);
                        *at += 1;
                    }
                    _ => s.push(c),
                }
            }
            None
        };
        skip(&mut at);
        if chars.get(at) != Some(&'{') {
            return out;
        }
        at += 1;
        loop {
            skip(&mut at);
            let Some(key) = string(&mut at) else {
                break;
            };
            skip(&mut at);
            if chars.get(at) != Some(&':') {
                break;
            }
            at += 1;
            skip(&mut at);
            let start = at;
            let value = match chars.get(at) {
                Some('"') => string(&mut at).map(Param::Text),
                Some('t') if text_at(&chars, at, "true") => {
                    at += 4;
                    Some(Param::Number(1.0))
                }
                Some('f') if text_at(&chars, at, "false") => {
                    at += 5;
                    Some(Param::Number(0.0))
                }
                Some(_) => {
                    while at < chars.len() && "+-.eE0123456789".contains(chars[at]) {
                        at += 1;
                    }
                    chars[start..at]
                        .iter()
                        .collect::<String>()
                        .parse()
                        .ok()
                        .map(Param::Number)
                }
                None => None,
            };
            let Some(value) = value else {
                break;
            };
            let raw: String = chars[start..at].iter().collect();
            out.entries.push((key, value, raw));
        }
        out
    }

    /// The value of `key`.
    pub fn get(&self, key: &str) -> Option<&Param> {
        self.entries
            .iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, v, _)| v)
    }

    /// The number at `key`.
    pub fn number(&self, key: &str) -> Option<f64> {
        match self.get(key)? {
            Param::Number(n) => Some(*n),
            Param::Text(_) => None,
        }
    }

    /// The text at `key`.
    pub fn text(&self, key: &str) -> Option<&str> {
        match self.get(key)? {
            Param::Text(s) => Some(s),
            Param::Number(_) => None,
        }
    }

    /// Writes the object compactly: `{"k":v,...}` in entry order with each value's text.
    pub fn to_json(&self) -> String {
        let mut out = String::from("{");
        for (i, (key, _, raw)) in self.entries.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&json_string(key));
            out.push(':');
            out.push_str(raw);
        }
        out.push('}');
        out
    }

    /// Sets `key` to `value`: an existing entry keeps its place and, when the value is the same
    /// (to 1/10000), its text; a new entry goes last.
    pub fn set(&mut self, key: &str, value: Param) {
        let raw = match &value {
            Param::Number(n) => number_text(*n),
            Param::Text(s) => json_string(s),
        };
        match self.entries.iter_mut().find(|(k, _, _)| k == key) {
            Some(entry) => {
                if !same(&entry.1, &value) {
                    entry.1 = value;
                    entry.2 = raw;
                }
            }
            None => self.entries.push((key.to_owned(), value, raw)),
        }
    }

    /// Removes `key`.
    pub fn remove(&mut self, key: &str) {
        self.entries.retain(|(k, _, _)| k != key);
    }
}

/// Whether two values are the same, numbers to 1/10000.
pub fn same(a: &Param, b: &Param) -> bool {
    match (a, b) {
        (Param::Number(x), Param::Number(y)) => (x - y).abs() < 5e-5,
        (Param::Text(x), Param::Text(y)) => x == y,
        _ => false,
    }
}

/// A number rounded to 1/10000 in its shortest text (`1`, `-0.25`).
pub fn number_text(value: f64) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    format!("{rounded}")
}

/// `text` as a JSON string.
pub fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A `#rrggbb` colour as RGB 0..1.
pub fn hex_colour(text: &str) -> Option<[f32; 3]> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let channel = |i: usize| {
        u8::from_str_radix(hex.get(i..i + 2)?, 16)
            .ok()
            .map(|c| f32::from(c) / 255.0)
    };
    Some([channel(0)?, channel(2)?, channel(4)?])
}

fn text_at(chars: &[char], at: usize, word: &str) -> bool {
    word.chars()
        .enumerate()
        .all(|(i, c)| chars.get(at + i) == Some(&c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn params_read_numbers_strings_and_colours() {
        let params = parse_params(
            r##"{"base_colour":"#2d5a27", "scale": 0.5, "count":4,"on":true, "shore_material":"mat.sand", "x": -1.5e1}"##,
        );
        assert_eq!(params["scale"], Param::Number(0.5));
        assert_eq!(params["count"], Param::Number(4.0));
        assert_eq!(params["on"], Param::Number(1.0));
        assert_eq!(params["x"], Param::Number(-15.0));
        assert_eq!(params["shore_material"], Param::Text("mat.sand".into()));
        assert_eq!(hex_colour("#ff8000"), Some([1.0, 128.0 / 255.0, 0.0]));
        assert!(parse_params("not json").is_empty());
        assert!(parse_params("{}").is_empty());
    }

    #[test]
    fn unchanged_values_keep_their_text_and_place() {
        let text = r##"{"y":0.95,"x":-1.0,"tint":"#8fc04a","w":2}"##;
        let mut params = Params::parse(text);
        assert_eq!(params.to_json(), text);
        params.set("x", Param::Number(-1.00001));
        params.set("w", Param::Number(2.5));
        params.set("h", Param::Number(1.0 / 3.0));
        params.remove("tint");
        assert_eq!(
            params.to_json(),
            r#"{"y":0.95,"x":-1.0,"w":2.5,"h":0.3333}"#
        );
    }
}
