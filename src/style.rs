use std::io::{self, Write};

use crossterm::style::{
    Attribute, Color, ContentStyle, ResetColor, SetAttribute, SetBackgroundColor,
    SetForegroundColor,
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Style {
    pub foreground: Option<Color>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub strike: bool,
}

impl Style {
    pub const fn color(color: Color) -> Self {
        Self {
            foreground: Some(color),
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            strike: false,
        }
    }

    pub fn content(self, highlight: bool) -> ContentStyle {
        let mut style = ContentStyle {
            foreground_color: self.foreground,
            ..ContentStyle::default()
        };
        for (enabled, attribute) in [
            (self.bold, Attribute::Bold),
            (self.italic, Attribute::Italic),
            (self.underline, Attribute::Underlined),
            (self.dim, Attribute::Dim),
            (self.strike, Attribute::CrossedOut),
        ] {
            if enabled {
                style.attributes.set(attribute);
            }
        }
        if highlight {
            style.background_color = Some(Color::DarkYellow);
            style.foreground_color = Some(Color::Black);
            style.attributes.set(Attribute::Bold);
        }
        style
    }
}

struct StyledWriter<'a, W> {
    out: &'a mut W,
    active: ContentStyle,
}

impl<W: Write> StyledWriter<'_, W> {
    fn piece(&mut self, text: &str, style: ContentStyle) -> io::Result<()> {
        if text.is_empty() {
            return Ok(());
        }
        if self.active == style {
            return self.out.write_all(text.as_bytes());
        }
        if self.active.attributes != style.attributes {
            // Disabling attributes individually is subtle (bold and dim share
            // a reset). Reset once, then establish the complete new style.
            if self.active != ContentStyle::default() {
                write!(self.out, "{}", ResetColor)?;
            }
            if let Some(color) = style.background_color {
                write!(self.out, "{}", SetBackgroundColor(color))?;
            }
            if let Some(color) = style.foreground_color {
                write!(self.out, "{}", SetForegroundColor(color))?;
            }
            for attribute in [
                Attribute::Bold,
                Attribute::Italic,
                Attribute::Underlined,
                Attribute::Dim,
                Attribute::CrossedOut,
            ] {
                if style.attributes.has(attribute) {
                    write!(self.out, "{}", SetAttribute(attribute))?;
                }
            }
        } else {
            if self.active.foreground_color != style.foreground_color {
                write!(
                    self.out,
                    "{}",
                    SetForegroundColor(style.foreground_color.unwrap_or(Color::Reset))
                )?;
            }
            if self.active.background_color != style.background_color {
                write!(
                    self.out,
                    "{}",
                    SetBackgroundColor(style.background_color.unwrap_or(Color::Reset))
                )?;
            }
        }
        self.active = style;
        self.out.write_all(text.as_bytes())
    }

    fn finish(self) -> io::Result<()> {
        if self.active != ContentStyle::default() {
            write!(self.out, "{}", ResetColor)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct Span {
    pub text: String,
    pub style: Style,
}

#[derive(Clone, Debug, Default)]
pub struct Line {
    pub spans: Vec<Span>,
    pub width: usize,
    /// Approximate byte offset in the original Markdown, used for resize anchoring.
    pub source: usize,
}

impl Line {
    pub fn push(&mut self, text: &str, style: Style) {
        self.push_sized(text, style, UnicodeWidthStr::width(text));
    }

    pub(crate) fn push_sized(&mut self, text: &str, style: Style, width: usize) {
        self.push_reserved(text, style, width, text.len());
    }

    pub(crate) fn push_reserved(
        &mut self,
        text: &str,
        style: Style,
        width: usize,
        capacity: usize,
    ) {
        self.width += width;
        if let Some(span) = self.spans.last_mut().filter(|span| span.style == style) {
            if capacity > text.len() {
                span.text.reserve_exact(capacity);
            }
            span.text.push_str(text);
        } else {
            let mut content = String::with_capacity(capacity.max(text.len()));
            content.push_str(text);
            self.spans.push(Span {
                text: content,
                style,
            });
        }
    }

    pub fn plain(&self) -> String {
        self.spans.iter().map(|span| span.text.as_str()).collect()
    }

    /// Emit each span's style so any row can begin a page.
    pub fn write(&self, out: &mut impl Write, color: bool, query: &str) -> io::Result<()> {
        if !color {
            for span in &self.spans {
                out.write_all(span.text.as_bytes())?;
            }
            return Ok(());
        }
        let mut writer = StyledWriter {
            out,
            active: ContentStyle::default(),
        };
        let plain = if query.is_empty() {
            String::new()
        } else {
            self.plain()
        };
        let matches: Vec<_> = if query.is_empty() {
            Vec::new()
        } else {
            plain
                .match_indices(query)
                .map(|(i, _)| i..i + query.len())
                .collect()
        };
        let mut offset = 0;
        let mut next_match = 0;
        for span in &self.spans {
            if matches.is_empty() {
                if span.style == Style::default() && writer.active == ContentStyle::default() {
                    writer.out.write_all(span.text.as_bytes())?;
                } else {
                    writer.piece(&span.text, span.style.content(false))?;
                }
                offset += span.text.len();
                continue;
            }
            let mut start = 0;
            let mut active = false;
            for (index, grapheme) in span.text.grapheme_indices(true) {
                while matches
                    .get(next_match)
                    .is_some_and(|range| range.end <= offset + index)
                {
                    next_match += 1;
                }
                let highlighted = matches
                    .get(next_match)
                    .is_some_and(|range| range.start < offset + index + grapheme.len());
                if highlighted != active {
                    if index > start {
                        writer.piece(&span.text[start..index], span.style.content(active))?;
                    }
                    start = index;
                    active = highlighted;
                }
            }
            writer.piece(&span.text[start..], span.style.content(active))?;
            offset += span.text.len();
        }
        writer.finish()
    }
}

/// Search styled spans without allocating a concatenated string for each row.
pub(crate) struct Search<'a> {
    query: &'a str,
    fallback: Vec<usize>,
}

impl<'a> Search<'a> {
    pub(crate) fn new(query: &'a str) -> Self {
        let bytes = query.as_bytes();
        let mut fallback = vec![0; bytes.len()];
        let mut matched = 0;
        for index in 1..bytes.len() {
            while matched > 0 && bytes[index] != bytes[matched] {
                matched = fallback[matched - 1];
            }
            if bytes[index] == bytes[matched] {
                matched += 1;
            }
            fallback[index] = matched;
        }
        Self { query, fallback }
    }

    pub(crate) fn contains(&self, line: &Line) -> bool {
        if self.query.is_empty() {
            return true;
        }
        if let [span] = line.spans.as_slice() {
            return span.text.contains(self.query);
        }
        let needle = self.query.as_bytes();
        let mut matched = 0;
        for span in &line.spans {
            let mut remaining = span.text.as_bytes();
            while !remaining.is_empty() {
                if matched == 0 {
                    let Some(index) = memchr::memchr(needle[0], remaining) else {
                        break;
                    };
                    remaining = &remaining[index..];
                }
                let byte = remaining[0];
                remaining = &remaining[1..];
                while matched > 0 && byte != needle[matched] {
                    matched = self.fallback[matched - 1];
                }
                if byte == needle[matched] {
                    matched += 1;
                    if matched == needle.len() {
                        return true;
                    }
                }
            }
        }
        false
    }
}

/// Remove control characters and bidi embedding/isolate markers. Keep emoji joiners.
pub fn safe_text(text: &str) -> String {
    text.chars().filter(|&c| !unsafe_char(c)).collect()
}

pub fn unsafe_char(c: char) -> bool {
    c.is_control() || matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

pub fn truncate(text: &str, width: usize) -> String {
    let mut result = String::new();
    let mut used = 0;
    for grapheme in text.graphemes(true) {
        let size = UnicodeWidthStr::width(grapheme);
        if used + size > width {
            break;
        }
        result.push_str(grapheme);
        used += size;
    }
    result
}
