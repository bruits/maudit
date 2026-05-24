use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use axum::http::{HeaderMap, HeaderName, HeaderValue};
use tracing::warn;

const MAX_LINE_LEN: usize = 2000;
const MAX_RULES: usize = 100;

#[derive(Debug)]
pub struct HeadersFile {
    path: PathBuf,
    rules: RwLock<Vec<Rule>>,
}

#[derive(Debug, Clone)]
struct Rule {
    pattern: Pattern,
    entries: Vec<HeaderEntry>,
}

#[derive(Debug, Clone)]
struct HeaderEntry {
    name: HeaderName,
    /// Original case for diagnostic output. `HeaderName` lower-cases.
    raw_name: String,
    value: String,
    detach: bool,
}

/// Parsed path pattern. We pre-tokenise into segments separated by `/` so the
/// matcher is a tight per-segment loop instead of a regex.
#[derive(Debug, Clone)]
struct Pattern {
    segments: Vec<Segment>,
    trailing_splat: bool,
}

#[derive(Debug, Clone)]
enum Segment {
    /// A literal string, must match exactly.
    Literal(String),
    /// `*`: matches any single path segment, captured as `:splat`.
    Splat,
    /// `:name`: matches any single path segment, captured under `name`.
    Placeholder(String),
}

impl HeadersFile {
    /// Missing file is treated as empty. A `_headers` file is opt-in.
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("_headers");
        let rules = read_rules(&path);
        Self {
            path,
            rules: RwLock::new(rules),
        }
    }

    /// Call after a build refreshes `dist/_headers`.
    pub fn reload(&self) {
        let new_rules = read_rules(&self.path);
        *self.rules.write().expect("headers lock poisoned") = new_rules;
    }

    pub fn is_empty(&self) -> bool {
        self.rules.read().expect("headers lock poisoned").is_empty()
    }

    /// Multiple matching rules contribute; same-name values are joined with a
    /// comma, mirroring Cloudflare's behaviour. Detach entries (`!`) remove
    /// matching headers from the accumulated set, regardless of source.
    pub fn headers_for(&self, path: &str) -> HeaderMap {
        // Use a name-keyed accumulator so we can comma-join repeated entries
        // and apply detaches in one pass after the loop.
        let mut accum: HashMap<HeaderName, AccumValue> = HashMap::new();

        let rules = self.rules.read().expect("headers lock poisoned");
        for rule in rules.iter() {
            let Some(captures) = rule.pattern.match_path(path) else {
                continue;
            };
            for entry in &rule.entries {
                if entry.detach {
                    accum.insert(entry.name.clone(), AccumValue::Detached);
                    continue;
                }
                let value = substitute_placeholders(&entry.value, &captures);
                accum
                    .entry(entry.name.clone())
                    .and_modify(|existing| existing.push(&value))
                    .or_insert_with(|| AccumValue::Set(value));
            }
        }
        drop(rules);

        let mut out = HeaderMap::new();
        for (name, value) in accum {
            let AccumValue::Set(joined) = value else {
                continue;
            };
            match HeaderValue::from_str(&joined) {
                Ok(v) => {
                    out.insert(name, v);
                }
                Err(e) => warn!("Skipping header {name} with invalid value: {e}"),
            }
        }
        out
    }
}

fn read_rules(path: &Path) -> Vec<Rule> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    match parse(&content) {
        Ok(rules) => rules,
        Err(e) => {
            warn!("Failed to parse {}: {e}", path.display());
            Vec::new()
        }
    }
}

enum AccumValue {
    Set(String),
    Detached,
}

impl AccumValue {
    fn push(&mut self, addition: &str) {
        if let AccumValue::Set(existing) = self {
            existing.push_str(", ");
            existing.push_str(addition);
        }
        // Detached stays detached: order of rules matters in the spec.
    }
}

fn parse(content: &str) -> Result<Vec<Rule>, String> {
    let mut rules: Vec<Rule> = Vec::new();
    let mut current: Option<Rule> = None;

    for (lineno, raw_line) in content.lines().enumerate() {
        let lineno = lineno + 1;

        if raw_line.len() > MAX_LINE_LEN {
            warn!("_headers line {lineno} exceeds {MAX_LINE_LEN} chars; skipping");
            continue;
        }

        // Strip trailing comment introduced by `#` only when it's the start of
        // the line (Cloudflare doesn't define inline comments). Keep `#` inside
        // header values intact.
        let trimmed_left = raw_line.trim_start();
        if trimmed_left.is_empty() || trimmed_left.starts_with('#') {
            if let Some(rule) = current.take() {
                rules.push(rule);
            }
            continue;
        }

        let is_indented = raw_line.starts_with(char::is_whitespace);

        if is_indented {
            let Some(rule) = current.as_mut() else {
                warn!(
                    "_headers line {lineno}: indented header line with no preceding URL pattern; skipping"
                );
                continue;
            };
            match parse_header_line(trimmed_left) {
                Ok(entry) => rule.entries.push(entry),
                Err(e) => warn!("_headers line {lineno}: {e}"),
            }
        } else {
            if let Some(rule) = current.take() {
                rules.push(rule);
            }
            if rules.len() >= MAX_RULES {
                warn!("_headers exceeds {MAX_RULES} rules; ignoring the rest");
                break;
            }
            match parse_pattern(trimmed_left) {
                Ok(pattern) => {
                    current = Some(Rule {
                        pattern,
                        entries: Vec::new(),
                    });
                }
                Err(e) => warn!("_headers line {lineno}: invalid pattern `{trimmed_left}`: {e}"),
            }
        }
    }
    if let Some(rule) = current.take() {
        rules.push(rule);
    }

    // Drop rules with no headers.
    rules.retain(|r| !r.entries.is_empty());
    Ok(rules)
}

fn parse_header_line(line: &str) -> Result<HeaderEntry, String> {
    let (name_part, value_part, detach) = if let Some(rest) = line.strip_prefix('!') {
        // `! Header-Name`: detach. No value expected, tolerate one anyway.
        let rest = rest.trim_start();
        let (n, v) = match rest.split_once(':') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => (rest.trim(), ""),
        };
        (n, v, true)
    } else {
        let (n, v) = line
            .split_once(':')
            .ok_or_else(|| format!("expected `Name: Value`, got `{line}`"))?;
        (n.trim(), v.trim(), false)
    };

    if name_part.is_empty() {
        return Err("empty header name".into());
    }
    let name = HeaderName::from_bytes(name_part.as_bytes())
        .map_err(|e| format!("invalid header name `{name_part}`: {e}"))?;

    Ok(HeaderEntry {
        name,
        raw_name: name_part.to_string(),
        value: value_part.to_string(),
        detach,
    })
}

fn parse_pattern(raw: &str) -> Result<Pattern, String> {
    // Strip `https://host` prefix if present. We ignore the authority.
    let path_part = if let Some(rest) = raw.strip_prefix("https://") {
        match rest.find('/') {
            Some(slash) => &rest[slash..],
            None => "/",
        }
    } else if raw.starts_with("http://") {
        return Err("absolute URLs must use https://".into());
    } else {
        raw
    };

    if !path_part.starts_with('/') {
        return Err("path must start with `/`".into());
    }

    // Detect a trailing splat (`/*`). We treat it specially so it can match
    // multiple segments, unlike a mid-path `*` which matches one segment.
    let (path_part, trailing_splat) = match path_part.strip_suffix("/*") {
        Some(stripped) if !stripped.is_empty() => (stripped, true),
        Some(_) => ("/", true), // pattern was `/*`
        None => (path_part, false),
    };

    let mut segments = Vec::new();
    let mut splat_count = 0;
    if trailing_splat {
        // Counted as a splat for the "single splat per URL" rule.
        splat_count += 1;
    }
    for segment in path_part.split('/').skip(1) {
        // `path_part` starts with `/`, so first split segment is "".
        if segment.is_empty() {
            // Treat consecutive `//` as `/` for forgiveness.
            continue;
        }
        if segment == "*" {
            splat_count += 1;
            segments.push(Segment::Splat);
        } else if let Some(name) = segment.strip_prefix(':') {
            if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(format!("invalid placeholder `{segment}`"));
            }
            if name == "splat" {
                return Err("`:splat` is reserved for splat captures".into());
            }
            segments.push(Segment::Placeholder(name.to_string()));
        } else {
            segments.push(Segment::Literal(segment.to_string()));
        }
    }

    if splat_count > 1 {
        return Err("only one `*` is allowed per pattern".into());
    }

    Ok(Pattern {
        segments,
        trailing_splat,
    })
}

impl Pattern {
    fn match_path<'a>(&self, path: &'a str) -> Option<Captures<'a>> {
        let path = path.split('?').next().unwrap_or(path); // ignore query string
        let mut path_segs = path.split('/').skip(1).filter(|s| !s.is_empty());
        let mut captures = Captures::default();

        for segment in &self.segments {
            let piece = path_segs.next()?;
            match segment {
                Segment::Literal(lit) => {
                    if lit != piece {
                        return None;
                    }
                }
                Segment::Splat => {
                    captures.splat = Some(piece.to_string());
                }
                Segment::Placeholder(name) => {
                    captures.named.insert(name.clone(), piece.to_string());
                }
            }
        }

        if self.trailing_splat {
            let rest: Vec<&str> = path_segs.collect();
            captures.splat = Some(rest.join("/"));
        } else if path_segs.next().is_some() {
            // More path than pattern, no trailing splat: no match.
            return None;
        }

        Some(captures)
    }
}

#[derive(Debug, Default)]
struct Captures<'a> {
    splat: Option<String>,
    named: std::collections::HashMap<String, String>,
    _marker: std::marker::PhantomData<&'a ()>,
}

fn substitute_placeholders(value: &str, captures: &Captures) -> String {
    // Replace `:splat` / `:name` references with their captures.
    let bytes = value.as_bytes();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b':' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_alphabetic() {
            let start = i + 1;
            let mut end = start;
            while end < bytes.len() {
                let c = bytes[end] as char;
                if c.is_alphanumeric() || c == '_' {
                    end += 1;
                } else {
                    break;
                }
            }
            let name = &value[start..end];
            let replacement = if name == "splat" {
                captures.splat.as_deref()
            } else {
                captures.named.get(name).map(String::as_str)
            };
            match replacement {
                Some(r) => out.push_str(r),
                None => out.push_str(&value[i..end]), // leave the literal `:name` if unbound
            }
            i = end;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

// Silence unused warning on `raw_name` while letting it stay around for future
// diagnostics surface.
#[allow(dead_code)]
fn _touch_raw_name(e: &HeaderEntry) -> &str {
    &e.raw_name
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> HeadersFile {
        HeadersFile {
            path: PathBuf::new(),
            rules: RwLock::new(parse(s).unwrap()),
        }
    }

    #[test]
    fn simple_match() {
        let f = parse_ok(
            r#"
/secure/page
  X-Frame-Options: DENY
"#,
        );
        let h = f.headers_for("/secure/page");
        assert_eq!(h.get("x-frame-options").unwrap(), "DENY");
    }

    #[test]
    fn trailing_splat_matches_subpaths() {
        let f = parse_ok(
            r#"
/static/*
  X-Robots-Tag: nosnippet
"#,
        );
        assert!(!f.headers_for("/static/a/b/c.png").is_empty());
        assert!(f.headers_for("/other/x.png").is_empty());
    }

    #[test]
    fn comma_join_repeated_headers() {
        let f = parse_ok(
            r#"
/*
  X-Robots-Tag: noindex
/static/*
  X-Robots-Tag: nosnippet
"#,
        );
        let h = f.headers_for("/static/styles.css");
        assert_eq!(h.get("x-robots-tag").unwrap(), "noindex, nosnippet");
    }

    #[test]
    fn detach_strips_header() {
        let f = parse_ok(
            r#"
/*
  Content-Security-Policy: default-src 'self';
/photo.jpg
  ! Content-Security-Policy
"#,
        );
        let h = f.headers_for("/photo.jpg");
        assert!(h.get("content-security-policy").is_none());
    }

    #[test]
    fn placeholder_substitution_in_value() {
        let f = parse_ok(
            r#"
/movies/:title
  X-Movie-Name: You are watching ":title"
"#,
        );
        let h = f.headers_for("/movies/dune");
        assert_eq!(h.get("x-movie-name").unwrap(), "You are watching \"dune\"");
    }

    #[test]
    fn coop_coep_only_on_root() {
        let f = parse_ok(
            r#"
/*
  Cross-Origin-Opener-Policy: same-origin
  Cross-Origin-Embedder-Policy: require-corp
"#,
        );
        let h = f.headers_for("/anywhere");
        assert_eq!(h.get("cross-origin-opener-policy").unwrap(), "same-origin");
        assert_eq!(
            h.get("cross-origin-embedder-policy").unwrap(),
            "require-corp"
        );
    }
}
