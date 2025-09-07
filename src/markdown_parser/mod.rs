pub mod event_processor;
pub mod highlight_event_processor;
pub mod image_event_processor;
pub mod math_event_processor;

use crate::converters::Converter;
use event_processor::ProcessWith;
use highlight_event_processor::HighlightEventProcessor;
use image_event_processor::ImageEventProcessor;
use math_event_processor::MathEventProcessor;
use pulldown_cmark::{Options, Parser, TextMergeStream};
use std::cell::RefCell;

macro_rules! render_pipeline {
    ($parser:expr, $($processor:expr),*) => {
        {
            let parse_iter = TextMergeStream::new($parser);
            $(
                let mut processor = $processor.borrow_mut();
                let parse_iter = parse_iter.process_with(&mut *processor);
            )*

            let mut html_output = String::new();
            pulldown_cmark::html::push_html(&mut html_output, parse_iter);
            html_output
        }
    }
}

#[allow(dead_code)]
pub struct MarkdownParser {
    pub with_katex: bool,
    pub with_highlight: bool,
    math_event_processor: Box<RefCell<MathEventProcessor>>,
    highlight_event_processor: Box<RefCell<HighlightEventProcessor>>,
    image_event_processor: Box<RefCell<ImageEventProcessor>>,
}

impl MarkdownParser {
    pub fn new() -> MarkdownParser {
        MarkdownParser {
            with_katex: false,
            with_highlight: false,
            math_event_processor: Box::new(RefCell::new(MathEventProcessor::new())),
            highlight_event_processor: Box::new(RefCell::new(HighlightEventProcessor::new())),
            image_event_processor: Box::new(RefCell::new(ImageEventProcessor::new())),
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
        options.insert(Options::ENABLE_SUPERSCRIPT);
        options.insert(Options::ENABLE_SUBSCRIPT);

        let parser = Parser::new_ext(&content_utf8, options);

        /* old solution. less flexible, but compiles faster and be much simpler */
        /*
        let mut html_output = String::new();
        let parse_iter = TextMergeStream::new(parser);
        let parse_iter = parse_iter
            .map(|event| self.math_event_processor.borrow().process_math_event(event))
            .map(|event| self.highlight_event_processor.borrow_mut().process_highlight_event(event))
            .map(|event| self.image_event_processor.borrow_mut().process_image_event(event))
            .flat_map(|event| event.into_iter());
        pulldown_cmark::html::push_html(&mut html_output, parse_iter);
        */

        /* new solution. seems more flexible */
        let html_output = render_pipeline!(
            parser,
            self.math_event_processor,
            self.highlight_event_processor,
            self.image_event_processor
        );

        html_output.into_bytes()
    }
}
