use std::borrow::Cow;
use std::fmt;

#[derive(Debug)]
pub enum ErrorKind {
    MissingEquals,
    BadKey,
    UnterminatedQuote,
    BadValue(String),
}

#[derive(Debug)]
pub struct ParseError {
    pub line: usize,
    pub kind: ErrorKind,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {:?}", self.line, self.kind)
    }
}
impl std::error::Error for ParseError {}

pub struct Entry<'a> {
    pub line: usize,
    pub key: &'a str,
    pub value: Cow<'a, str>,
}

/// Yields one entry per `KEY=VALUE` line. Skips blanks and `#` comments.
pub fn parse(src: &str) -> impl Iterator<Item = Result<Entry<'_>, ParseError>> + '_ {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src); // Notepad BOM
    src.lines().enumerate().filter_map(|(i, raw)| {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }
        let line_no = i + 1;
        Some(match parse_line(line) {
            Ok((key, value)) => Ok(Entry {
                line: line_no,
                key,
                value,
            }),
            Err(kind) => Err(ParseError {
                line: line_no,
                kind,
            }),
        })
    })
}

fn parse_line(line: &str) -> Result<(&str, Cow<'_, str>), ErrorKind> {
    let line = match line.strip_prefix("export ") {
        Some(rest) => rest.trim_start(),
        None => line,
    };
    let (key, rest) = line.split_once('=').ok_or(ErrorKind::MissingEquals)?;
    let key = key.trim_end();
    let valid = !key.is_empty()
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-');
    if !valid {
        return Err(ErrorKind::BadKey);
    }
    Ok((key, parse_value(rest.trim_start())?))
}

fn parse_value(v: &str) -> Result<Cow<'_, str>, ErrorKind> {
    match v.as_bytes().first() {
        Some(b'"') => parse_double_quoted(&v[1..]),
        Some(b'\'') => {
            let end = v[1..].find('\'').ok_or(ErrorKind::UnterminatedQuote)?;
            Ok(Cow::Borrowed(&v[1..1 + end])) // literal, no escapes
        }
        _ => {
            let v = v.find(" #").map_or(v, |i| &v[..i]); // inline comment
            Ok(Cow::Borrowed(v.trim_end()))
        }
    }
}

fn parse_double_quoted(s: &str) -> Result<Cow<'_, str>, ErrorKind> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'"' => return Ok(Cow::Borrowed(&s[..i])), // fast path: no escapes
            b'\\' => break,
            _ => i += 1,
        }
    }
    if i == bytes.len() {
        return Err(ErrorKind::UnterminatedQuote);
    }
    // slow path: has escapes
    let mut out = String::with_capacity(s.len());
    out.push_str(&s[..i]);
    let mut chars = s[i..].chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Ok(Cow::Owned(out)),
            '\\' => match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => return Err(ErrorKind::UnterminatedQuote),
            },
            c => out.push(c),
        }
    }
    Err(ErrorKind::UnterminatedQuote)
}

// ---- mapping into a struct ----

#[derive(Debug, Default)]
pub struct Config {
    pub wallpaper_dir: String,
    pub thumb_width: u32,
    pub animate: bool,
}

fn parse_bool(s: &str) -> Option<bool> {
    if ["1", "true", "yes", "on"]
        .iter()
        .any(|t| s.eq_ignore_ascii_case(t))
    {
        Some(true)
    } else if ["0", "false", "no", "off"]
        .iter()
        .any(|t| s.eq_ignore_ascii_case(t))
    {
        Some(false)
    } else {
        None
    }
}

impl Config {
    pub fn from_env_str(src: &str) -> Result<Self, ParseError> {
        let mut cfg = Config::default();
        for entry in parse(src) {
            let Entry { line, key, value } = entry?;
            let bad = || ParseError {
                line,
                kind: ErrorKind::BadValue(key.to_string()),
            };
            match key {
                "WALLPAPER_DIR" => cfg.wallpaper_dir = value.into_owned(),
                "THUMB_WIDTH" => cfg.thumb_width = value.parse().map_err(|_| bad())?,
                "ANIMATE" => cfg.animate = parse_bool(&value).ok_or_else(bad)?,
                _ => {} // unknown keys ignored; return an error here if you'd rather
            }
        }
        Ok(cfg)
    }
}

#[test]
fn basic() {
    let cfg = Config::from_env_str(
        "# c\nexport THUMB_WIDTH=520\nANIMATE=yes # hi\nWALLPAPER_DIR='C:\\new\\pics'\n",
    )
    .unwrap();
    assert_eq!(cfg.thumb_width, 520);
    assert!(cfg.animate);
    assert_eq!(cfg.wallpaper_dir, r"C:\new\pics");
}
