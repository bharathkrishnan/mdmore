use std::collections::VecDeque;

use crate::render::Renderer;
use crate::style::{Line, Search};

/// Renders rows on request and caches them for backward scrolling.
pub struct Document<'a> {
    source: &'a str,
    renderer: Renderer<'a>,
    pub lines: VecDeque<Line>,
    /// Absolute index of the first retained row.
    pub base: usize,
    cached_bytes: usize,
    pub complete: bool,
    pub width: usize,
    highlight: bool,
}

impl<'a> Document<'a> {
    pub fn new(source: &'a str, width: usize, highlight: bool) -> Self {
        Self {
            source,
            renderer: Renderer::new(source, width, highlight),
            lines: VecDeque::new(),
            base: 0,
            cached_bytes: 0,
            complete: false,
            width: width.max(1),
            highlight,
        }
    }

    pub fn ensure(&mut self, count: usize) {
        while self.end() < count && !self.complete {
            match self.renderer.next() {
                Some(line) => {
                    self.cached_bytes = self.cached_bytes.saturating_add(line_size(&line));
                    self.lines.push_back(line);
                }
                None => self.complete = true,
            }
        }
    }

    /// Render up to 128 more rows, then return to the event loop.
    pub fn advance(&mut self) {
        self.ensure(self.end().saturating_add(128));
    }

    pub fn end(&self) -> usize {
        self.base.saturating_add(self.lines.len())
    }

    pub fn line(&self, row: usize) -> Option<&Line> {
        row.checked_sub(self.base)
            .and_then(|index| self.lines.get(index))
    }

    /// Discard old rows while preserving the viewport and a small amount of history.
    pub fn trim(&mut self, limit: usize, protect: usize) {
        if self.cached_bytes <= limit {
            return;
        }
        let eligible = protect.saturating_sub(self.base).min(self.lines.len());
        let mut count = 0;
        while count < eligible && self.cached_bytes > limit {
            self.cached_bytes = self
                .cached_bytes
                .saturating_sub(line_size(&self.lines[count]));
            count += 1;
        }
        if count > 0 {
            self.lines.drain(..count);
            self.base += count;
            if self.lines.capacity() > self.lines.len().saturating_mul(2).max(256) {
                self.lines.shrink_to_fit();
            }
        }
    }

    pub fn reset_width(&mut self, width: usize) {
        *self = Self::new(self.source, width, self.highlight);
    }

    pub fn rewind(&mut self) {
        self.renderer = Renderer::new(self.source, self.width, self.highlight);
        self.lines = VecDeque::new();
        self.base = 0;
        self.cached_bytes = 0;
        self.complete = false;
    }

    pub fn source_len(&self) -> usize {
        self.source.len()
    }

    pub fn resize(&mut self, width: usize, top: usize, height: usize) -> usize {
        self.resize_limited(width, top, height, usize::MAX)
    }

    pub fn resize_limited(
        &mut self,
        width: usize,
        top: usize,
        height: usize,
        limit: usize,
    ) -> usize {
        let anchor = self.line(top).map_or(0, |line| line.source);
        self.resize_to_source(width, anchor, height, limit)
    }

    pub fn resize_to_source(
        &mut self,
        width: usize,
        anchor: usize,
        height: usize,
        limit: usize,
    ) -> usize {
        *self = Self::new(self.source, width, self.highlight);
        let top = loop {
            let start = self.end();
            self.ensure(start.saturating_add(128));
            if let Some(row) = self
                .lines
                .iter()
                .skip(start - self.base)
                .position(|line| line.source >= anchor)
            {
                break start + row;
            }
            if self.complete {
                break self.end().saturating_sub(1);
            }
            self.trim(limit, self.end().saturating_sub(128));
        };
        self.ensure(top.saturating_add(height).saturating_add(1));
        top.min(self.end().saturating_sub(height))
    }

    pub fn find_recent(&self, query: &str, start: usize) -> Option<usize> {
        let search = Search::new(query);
        (start.max(self.base)..self.end())
            .rev()
            .find(|&i| search.contains(self.line(i).unwrap()))
    }

    pub fn find_cached(&self, query: &str, start: usize, backward: bool) -> Option<usize> {
        let search = Search::new(query);
        if backward {
            (self.base..start.min(self.end()))
                .rev()
                .find(|&i| search.contains(self.line(i).unwrap()))
        } else {
            (start.max(self.base)..self.end()).find(|&i| search.contains(self.line(i).unwrap()))
        }
    }
}

fn line_size(line: &Line) -> usize {
    std::mem::size_of::<Line>()
        + line.spans.capacity() * std::mem::size_of::<crate::style::Span>()
        + line
            .spans
            .iter()
            .map(|span| span.text.capacity())
            .sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_rows_replay_with_identical_multiline_highlighting() {
        let source = format!(
            "```rust\n/* comment\n{}\n*/\n```\n",
            "still a comment\n".repeat(4000)
        );
        let mut document = Document::new(&source, 80, true);
        document.ensure(120);
        let mut expected = Vec::new();
        document
            .line(100)
            .unwrap()
            .write(&mut expected, true, "")
            .unwrap();
        while !document.complete {
            document.advance();
            document.trim(16 * 1024, document.end().saturating_sub(24));
        }
        assert!(document.base > 100);
        assert!(document.cached_bytes <= 16 * 1024);
        document.rewind();
        document.ensure(101);
        let mut actual = Vec::new();
        document
            .line(100)
            .unwrap()
            .write(&mut actual, true, "")
            .unwrap();
        assert_eq!(actual, expected);
    }
}
