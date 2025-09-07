use pulldown_cmark::{CodeBlockKind, Event, Tag, TagEnd};
use syntect::html::{ClassStyle, ClassedHTMLGenerator};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

use super::event_processor::EventProcessor;

#[derive(Debug, Eq, PartialEq)]
pub enum State {
    NotCodeBlock,
    InCodeBlock,
}

pub struct HighlightEventProcessor {
    pub state: State,
    pub language: Option<String>,
    pub syntax_set: SyntaxSet,
}

impl HighlightEventProcessor {
    pub fn new() -> HighlightEventProcessor {
        HighlightEventProcessor {
            state: State::NotCodeBlock,
            language: None,
            syntax_set: SyntaxSet::load_defaults_newlines(),
        }
    }

    pub fn process_highlight_event<'a>(&mut self, event: Event<'a>) -> Event<'a> {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang))) => {
                self.state = State::InCodeBlock;
                self.language = Some(lang.to_string());
                Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
            }
            Event::Text(t) => {
                if self.state == State::InCodeBlock {
                    let lang = self.language.clone().unwrap();
                    let syntax = self.syntax_set.find_syntax_by_token(lang.as_str());

                    if let Some(syntax) = syntax {
                        let code = t.to_string();

                        let mut html_generator = ClassedHTMLGenerator::new_with_class_style(
                            syntax,
                            &self.syntax_set,
                            ClassStyle::Spaced,
                        );
                        for line in LinesWithEndings::from(code.as_str()) {
                            let _ = html_generator.parse_html_for_line_which_includes_newline(line);
                        }

                        let output_html = html_generator.finalize();
                        Event::Html(output_html.into())
                    } else {
                        Event::Text(t)
                    }
                } else {
                    Event::Text(t)
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                self.state = State::NotCodeBlock;
                self.language = None;
                event
            }
            _ => event,
        }
    }
}

impl EventProcessor for HighlightEventProcessor {
    fn apply<'a>(&'a mut self, iter: impl Iterator<Item = Event<'a>>) -> impl Iterator<Item = Event<'a>> {
        iter.map(move |event| self.process_highlight_event(event))
    }
}

#[cfg(test)]
mod tests {
    use pulldown_cmark::{Parser, TextMergeStream};
    use std::cell::RefCell;
    use syntect::highlighting::ThemeSet;
    use syntect::html::{css_for_theme_with_class_style, ClassStyle};

    use crate::markdown_parser::highlight_event_processor::HighlightEventProcessor;

    #[test]
    fn test_highlight_processor() {
        let content = r#"```python
import json

if __name__ == "__main__":
    print("helloworld")
```
"#;

        let highlight_event_processor = HighlightEventProcessor::new();
        let highlight_event_processor = Box::new(RefCell::new(highlight_event_processor));

        let parser = Parser::new(content);
        let parser_iter = TextMergeStream::new(parser).map(|event| {
            highlight_event_processor
                .borrow_mut()
                .process_highlight_event(event)
        });

        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser_iter);
        println!("[output]\n{}", &html_output);
    }

    #[test]
    fn get_css() {
        let ts = ThemeSet::load_defaults();

        let dark_theme = &ts.themes["Solarized (dark)"];
        let css_dark = css_for_theme_with_class_style(dark_theme, ClassStyle::Spaced).unwrap();
        println!("[[css-dark]]\n{}", css_dark);

        let light_theme = &ts.themes["InspiredGitHub"];
        let css_light = css_for_theme_with_class_style(light_theme, ClassStyle::Spaced).unwrap();
        println!("[[css-light]]\n{}", css_light);
    }
}
