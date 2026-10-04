//! Render Markdown events as styled terminal rows.
use std::borrow::Cow;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::OnceLock;

use crossterm::style::Color;
use pulldown_cmark::{
    Alignment, CodeBlockKind, CowStr, Event, OffsetIter, Options, Parser, Tag, TagEnd,
};
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

use crate::style::{Line, Style, safe_text, truncate, unsafe_char};

const ACCENT: Style = Style::color(Color::Cyan);
const MUTED: Style = Style::color(Color::DarkGrey);
const CODE: Style = Style::color(Color::Yellow);
const HIGHLIGHT_LIMIT: usize = 16 * 1024;

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
static THEMES: OnceLock<ThemeSet> = OnceLock::new();

fn word_width(text: &str, limit: usize) -> usize {
    let ascii = text
        .bytes()
        .take(limit.saturating_add(1))
        .take_while(|byte| (b'!'..=b'~').contains(byte))
        .count();
    if ascii > limit
        || ascii == text.len()
        || (text.as_bytes()[ascii].is_ascii_whitespace()
            && text.as_bytes().get(ascii + 1).is_none_or(u8::is_ascii))
    {
        return ascii;
    }
    // ASCII letters and spaces can share a grapheme with a combining mark.
    let boundary = ascii.saturating_sub(1);
    let mut width = boundary;
    for grapheme in text[boundary..].graphemes(true) {
        if grapheme.chars().all(char::is_whitespace) {
            break;
        }
        width += UnicodeWidthStr::width(grapheme);
        if width > limit {
            break;
        }
    }
    width
}

struct Run<'a> {
    text: CowStr<'a>,
    cursor: usize,
    style: Style,
    literal: bool,
    source: usize,
    exact_offset: bool,
    word_start: bool,
}

impl<'a> Run<'a> {
    fn new(
        text: CowStr<'a>,
        style: Style,
        literal: bool,
        source: usize,
        exact_offset: bool,
    ) -> Self {
        Self {
            text,
            cursor: 0,
            style,
            literal,
            source,
            exact_offset,
            word_start: true,
        }
    }
}

struct CodeInput<'a> {
    text: CowStr<'a>,
    cursor: usize,
    source: usize,
}

struct CodeState {
    highlighter: Option<HighlightLines<'static>>,
    header: bool,
}

struct Table {
    widths: Vec<usize>,
    alignments: Vec<Alignment>,
    cells: Vec<Vec<Line>>,
    cell: usize,
    active: bool,
    source: usize,
}

struct List {
    next: Option<u64>,
    indent: usize,
}

pub struct Renderer<'a> {
    parser: OffsetIter<'a>,
    width: usize,
    highlight: bool,
    runs: VecDeque<Run<'a>>,
    ready: VecDeque<Line>,
    line: Line,
    prefix_width: usize,
    styles: Vec<Style>,
    lists: Vec<List>,
    quotes: usize,
    marker: Option<String>,
    code: Option<CodeState>,
    code_input: Option<CodeInput<'a>>,
    links: Vec<String>,
    table_cell: usize,
    table: Option<Table>,
    position: usize,
    finished: bool,
    blank: bool,
}

impl<'a> Renderer<'a> {
    pub fn new(markdown: &'a str, width: usize, highlight: bool) -> Self {
        let options = Options::ENABLE_TABLES
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_FOOTNOTES;
        Self {
            parser: Parser::new_ext(markdown, options).into_offset_iter(),
            width: width.max(1),
            highlight,
            runs: VecDeque::new(),
            ready: VecDeque::new(),
            line: Line::default(),
            prefix_width: 0,
            styles: vec![Style::default()],
            lists: Vec::new(),
            quotes: 0,
            marker: None,
            code: None,
            code_input: None,
            links: Vec::new(),
            table_cell: 0,
            table: None,
            position: 0,
            finished: false,
            blank: true,
        }
    }

    fn style(&self) -> Style {
        *self.styles.last().expect("base style always present")
    }

    fn push_style(&mut self, change: impl FnOnce(&mut Style)) {
        let mut style = self.style();
        change(&mut style);
        self.styles.push(style);
    }

    fn pop_style(&mut self) {
        if self.styles.len() > 1 {
            self.styles.pop();
        }
    }

    fn run(&mut self, text: CowStr<'a>, style: Style, literal: bool, source: usize, exact: bool) {
        self.runs
            .push_back(Run::new(text, style, literal, source, exact));
    }

    fn generated(&mut self, text: String, style: Style) {
        self.run(text.into(), style, false, self.position, false);
    }

    fn prefix(&mut self) {
        if !self.line.spans.is_empty() {
            return;
        }
        self.line.source = self.position;
        if self.table.as_ref().is_some_and(|table| table.active) {
            self.prefix_width = 0;
            return;
        }
        if self.quotes == 0 && self.lists.is_empty() && self.marker.is_none() {
            self.prefix_width = 0;
            if self.code.as_ref().is_some_and(|code| !code.header) && self.width > 1 {
                let (prefix, width) = if self.width > 2 {
                    ("│ ", 2)
                } else {
                    ("│", 1)
                };
                self.line.push_sized(prefix, MUTED, width);
                self.prefix_width = width;
            }
            return;
        }
        let mut prefix = "│ ".repeat(self.quotes.min(self.width / 2));
        let indent: usize = self.lists.iter().map(|list| list.indent).sum();
        if let Some(marker) = self.marker.take() {
            let preceding = indent.saturating_sub(self.lists.last().map_or(0, |list| list.indent));
            prefix.push_str(&" ".repeat(preceding.min(self.width)));
            prefix.push_str(&marker);
        } else {
            prefix.push_str(&" ".repeat(indent.min(self.width)));
        }
        if self.code.as_ref().is_some_and(|code| !code.header) {
            prefix.push_str("│ ");
        }
        // Deep nesting must leave at least one cell available for content.
        let prefix = truncate(&prefix, self.width.saturating_sub(1));
        self.prefix_width = UnicodeWidthStr::width(prefix.as_str());
        if !prefix.is_empty() {
            self.line.push(&prefix, MUTED);
        }
    }

    fn flush(&mut self, force: bool) {
        if force || self.line.width > self.prefix_width {
            let line = std::mem::take(&mut self.line);
            if let Some(table) = self.table.as_mut().filter(|table| table.active) {
                table.cells[table.cell].push(line);
            } else {
                self.blank = line.spans.is_empty();
                self.ready.push_back(line);
            }
        } else {
            self.line = Line::default();
        }
        self.prefix_width = 0;
    }

    fn gap(&mut self) {
        self.flush(false);
        if !self.blank {
            self.ready.push_back(Line {
                source: self.position,
                ..Line::default()
            });
            self.blank = true;
        }
    }

    /// Consume text up to a row boundary, retaining the run cursor when wrapping.
    fn advance(&mut self) {
        if self
            .runs
            .front()
            .is_some_and(|run| run.cursor == run.text.len())
        {
            self.runs.pop_front();
            return;
        }
        self.prefix();
        let width = self
            .table
            .as_ref()
            .filter(|table| table.active)
            .map_or(self.width, |table| table.widths[table.cell]);
        let run = self.runs.front_mut().expect("nonempty run queue");
        let rest = &run.text[run.cursor..];
        let grapheme = match rest.as_bytes() {
            [b'\r', b'\n', ..] => &rest[..2],
            [first, second, ..] if first.is_ascii() && second.is_ascii() => &rest[..1],
            [_] => rest,
            _ => rest.graphemes(true).next().expect("nonempty run"),
        };
        let source = run.source + if run.exact_offset { run.cursor } else { 0 };
        if self.line.width == self.prefix_width {
            self.line.source = source;
        }
        self.position = source;

        if grapheme == "\n" || grapheme == "\r\n" {
            run.cursor += grapheme.len();
            run.word_start = true;
            if run.literal {
                self.flush(true);
                if let Some(code) = self.code.as_mut() {
                    code.header = false;
                }
            } else if self.line.width > self.prefix_width {
                self.line.push(" ", run.style);
            }
            return;
        }
        if !run.literal && run.word_start && !grapheme.chars().all(char::is_whitespace) {
            // Limit word lookahead to one screen width.
            let capacity = width - self.prefix_width;
            let word_width = word_width(rest, capacity);
            if word_width <= capacity
                && self.line.width > self.prefix_width
                && self.line.width + word_width > width
            {
                self.flush(false);
                return;
            }
        }

        // Printable ASCII has one cell per byte. Batch it without splitting
        // words or the grapheme immediately before a non-ASCII character.
        let available = width.saturating_sub(self.line.width);
        let space = rest.starts_with(' ');
        let mut length = rest
            .bytes()
            .take(available)
            .take_while(|&byte| {
                (b' '..=b'~').contains(&byte) && (run.literal || (byte == b' ') == space)
            })
            .count();
        if rest
            .as_bytes()
            .get(length)
            .is_some_and(|byte| !byte.is_ascii())
        {
            length = length.saturating_sub(1);
        }
        if length > 0 {
            if run.literal || !space || self.line.width > self.prefix_width {
                self.line.push_reserved(
                    &rest[..length],
                    run.style,
                    length,
                    rest.len().min(available),
                );
            }
            run.cursor += length;
            run.word_start = space;
            self.position = source + if run.exact_offset { length - 1 } else { 0 };
            return;
        }

        // Keep one grapheme iterator for a Unicode word fragment instead of
        // restarting it and appending to the span for every character.
        if rest
            .as_bytes()
            .get(grapheme.len())
            .is_some_and(|byte| !byte.is_ascii())
        {
            let mut length = 0;
            let mut cells = 0;
            let mut last = 0;
            let following = rest[grapheme.len()..]
                .grapheme_indices(true)
                .map(|(index, part)| (grapheme.len() + index, part));
            for (index, part) in std::iter::once((0, grapheme)).chain(following) {
                if part.chars().all(char::is_whitespace) || part.chars().any(unsafe_char) {
                    break;
                }
                let size = UnicodeWidthStr::width(part);
                if cells + size > available {
                    break;
                }
                if self.line.width == self.prefix_width && cells == 0 {
                    self.line.source = source + if run.exact_offset { index } else { 0 };
                }
                cells += size;
                length = index + part.len();
                last = index;
                // Let the ASCII fast path handle the next plain byte run.
                if rest.as_bytes().get(length).is_some_and(u8::is_ascii) {
                    break;
                }
            }
            if length > 0 {
                self.line.push_sized(&rest[..length], run.style, cells);
                run.cursor += length;
                run.word_start = false;
                self.position = source + if run.exact_offset { last } else { 0 };
                return;
            }
        }

        let whitespace = grapheme.chars().all(char::is_whitespace);
        let text = if grapheme == "\t" && run.literal {
            Cow::Owned(" ".repeat(4 - (self.line.width - self.prefix_width) % 4))
        } else if grapheme.chars().any(unsafe_char) {
            Cow::Owned(safe_text(grapheme))
        } else if !run.literal && whitespace {
            Cow::Borrowed(" ")
        } else {
            Cow::Borrowed(grapheme)
        };
        let size = UnicodeWidthStr::width(text.as_ref());
        if self.line.width + size > width && self.line.width > self.prefix_width {
            if !run.literal && text == " " {
                run.cursor += grapheme.len();
                run.word_start = true;
            }
            self.flush(false);
            return;
        }
        run.cursor += grapheme.len();
        run.word_start = whitespace;
        if !run.literal && text == " " && self.line.width == self.prefix_width {
            return;
        }
        // Replace text that cannot fit in the available columns.
        if size > width - self.prefix_width {
            self.line.push("�", run.style);
        } else if !text.is_empty() {
            self.line.push_sized(&text, run.style, size);
        }
    }

    fn code_line(&mut self) {
        let input = self.code_input.as_mut().expect("code input present");
        if input.cursor == input.text.len() {
            self.code_input = None;
            return;
        }
        let start = input.cursor;
        let rest = &input.text[start..];
        let mut limit = rest.len().min(HIGHLIGHT_LIMIT);
        while !rest.is_char_boundary(limit) {
            limit -= 1;
        }
        let newline = rest[..limit].find('\n');
        let length = newline.map_or(limit, |index| index + 1);
        let text = &rest[..length];
        let source = input.source + start;
        input.cursor += length;
        let code = self.code.as_mut().expect("code state present");
        if newline.is_none() && length < rest.len() {
            // Skip highlighting for lines over HIGHLIGHT_LIMIT bytes.
            code.highlighter = None;
        }
        let highlighted = code.highlighter.as_mut().and_then(|highlighter| {
            highlighter
                .highlight_line(text, SYNTAXES.get().expect("initialized syntax set"))
                .ok()
        });
        let borrowed = if let CowStr::Borrowed(source) = &input.text {
            Some(*source)
        } else {
            None
        };
        let piece = |text: &str, offset: usize| -> CowStr<'a> {
            if let Some(original) = borrowed {
                CowStr::Borrowed(&original[start + offset..start + offset + text.len()])
            } else {
                text.to_owned().into()
            }
        };
        if let Some(parts) = highlighted {
            let mut offset = 0;
            for (style, text) in parts {
                let style = Style {
                    foreground: Some(Color::Rgb {
                        r: style.foreground.r,
                        g: style.foreground.g,
                        b: style.foreground.b,
                    }),
                    bold: style.font_style.contains(FontStyle::BOLD),
                    italic: style.font_style.contains(FontStyle::ITALIC),
                    underline: style.font_style.contains(FontStyle::UNDERLINE),
                    ..Style::default()
                };
                self.runs.push_back(Run::new(
                    piece(text, offset),
                    style,
                    true,
                    source + offset,
                    true,
                ));
                offset += text.len();
            }
        } else {
            self.runs
                .push_back(Run::new(piece(text, 0), CODE, true, source, true));
        }
    }

    fn table_row(&mut self, header: bool) {
        let table = self.table.as_mut().expect("table present");
        let height = table.cells.iter().map(Vec::len).max().unwrap_or(1).max(1);
        for row in 0..height {
            let mut line = Line {
                source: table.source,
                ..Line::default()
            };
            for (column, &width) in table.widths.iter().enumerate() {
                if column > 0 {
                    line.push(" │ ", MUTED);
                }
                let cell = table.cells[column].get(row);
                let padding = width.saturating_sub(cell.map_or(0, |line| line.width));
                let before = match table.alignments[column] {
                    Alignment::Right => padding,
                    Alignment::Center => padding / 2,
                    _ => 0,
                };
                line.push(&" ".repeat(before), Style::default());
                if let Some(cell) = cell {
                    for span in &cell.spans {
                        line.push(&span.text, span.style);
                    }
                }
                line.push(&" ".repeat(padding - before), Style::default());
            }
            self.ready.push_back(line);
        }
        if header {
            let mut line = Line {
                source: table.source,
                ..Line::default()
            };
            for (column, &width) in table.widths.iter().enumerate() {
                if column > 0 {
                    line.push("─┼─", MUTED);
                }
                line.push(&"─".repeat(width), MUTED);
            }
            self.ready.push_back(line);
        }
        for cell in &mut table.cells {
            cell.clear();
        }
        self.blank = false;
    }

    fn event(&mut self, event: Event<'a>, range: Range<usize>) {
        self.position = self.position.max(range.start);
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => self.flush(false),
                Tag::Heading { level, .. } => {
                    self.gap();
                    self.push_style(|style| {
                        style.foreground = Some(Color::Cyan);
                        style.bold = true;
                    });
                    self.generated(format!("{} ", "#".repeat(level as usize)), self.style());
                }
                Tag::BlockQuote(_) => {
                    self.gap();
                    self.quotes += 1;
                }
                Tag::List(start) => {
                    self.flush(false);
                    self.lists.push(List {
                        next: start,
                        indent: 2,
                    });
                }
                Tag::Item => {
                    self.flush(false);
                    if let Some(list) = self.lists.last_mut() {
                        let marker = if let Some(number) = list.next.as_mut() {
                            let text = format!("{number}. ");
                            *number = number.saturating_add(1);
                            text
                        } else {
                            "• ".to_owned()
                        };
                        list.indent = UnicodeWidthStr::width(marker.as_str());
                        self.marker = Some(marker);
                    }
                }
                Tag::Emphasis => self.push_style(|style| style.italic = true),
                Tag::Strong => self.push_style(|style| style.bold = true),
                Tag::Strikethrough => self.push_style(|style| style.strike = true),
                Tag::Link { dest_url, .. } => {
                    self.links.push(safe_text(&dest_url));
                    self.push_style(|style| {
                        style.foreground = Some(Color::Blue);
                        style.underline = true;
                    });
                }
                Tag::Image { dest_url, .. } => {
                    self.links.push(safe_text(&dest_url));
                    self.push_style(|style| style.dim = true);
                    self.generated("[image: ".to_owned(), self.style());
                }
                Tag::CodeBlock(kind) => {
                    self.gap();
                    let language = match kind {
                        CodeBlockKind::Fenced(info) => {
                            info.split_whitespace().next().unwrap_or("").to_owned()
                        }
                        CodeBlockKind::Indented => String::new(),
                    };
                    self.generated(
                        format!(
                            "┌─ {}",
                            if language.is_empty() {
                                "code"
                            } else {
                                &language
                            }
                        ),
                        MUTED,
                    );
                    let highlighter = if self.highlight {
                        let syntaxes = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
                        let themes = THEMES.get_or_init(ThemeSet::load_defaults);
                        let syntax = syntaxes
                            .find_syntax_by_token(&language)
                            .unwrap_or_else(|| syntaxes.find_syntax_plain_text());
                        Some(HighlightLines::new(
                            syntax,
                            &themes.themes["base16-ocean.dark"],
                        ))
                    } else {
                        None
                    };
                    // The header must be flushed after its queued text is consumed.
                    self.run("\n".into(), MUTED, true, range.start, false);
                    self.code = Some(CodeState {
                        highlighter,
                        header: true,
                    });
                }
                Tag::Table(alignments) => {
                    self.gap();
                    let count = alignments.len();
                    // Small screens and nested tables use a flowing layout.
                    // Normal tables buffer just one row and wrap cells to fit.
                    if count > 0
                        && self.quotes == 0
                        && self.lists.is_empty()
                        && self.width
                            >= count
                                .saturating_mul(4)
                                .saturating_add(count.saturating_sub(1).saturating_mul(3))
                    {
                        let available = self.width - (count - 1) * 3;
                        let widths = (0..count)
                            .map(|i| available / count + usize::from(i < available % count))
                            .collect();
                        self.table = Some(Table {
                            widths,
                            alignments,
                            cells: vec![Vec::new(); count],
                            cell: 0,
                            active: false,
                            source: range.start,
                        });
                    }
                }
                Tag::TableHead => {
                    self.table_cell = 0;
                    self.push_style(|style| {
                        style.bold = true;
                        style.foreground = Some(Color::Cyan);
                    });
                }
                Tag::TableRow => {
                    self.flush(false);
                    self.table_cell = 0;
                    if let Some(table) = self.table.as_mut() {
                        table.source = range.start;
                    }
                }
                Tag::TableCell => {
                    if let Some(table) = self.table.as_mut() {
                        table.cell = self.table_cell.min(table.widths.len() - 1);
                        table.active = true;
                    } else if self.table_cell > 0 {
                        self.generated(" │ ".to_owned(), MUTED);
                    }
                    self.table_cell += 1;
                }
                Tag::FootnoteDefinition(label) => {
                    self.gap();
                    self.generated(format!("[{label}] "), ACCENT);
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => {
                    if self.lists.is_empty() {
                        self.gap();
                    } else {
                        self.flush(false);
                    }
                }
                TagEnd::Heading(_) => {
                    self.flush(false);
                    self.pop_style();
                    self.gap();
                }
                TagEnd::BlockQuote(_) => {
                    self.flush(false);
                    self.quotes = self.quotes.saturating_sub(1);
                    self.gap();
                }
                TagEnd::List(_) => {
                    self.flush(false);
                    self.lists.pop();
                    if self.lists.is_empty() {
                        self.gap();
                    }
                }
                TagEnd::Item => {
                    self.flush(false);
                    self.marker = None;
                }
                TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough => self.pop_style(),
                TagEnd::Link | TagEnd::Image => {
                    if tag == TagEnd::Image {
                        self.generated("]".to_owned(), self.style());
                    }
                    self.pop_style();
                    if let Some(url) = self.links.pop().filter(|url| !url.is_empty()) {
                        self.generated(format!(" ({url})"), MUTED);
                    }
                }
                TagEnd::CodeBlock => {
                    self.flush(false);
                    self.code = None;
                    self.generated("└─".to_owned(), MUTED);
                    self.run("\n".into(), MUTED, true, range.start, false);
                }
                TagEnd::TableHead => {
                    self.flush(false);
                    self.pop_style();
                    if self.table.is_some() {
                        self.table_row(true);
                    } else {
                        self.generated("─".repeat(self.width.min(60)), MUTED);
                        self.run("\n".into(), MUTED, true, range.start, false);
                    }
                }
                TagEnd::TableCell => {
                    self.flush(false);
                    if let Some(table) = self.table.as_mut() {
                        table.active = false;
                    }
                }
                TagEnd::TableRow => {
                    self.flush(false);
                    if self.table.is_some() {
                        self.table_row(false);
                    }
                }
                TagEnd::Table => {
                    self.table = None;
                    self.gap();
                }
                TagEnd::FootnoteDefinition => self.gap(),
                _ => {}
            },
            Event::Text(text) => {
                if self.code.is_some() {
                    self.code_input = Some(CodeInput {
                        text,
                        cursor: 0,
                        source: range.start,
                    });
                } else {
                    let exact = text.len() == range.len();
                    self.run(text, self.style(), false, range.start, exact);
                }
            }
            Event::Code(text) => self.run(text, CODE, false, range.start, false),
            Event::Html(text) | Event::InlineHtml(text) => {
                self.run(text, MUTED, true, range.start, true)
            }
            Event::SoftBreak => self.generated(" ".to_owned(), self.style()),
            Event::HardBreak => self.flush(true),
            Event::Rule => {
                self.gap();
                self.generated("─".repeat(self.width.min(60)), MUTED);
                self.run("\n".into(), MUTED, true, range.start, false);
            }
            Event::TaskListMarker(checked) => {
                self.generated(if checked { "☑ " } else { "☐ " }.to_owned(), ACCENT)
            }
            Event::FootnoteReference(label) => self.generated(format!("[{label}]"), ACCENT),
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                self.run(text, CODE, false, range.start, false)
            }
        }
    }
}

impl Iterator for Renderer<'_> {
    type Item = Line;

    fn next(&mut self) -> Option<Line> {
        loop {
            if let Some(line) = self.ready.pop_front() {
                return Some(line);
            }
            if !self.runs.is_empty() {
                self.advance();
                continue;
            }
            if self.code_input.is_some() {
                self.code_line();
                continue;
            }
            if self.finished {
                return None;
            }
            if let Some((event, range)) = self.parser.next() {
                self.event(event, range);
            } else {
                self.flush(false);
                self.finished = true;
            }
        }
    }
}
