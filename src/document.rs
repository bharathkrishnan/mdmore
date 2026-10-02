use crate::render::Renderer;
use crate::style::{Line, Search};

/// Renders rows on request and caches them for backward scrolling.
pub struct Document<'a> {
    source: &'a str,
    renderer: Renderer<'a>,
    pub lines: Vec<Line>,
    pub complete: bool,
    pub width: usize,
    highlight: bool,
}

impl<'a> Document<'a> {
    pub fn new(source: &'a str, width: usize, highlight: bool) -> Self {
        Self {
            source,
            renderer: Renderer::new(source, width, highlight),
            lines: Vec::new(),
            complete: false,
            width: width.max(1),
            highlight,
        }
    }

    pub fn ensure(&mut self, count: usize) {
        while self.lines.len() < count && !self.complete {
            match self.renderer.next() {
                Some(line) => self.lines.push(line),
                None => self.complete = true,
            }
        }
    }

    /// Render up to 128 more rows, then return to the event loop.
    pub fn advance(&mut self) {
        self.ensure(self.lines.len().saturating_add(128));
    }

    pub fn source_len(&self) -> usize {
        self.source.len()
    }

    pub fn resize(&mut self, width: usize, top: usize, height: usize) -> usize {
        let anchor = self.lines.get(top).map_or(0, |line| line.source);
        *self = Self::new(self.source, width, self.highlight);
        // Reflow to the saved source offset, then fill the viewport.
        loop {
            self.ensure(self.lines.len().saturating_add(1));
            if self.complete || self.lines.last().is_some_and(|line| line.source >= anchor) {
                break;
            }
        }
        let top = self
            .lines
            .iter()
            .position(|line| line.source >= anchor)
            .unwrap_or_else(|| self.lines.len().saturating_sub(1));
        self.ensure(top.saturating_add(height).saturating_add(1));
        top.min(self.lines.len().saturating_sub(height))
    }

    pub fn find_cached(&self, query: &str, start: usize, backward: bool) -> Option<usize> {
        let search = Search::new(query);
        if backward {
            (0..start.min(self.lines.len()))
                .rev()
                .find(|&i| search.contains(&self.lines[i]))
        } else {
            (start..self.lines.len()).find(|&i| search.contains(&self.lines[i]))
        }
    }
}
