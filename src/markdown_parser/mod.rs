pub mod highlight_event_processor;
pub mod math_event_processor;

use crate::{
    converters::Converter, markdown_parser::highlight_event_processor::HighlightEventProcessor,
};
use math_event_processor::MathEventProcessor;
use pulldown_cmark::{Options, Parser, TextMergeStream};
use std::cell::RefCell;

#[allow(dead_code)]
pub struct MarkdownParser {
    pub with_katex: bool,
    pub with_highlight: bool,
    pub math_event_processor: MathEventProcessor,
    pub highlight_event_processor: Box<RefCell<HighlightEventProcessor>>,
}

impl MarkdownParser {
    pub fn new() -> MarkdownParser {
        MarkdownParser {
            with_katex: false,
            with_highlight: false,
            math_event_processor: MathEventProcessor::new(),
            highlight_event_processor: Box::new(RefCell::new(HighlightEventProcessor::new())),
        }
    }
}

impl Converter for MarkdownParser {
    fn convert(&self, content: Vec<u8>) -> Vec<u8> {
        let content_utf8 = String::from_utf8(content).unwrap();

        let mut options = Options::empty();
        options.insert(Options::ENABLE_MATH);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_GFM);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TABLES);

        let parser = Parser::new_ext(content_utf8.as_str(), options);
        let parse_iter = TextMergeStream::new(parser)
            .map(|event| self.math_event_processor.process_math_event(event))
            .map(|event| {
                self.highlight_event_processor
                    .borrow_mut()
                    .process_highlight_event(event)
            });

        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parse_iter);

        html_output.into_bytes()
    }
}
