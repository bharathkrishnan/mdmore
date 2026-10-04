use crossterm::style::Color;
use mdmore::document::Document;
use mdmore::render::Renderer;
use mdmore::style::{Line, Style, safe_text};
use unicode_width::UnicodeWidthStr;

fn render(source: &str, width: usize) -> Vec<Line> {
    Renderer::new(source, width, false).collect()
}

fn text(lines: &[Line]) -> String {
    lines.iter().map(Line::plain).collect::<Vec<_>>().join("\n")
}

#[test]
fn markdown_has_structure_and_nested_styles() {
    let lines = render(
        "# Heading\n\nA **bold and *italic*** paragraph with `code`.\n\n> quote\n\n3. three\n4. four\n\n- [x] done\n- [ ] todo",
        80,
    );
    let plain = text(&lines);
    for expected in [
        "# Heading",
        "bold and italic",
        "code",
        "│ quote",
        "3. three",
        "4. four",
        "☑ done",
        "☐ todo",
    ] {
        assert!(
            plain.contains(expected),
            "missing {expected:?} in {plain:?}"
        );
    }
    assert!(
        lines
            .iter()
            .flat_map(|line| &line.spans)
            .any(|span| span.text.contains("italic") && span.style.bold && span.style.italic)
    );
    assert!(
        lines
            .iter()
            .flat_map(|line| &line.spans)
            .any(|span| span.text.contains("code") && span.style.foreground == Some(Color::Yellow))
    );
}

#[test]
fn colors_are_self_contained_after_a_page_boundary() {
    crossterm::style::force_color_output(true);
    let lines = render(&format!("**{}**", "styled ".repeat(100).trim_end()), 20);
    assert!(lines.len() > 20);
    let later = &lines[17];
    assert!(
        later
            .spans
            .iter()
            .filter(|span| !span.text.trim().is_empty())
            .all(|span| span.style.bold)
    );
    let mut ansi = Vec::new();
    later.write(&mut ansi, true, "").unwrap();
    let ansi = String::from_utf8(ansi).unwrap();
    assert!(ansi.contains("\x1b[1m"), "{ansi:?}");
    assert!(ansi.contains("\x1b[0m"), "{ansi:?}");
    let mut plain = Vec::new();
    later.write(&mut plain, false, "styled").unwrap();
    assert!(!plain.contains(&0x1b));
}

#[test]
fn large_paragraph_is_rendered_only_as_requested() {
    let source = "a long paragraph with **formatting** and words ".repeat(100_000);
    let mut doc = Document::new(&source, 40, false);
    doc.ensure(24);
    assert_eq!(doc.lines.len(), 24);
    assert!(!doc.complete);
    assert!(doc.lines.last().unwrap().source < 2_000);
    let first = doc.lines[0].plain();
    doc.ensure(100);
    assert_eq!(doc.lines.len(), 100);
    assert_eq!(doc.lines[0].plain(), first);
    doc.ensure(24);
    assert_eq!(doc.lines.len(), 100);
}

#[test]
fn a_single_huge_code_block_is_progressive() {
    let source = format!("```rust\n{}```\n", "let answer = 42;\n".repeat(100_000));
    let mut doc = Document::new(&source, 80, true);
    doc.ensure(24);
    assert_eq!(doc.lines.len(), 24);
    assert!(!doc.complete);
    assert!(doc.lines.last().unwrap().source < 1_000);
    assert!(
        doc.lines
            .iter()
            .flat_map(|line| &line.spans)
            .any(|span| matches!(span.style.foreground, Some(Color::Rgb { .. })))
    );
}

#[test]
fn multiline_syntax_state_survives_rendering_in_pages() {
    let source = format!(
        "```rust\n/* comment starts\n{}*/\nlet after = 42;\n```",
        "still a comment\n".repeat(40)
    );
    let mut doc = Document::new(&source, 80, true);
    doc.ensure(24);
    assert!(!doc.complete);
    let color = |line: &Line, word: &str| {
        line.spans
            .iter()
            .find(|span| span.text.contains(word))
            .unwrap()
            .style
            .foreground
    };
    let comment_color = color(&doc.lines[2], "still");
    assert!(matches!(comment_color, Some(Color::Rgb { .. })));
    doc.ensure(100);
    assert_eq!(color(&doc.lines[40], "still"), comment_color);
    let after = doc
        .lines
        .iter()
        .find(|line| line.plain().contains("let after"))
        .unwrap();
    assert_ne!(color(after, "let"), comment_color);
}

#[test]
fn unicode_graphemes_and_nested_prefixes_fit_every_terminal_width() {
    let sources = [
        "# Hello 👩🏽‍💻 café e\u{301} 世界\n\n**αβγ** and a verylongwordwithnospaces",
        "> > - nested 世界 words 👨‍👩‍👧‍👦\n> >   continuation",
        "```\n\t世界 hello\n\n👩🏽‍💻\n```",
        "| Name | Result |\n|---|---|\n| 世界 | 👩🏽‍💻 |",
    ];
    for source in sources {
        for width in 1..=40 {
            let lines = render(source, width);
            assert!(
                lines.len() < 500,
                "renderer did not progress at width {width}"
            );
            for line in &lines {
                assert!(
                    UnicodeWidthStr::width(line.plain().as_str()) <= width,
                    "width={width}, line={:?}",
                    line.plain()
                );
                assert_eq!(line.width, UnicodeWidthStr::width(line.plain().as_str()));
            }
        }
    }
    let plain = text(&render(sources[0], 12));
    assert!(plain.contains("👩🏽‍💻"));
    assert!(plain.contains("e\u{301}"));
}

#[test]
fn ascii_adjacent_to_combining_marks_stays_in_one_grapheme() {
    let wrapped = render("x abc \u{301}d", 6);
    assert_eq!(wrapped[0].plain(), "x ");
    assert_eq!(wrapped[1].plain(), "abc \u{301}d");
    let words = ["abcde\u{301}", "abc1\u{fe0f}\u{20e3}", "abc \u{301}d"];
    for word in words {
        for source in [word.to_owned(), format!("```text\n{word}\n```")] {
            for width in 1..=12 {
                let lines = render(&source, width);
                for line in &lines {
                    assert_eq!(line.width, UnicodeWidthStr::width(line.plain().as_str()));
                    assert!(line.width <= width);
                }
                if width >= 4 {
                    let plain = text(&lines);
                    for grapheme in ["e\u{301}", "1\u{fe0f}\u{20e3}", " \u{301}"] {
                        if word.contains(grapheme) {
                            assert!(plain.contains(grapheme), "{plain:?}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn unicode_runs_keep_graphemes_styles_and_source_anchors() {
    let word = "世界你好日本語한국어か\u{3099}👩🏽‍💻e\u{301}";
    let source = format!("**{word}**");
    let lines = render(&source, 12);
    let rows: Vec<_> = lines.iter().filter(|line| !line.spans.is_empty()).collect();
    assert_eq!(
        rows.iter().map(|line| line.plain()).collect::<Vec<_>>(),
        ["世界你好日本", "語한국어か\u{3099}👩🏽‍💻", "e\u{301}",]
    );
    assert_eq!(
        rows.iter().map(|line| line.source).collect::<Vec<_>>(),
        [2, 2 + word.find('語').unwrap(), 2 + word.find('e').unwrap(),]
    );
    assert!(
        rows.iter()
            .flat_map(|line| &line.spans)
            .all(|span| span.style.bold)
    );
    assert_eq!(
        rows.iter().map(|line| line.width).collect::<Vec<_>>(),
        [12, 12, 1]
    );

    // An initial zero-width cluster must not move the resize anchor backward.
    let lines = render("\u{301}\u{300}世界", 2);
    assert_eq!(lines[0].plain(), "\u{301}\u{300}世");
    assert_eq!(lines[0].source, 4);
    assert_eq!(lines[1].plain(), "界");
    assert_eq!(lines[1].source, 7);
}

#[test]
fn terminal_escape_injection_is_removed_even_from_code_and_html() {
    let source = "# bad\x1b[2J\n\n```\ncode\x1b]52;c;payload\x07\n```\n\n<div>raw\x1b[31m</div>\n\nhello\u{202e}world";
    let lines = render(source, 80);
    for span in lines.iter().flat_map(|line| &line.spans) {
        assert!(!span.text.chars().any(|c| c.is_control() || c == '\u{202e}'));
    }
    assert_eq!(safe_text("file\x1b[2J\u{202e}.md"), "file[2J.md");
}

#[test]
fn reference_links_resolve_even_when_defined_at_the_end() {
    let lines = render(
        "[the manual][docs]\n\nLater text.\n\n[docs]: https://example.com/manual",
        80,
    );
    assert!(text(&lines).contains("the manual (https://example.com/manual)"));
}

#[test]
fn resize_preserves_a_source_anchor_without_rendering_to_eof() {
    let source = "A paragraph with enough text to wrap in narrow terminals.\n\n".repeat(5_000);
    let mut doc = Document::new(&source, 40, false);
    doc.ensure(100);
    let anchor = doc.lines[50].source;
    let top = doc.resize(20, 50, 24);
    assert!(!doc.complete);
    assert!(doc.lines.len() < 300);
    assert!(doc.lines[top].source.abs_diff(anchor) <= 40);
    assert!(doc.lines.iter().all(|line| line.width <= 20));
}

#[test]
fn empty_input_finishes_and_long_words_cannot_stall() {
    let mut empty = Document::new("", 80, false);
    empty.ensure(24);
    assert!(empty.complete);
    assert!(empty.lines.is_empty());
    let source = "x".repeat(1_000_000);
    let mut doc = Document::new(&source, 1, false);
    doc.ensure(24);
    assert_eq!(doc.lines.len(), 24);
    assert!(doc.lines.iter().all(|line| line.plain() == "x"));
}

#[test]
fn search_can_find_a_match_across_style_spans() {
    crossterm::style::force_color_output(true);
    let mut doc = Document::new("one\n\ntwo **styled** words\n\nthree", 80, false);
    doc.ensure(100);
    let found = doc.find_cached("two styled", 0, false).unwrap();
    let mut ansi = Vec::new();
    doc.lines[found]
        .write(&mut ansi, true, "two styled")
        .unwrap();
    let ansi = String::from_utf8(ansi).unwrap();
    assert!(ansi.contains("48;5;3"), "{ansi:?}");
    assert!(doc.find_cached("two styled", found, true).is_none());
    assert_eq!(doc.find_cached("two styled", found + 1, true), Some(found));
}

#[test]
fn cached_search_matches_plain_text_across_unicode_and_repeated_prefixes() {
    let sources = [
        "abababaé世界e\u{301}".to_owned(),
        format!("{}b", "a".repeat(40)),
    ];
    let queries = [
        "",
        "aba",
        "babab",
        "é世界",
        "世界e\u{301}",
        "aaaaab",
        "aaaaac",
        "absent",
    ];
    for source in sources {
        let boundaries: Vec<_> = source
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(source.len()))
            .collect();
        for (index, &first) in boundaries.iter().enumerate() {
            for &second in &boundaries[index..] {
                let mut line = Line::default();
                line.push(&source[..first], Style::default());
                line.push(
                    &source[first..second],
                    Style {
                        bold: true,
                        ..Style::default()
                    },
                );
                line.push(&source[second..], Style::default());
                let mut doc = Document::new("", 80, false);
                doc.lines.push(line);
                for query in queries {
                    let expected = source.contains(query).then_some(0);
                    assert_eq!(doc.find_cached(query, 0, false), expected);
                    assert_eq!(doc.find_cached(query, 1, true), expected);
                    assert_eq!(doc.find_cached(query, 0, true), None);
                }
            }
        }
    }
}

#[test]
fn repeated_search_highlights_follow_matches_across_styles() {
    crossterm::style::force_color_output(true);
    let styles = [
        Style::default(),
        Style {
            bold: true,
            ..Style::default()
        },
        Style {
            italic: true,
            ..Style::default()
        },
    ];
    let mut line = Line::default();
    for (part, style) in ["ab", "aba", "ba"].into_iter().zip(styles) {
        line.push(part, style);
    }
    let mut ansi = Vec::new();
    line.write(&mut ansi, true, "aba").unwrap();
    let expected = format!(
        "{}{}{}{}{}",
        styles[0].content(true).apply("ab"),
        styles[1].content(true).apply("a"),
        styles[1].content(false).apply("b"),
        styles[1].content(true).apply("a"),
        styles[2].content(true).apply("ba")
    );
    assert_eq!(String::from_utf8(ansi).unwrap(), expected);

    let mut combining = Line::default();
    combining.push("e\u{301} e\u{301}", Style::default());
    let mut ansi = Vec::new();
    combining.write(&mut ansi, true, "\u{301}").unwrap();
    let expected = format!(
        "{} {}",
        Style::default().content(true).apply("e\u{301}"),
        Style::default().content(true).apply("e\u{301}")
    );
    assert_eq!(String::from_utf8(ansi).unwrap(), expected);
}

#[test]
fn literal_crlf_is_one_line_break() {
    let lines = render("<pre>a\r\nb</pre>", 80);
    assert_eq!(lines[0].plain(), "<pre>a");
    assert_eq!(lines[1].plain(), "b</pre>");
}

#[test]
fn giant_code_line_wraps_without_eager_rendering() {
    let source = format!("```rust\n{}\n```", "a".repeat(1_000_000));
    let mut doc = Document::new(&source, 40, true);
    doc.ensure(24);
    assert_eq!(doc.lines.len(), 24);
    assert!(!doc.complete);
    assert!(doc.lines.last().unwrap().source < 2_000);
}

#[test]
fn code_indentation_and_blank_lines_survive() {
    let lines = render("```text\n    first\n\n\tsecond\n```", 80);
    let plain = text(&lines);
    assert!(plain.contains("│     first"), "{plain:?}");
    assert!(plain.contains("│ \n"), "{plain:?}");
    assert!(plain.contains("│     second"), "{plain:?}");
}

#[test]
fn table_columns_stay_aligned_when_cells_wrap() {
    let lines = render(
        "| Left | Center | Right |\n|:---|:---:|---:|\n| **a long wrapping cell with many words** | x | 42 |\n| short | y | 7 |",
        42,
    );
    let rows: Vec<_> = lines
        .iter()
        .filter(|line| line.plain().contains('│'))
        .collect();
    assert!(rows.len() > 3);
    for row in &rows {
        assert_eq!(row.width, 42);
        let plain = row.plain();
        let positions: Vec<_> = plain
            .char_indices()
            .filter(|(_, ch)| *ch == '│')
            .map(|(i, _)| UnicodeWidthStr::width(&plain[..i]))
            .collect();
        assert_eq!(positions, vec![13, 28]);
    }
    assert!(
        rows.iter()
            .flat_map(|line| &line.spans)
            .any(|span| span.style.bold && span.text.contains("wrapping"))
    );
    assert!(rows.iter().any(|line| line.plain().ends_with("42")));
}
