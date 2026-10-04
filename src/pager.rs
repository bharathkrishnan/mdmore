use std::io::{self, Write};
use std::time::Duration;

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEventKind,
};
use crossterm::style::{Attribute, Print, ResetColor, SetAttribute};
use crossterm::terminal::{
    self, Clear, ClearType, DisableLineWrap, EnableLineWrap, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use crossterm::{execute, queue};
use mdmore::document::Document;
use mdmore::style::{safe_text, truncate, unsafe_char};
use unicode_segmentation::UnicodeSegmentation;

struct Terminal {
    mouse: bool,
}

impl Terminal {
    fn enter(mouse: bool) -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let guard = Self { mouse };
        let mut out = io::stdout();
        execute!(out, EnterAlternateScreen, Hide, DisableLineWrap)?;
        if mouse {
            execute!(out, EnableMouseCapture)?;
        }
        Ok(guard)
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let mut out = io::stdout();
        if self.mouse {
            let _ = execute!(out, DisableMouseCapture);
        }
        let _ = execute!(
            out,
            ResetColor,
            SetAttribute(Attribute::Reset),
            EnableLineWrap,
            Show,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

enum Mode {
    Reading,
    Search(String),
    Help,
}

struct Pager<'a> {
    doc: Document<'a>,
    name: String,
    columns: u16,
    rows: u16,
    width_limit: Option<u16>,
    top: usize,
    color: bool,
    query: String,
    last_match: Option<usize>,
    quit: bool,
    mode: Mode,
    message: String,
    // Compare serialized rows to redraw only those that changed.
    frame: Vec<Vec<u8>>,
    scratch: Vec<Vec<u8>>,
    output: Vec<u8>,
    frame_top: usize,
    frame_query: String,
    frame_help: bool,
}

impl<'a> Pager<'a> {
    fn new(
        source: &'a str,
        name: &str,
        width_limit: Option<u16>,
        color: bool,
        highlight: bool,
    ) -> io::Result<Self> {
        let (columns, rows) = terminal::size()?;
        let width = width_limit.unwrap_or(columns).min(columns).max(1);
        Ok(Self {
            doc: Document::new(source, usize::from(width), highlight),
            name: safe_text(name),
            columns: columns.max(1),
            rows: rows.max(1),
            width_limit,
            top: 0,
            color,
            query: String::new(),
            last_match: None,
            quit: false,
            mode: Mode::Reading,
            message: String::new(),
            frame: Vec::new(),
            scratch: Vec::new(),
            output: Vec::new(),
            frame_top: 0,
            frame_query: String::new(),
            frame_help: false,
        })
    }

    fn height(&self) -> usize {
        usize::from(self.rows.saturating_sub(1))
    }

    fn clamp(&mut self) {
        self.top = self
            .top
            .min(self.doc.lines.len().saturating_sub(self.height().max(1)));
    }

    fn move_down(&mut self, amount: usize) {
        self.last_match = None;
        let target = self.top.saturating_add(amount);
        self.doc
            .ensure(target.saturating_add(self.height()).saturating_add(1));
        self.top = target;
        self.clamp();
    }

    fn status(&self) -> String {
        match &self.mode {
            Mode::Search(input) => format!("/{input}▏"),
            Mode::Help => "Help · press any key to return".to_owned(),
            Mode::Reading => {
                if !self.message.is_empty() {
                    return self.message.clone();
                }
                if self.doc.complete && self.doc.lines.is_empty() {
                    return format!("{} · empty · END   q:quit", self.name);
                }
                let at_end = self.doc.complete && self.top + self.height() >= self.doc.lines.len();
                let progress = if at_end {
                    "END".to_owned()
                } else {
                    let offset = self.doc.lines.get(self.top).map_or(0, |line| line.source);
                    format!(
                        "{}%",
                        offset.saturating_mul(100) / self.doc.source_len().max(1)
                    )
                };
                format!(
                    "{} · {}–{} · {progress}   Space:page  /:search  ?:help  q:quit",
                    self.name,
                    self.top + 1,
                    (self.top + self.height()).min(self.doc.lines.len())
                )
            }
        }
    }

    fn draw(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.doc
            .ensure(self.top.saturating_add(self.height()).saturating_add(1));
        self.clamp();
        let mut next_frame = std::mem::take(&mut self.scratch);
        next_frame.resize_with(usize::from(self.rows), Vec::new);
        let is_help = matches!(self.mode, Mode::Help);
        let reuse = !is_help && !self.frame_help && self.query == self.frame_query;
        let help = [
            "mdmore — keys",
            "",
            "Space / f / PageDown     Next page",
            "b / PageUp               Previous page",
            "j / Down / Enter         Down one line",
            "k / Up                   Up one line",
            "Ctrl-D / Ctrl-U          Down / up half a page",
            "g / Home                 Beginning",
            "G / End                  End (Esc cancels)",
            "/                        Search (case-sensitive)",
            "n / N                    Next / previous matching line",
            "Esc                      Clear search highlighting",
            "Mouse wheel              Scroll",
            "q / Ctrl-C               Quit",
            "? / h                    This help",
        ];
        for (row, bytes) in next_frame.iter_mut().take(self.height()).enumerate() {
            bytes.clear();
            let cached = (self.top + row)
                .checked_sub(self.frame_top)
                .filter(|&index| index < self.frame.len().saturating_sub(1))
                .and_then(|index| self.frame.get(index));
            if let Some(cached) = cached.filter(|_| reuse) {
                bytes.extend_from_slice(cached);
            } else if is_help {
                bytes.extend_from_slice(
                    truncate(
                        help.get(row).copied().unwrap_or(""),
                        usize::from(self.columns),
                    )
                    .as_bytes(),
                );
            } else if let Some(line) = self.doc.lines.get(self.top + row) {
                line.write(bytes, self.color, &self.query)?;
            }
        }
        let status = &mut next_frame[self.height()];
        status.clear();
        if self.color {
            queue!(status, SetAttribute(Attribute::Reverse))?;
        }
        write!(
            status,
            "{}",
            truncate(&safe_text(&self.status()), usize::from(self.columns))
        )?;
        if self.color {
            queue!(status, ResetColor)?;
        }
        // Write changed rows together to reduce partial updates.
        self.output.clear();
        let output = &mut self.output;
        for (row, bytes) in next_frame.iter().enumerate() {
            if self.frame.get(row) != Some(bytes) {
                queue!(
                    output,
                    MoveTo(0, row as u16),
                    ResetColor,
                    Clear(ClearType::CurrentLine)
                )?;
                output.extend_from_slice(bytes);
            }
        }
        out.write_all(output)?;
        out.flush()?;
        self.scratch = std::mem::replace(&mut self.frame, next_frame);
        self.frame_top = self.top;
        self.frame_query.clone_from(&self.query);
        self.frame_help = is_help;
        Ok(())
    }

    fn resize(&mut self, columns: u16, rows: u16) {
        self.last_match = None;
        self.columns = columns.max(1);
        self.rows = rows.max(1);
        let width = usize::from(
            self.width_limit
                .unwrap_or(self.columns)
                .min(self.columns)
                .max(1),
        );
        if width != self.doc.width {
            self.top = self.doc.resize(width, self.top, self.height());
        }
        self.frame.clear();
        self.clamp();
    }

    /// Check for cancellation, exit, or resize between rendering batches.
    fn cancel_pending(&mut self) -> io::Result<bool> {
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(key)
                    if key.kind != KeyEventKind::Release
                        && (matches!(key.code, KeyCode::Esc | KeyCode::Char('q'))
                            || key.code == KeyCode::Char('c')
                                && key.modifiers.contains(KeyModifiers::CONTROL)) =>
                {
                    self.quit = key.code != KeyCode::Esc;
                    self.message = "Cancelled · q:quit".to_owned();
                    return Ok(true);
                }
                Event::Resize(columns, rows) => {
                    self.resize(columns, rows);
                    return Ok(true);
                }
                _ => {}
            }
        }
        Ok(false)
    }

    fn busy(&self, out: &mut impl Write, message: &str) -> io::Result<()> {
        queue!(
            out,
            MoveTo(0, self.rows.saturating_sub(1)),
            Clear(ClearType::CurrentLine),
            Print(truncate(message, usize::from(self.columns)))
        )?;
        out.flush()
    }

    fn end(&mut self, out: &mut impl Write) -> io::Result<()> {
        self.busy(out, "Rendering to end… Esc:cancel")?;
        while !self.doc.complete {
            if self.cancel_pending()? {
                return Ok(());
            }
            self.doc.advance();
        }
        self.top = self.doc.lines.len().saturating_sub(self.height());
        self.frame.clear();
        Ok(())
    }

    fn search(
        &mut self,
        backward: bool,
        include_current: bool,
        out: &mut impl Write,
    ) -> io::Result<()> {
        if self.query.is_empty() {
            self.message = "Press / to enter a search".to_owned();
            return Ok(());
        }
        let anchor = if include_current {
            self.top
        } else {
            self.last_match.unwrap_or(self.top)
        };
        if backward && anchor == 0 {
            self.message = "No earlier match".to_owned();
            return Ok(());
        }
        let mut start = if backward || include_current {
            anchor
        } else {
            anchor + 1
        };
        self.busy(out, "Searching… Esc:cancel")?;
        loop {
            if let Some(row) = self.doc.find_cached(&self.query, start, backward) {
                self.top = row;
                self.last_match = Some(row);
                self.clamp();
                self.message.clear();
                self.frame.clear();
                return Ok(());
            }
            if backward || self.doc.complete {
                self.message = if backward {
                    "No earlier match"
                } else {
                    "No further match"
                }
                .to_owned();
                self.frame.clear();
                return Ok(());
            }
            if self.cancel_pending()? {
                self.frame.clear();
                return Ok(());
            }
            start = self.doc.lines.len();
            self.doc.advance();
        }
    }

    fn key(&mut self, key: KeyEvent, out: &mut impl Write) -> io::Result<bool> {
        if key.kind == KeyEventKind::Release {
            return Ok(false);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            return Ok(true);
        }
        self.message.clear();
        if let Mode::Search(input) = &mut self.mode {
            match key.code {
                KeyCode::Esc => self.mode = Mode::Reading,
                KeyCode::Enter => {
                    if !input.is_empty() {
                        self.query = input.clone();
                    }
                    self.mode = Mode::Reading;
                    self.last_match = None;
                    self.search(false, true, out)?;
                }
                KeyCode::Backspace => {
                    if let Some((index, _)) = input.grapheme_indices(true).next_back() {
                        input.truncate(index);
                    }
                }
                KeyCode::Char(c)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                        && !unsafe_char(c) =>
                {
                    input.push(c)
                }
                _ => {}
            }
            return Ok(false);
        }
        if matches!(self.mode, Mode::Help) {
            self.mode = Mode::Reading;
            return Ok(false);
        }
        let page = self.height().saturating_sub(1).max(1);
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            self.last_match = None;
            match key.code {
                KeyCode::Char('d') => self.move_down((page / 2).max(1)),
                KeyCode::Char('u') => self.top = self.top.saturating_sub((page / 2).max(1)),
                KeyCode::Char('f') => self.move_down(page),
                KeyCode::Char('b') => self.top = self.top.saturating_sub(page),
                _ => {}
            }
            return Ok(false);
        }
        if matches!(
            key.code,
            KeyCode::Char(' ' | 'f' | 'b' | 'j' | 'k' | 'g' | 'G')
                | KeyCode::PageDown
                | KeyCode::PageUp
                | KeyCode::Down
                | KeyCode::Up
                | KeyCode::Enter
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Esc
        ) {
            self.last_match = None;
        }
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char(' ') | KeyCode::Char('f') | KeyCode::PageDown => self.move_down(page),
            KeyCode::Char('b') | KeyCode::PageUp => self.top = self.top.saturating_sub(page),
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Enter => self.move_down(1),
            KeyCode::Char('k') | KeyCode::Up => self.top = self.top.saturating_sub(1),
            KeyCode::Char('g') | KeyCode::Home => self.top = 0,
            KeyCode::Char('G') | KeyCode::End => self.end(out)?,
            KeyCode::Char('/') => self.mode = Mode::Search(String::new()),
            KeyCode::Char('n') => self.search(false, false, out)?,
            KeyCode::Char('N') => self.search(true, false, out)?,
            KeyCode::Char('?') | KeyCode::Char('h') => self.mode = Mode::Help,
            KeyCode::Esc => self.query.clear(),
            _ => {}
        }
        Ok(false)
    }
}

pub fn run(
    source: &str,
    name: &str,
    width_limit: Option<u16>,
    color: bool,
    highlight: bool,
    mouse: bool,
) -> io::Result<()> {
    // Render the first viewport before changing terminal modes.
    let mut pager = Pager::new(source, name, width_limit, color, highlight)?;
    pager.doc.ensure(pager.height().saturating_add(1));
    // Check input setup before entering raw mode. Poll retains queued keys.
    event::poll(Duration::ZERO)?;
    // A panic must restore the terminal before the standard panic hook prints.
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            ResetColor,
            SetAttribute(Attribute::Reset),
            EnableLineWrap,
            Show,
            LeaveAlternateScreen
        );
        old_hook(info);
    }));
    let _terminal = Terminal::enter(mouse)?;
    let mut out = io::stdout();
    loop {
        pager.draw(&mut out)?;
        match event::read()? {
            Event::Key(key) => {
                if pager.key(key, &mut out)? || pager.quit {
                    break;
                }
            }
            Event::Resize(columns, rows) => pager.resize(columns, rows),
            Event::Mouse(mouse) if matches!(pager.mode, Mode::Reading) => match mouse.kind {
                MouseEventKind::ScrollDown => pager.move_down(3),
                MouseEventKind::ScrollUp => {
                    pager.last_match = None;
                    pager.top = pager.top.saturating_sub(3);
                }
                _ => {}
            },
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pager(source: &str) -> Pager<'_> {
        Pager {
            doc: Document::new(source, 80, false),
            name: "test".into(),
            columns: 80,
            rows: 24,
            width_limit: None,
            top: 0,
            color: true,
            query: String::new(),
            last_match: None,
            quit: false,
            mode: Mode::Reading,
            message: String::new(),
            frame: Vec::new(),
            scratch: Vec::new(),
            output: Vec::new(),
            frame_top: 0,
            frame_query: String::new(),
            frame_help: false,
        }
    }

    fn check(p: &mut Pager<'_>) {
        p.draw(&mut Vec::new()).unwrap();
        if matches!(p.mode, Mode::Reading) {
            for row in 0..p.height() {
                let mut expected = Vec::new();
                if let Some(line) = p.doc.lines.get(p.top + row) {
                    line.write(&mut expected, p.color, &p.query).unwrap();
                }
                assert_eq!(p.frame[row], expected, "row {row}");
            }
        }
    }

    #[test]
    fn frame_survives_scroll_search_help_and_reflow() {
        crossterm::style::force_color_output(true);
        let source = "Some **bold** text with 日本語 and 👩‍🌾.\n\n".repeat(100);
        let mut p = pager(&source);
        check(&mut p);
        p.top = 3;
        check(&mut p);
        p.top = 1;
        check(&mut p);
        p.query = "bold".into();
        check(&mut p);
        p.mode = Mode::Help;
        check(&mut p);
        p.mode = Mode::Reading;
        check(&mut p);
        p.query.clear();
        check(&mut p);
        p.resize(40, 12);
        check(&mut p);
        p.top = p.doc.lines.len().saturating_sub(1);
        check(&mut p);
        p.resize(80, 24);
        check(&mut p);
    }

    #[test]
    #[ignore = "release performance measurement"]
    fn draw_benchmark() {
        use std::{hint::black_box, time::Instant};
        crossterm::style::force_color_output(true);
        let source = "Some **bold** and *italic* text with 日本語 and 👩‍🌾.\n\n".repeat(100);
        for query in ["", "bold"] {
            let mut p = pager(&source);
            p.query = query.into();
            p.doc.ensure(100);
            let mut out = std::io::sink();
            p.draw(&mut out).unwrap();
            let start = Instant::now();
            for index in 0..20_000 {
                p.top = index % 20;
                p.draw(&mut out).unwrap();
                black_box(&p.frame);
            }
            eprintln!(
                "draw query={query:?}: {} ns/iteration",
                start.elapsed().as_nanos() / 20_000
            );
        }
    }
}
