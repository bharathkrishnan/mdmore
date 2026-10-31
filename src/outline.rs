//! Extract heading positions without rendering the document.
use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: usize,
    pub text: String,
    pub folded: String,
    pub source: usize,
}

pub fn headings(source: &str) -> Vec<Heading> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let mut result = Vec::new();
    let mut current: Option<Heading> = None;
    for (event, range) in Parser::new_ext(source, options).into_offset_iter() {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(Heading {
                    level: level as usize,
                    text: String::new(),
                    folded: String::new(),
                    source: range.start,
                });
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut heading) = current.take() {
                    heading.folded = heading.text.to_lowercase();
                    result.push(heading);
                }
            }
            Event::Text(text) | Event::Code(text) | Event::InlineMath(text)
                if current.is_some() =>
            {
                current.as_mut().unwrap().text.push_str(&text);
            }
            Event::SoftBreak | Event::HardBreak if current.is_some() => {
                current.as_mut().unwrap().text.push(' ')
            }
            _ => {}
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn markdown_headings_keep_text_levels_and_offsets() {
        let source = "# **Hello** `world`\n\n```\n# not a heading\n```\n\nSub\n---\n";
        let found = headings(source);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].text, "Hello world");
        assert_eq!(found[0].level, 1);
        assert_eq!(found[1].text, "Sub");
        assert_eq!(found[1].level, 2);
        assert!(source[found[1].source..].starts_with("Sub"));
    }
}
