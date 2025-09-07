// Since this is in the tools module, we need to go up one more level to access the crate root
use sushi_gen::markdown_parser::{
    highlight_event_processor::HighlightEventProcessor,
    image_event_processor::ImageEventProcessor,
    math_event_processor::MathEventProcessor,
    event_processor::DebugEventProcessor,
};
use sushi_gen::markdown_parser::event_processor::ProcessWith;
use sushi_gen::render_pipeline;
use std::path::PathBuf;
use pulldown_cmark::{Options, Parser, TextMergeStream};
use std::cell::RefCell;
use clap::Parser as cp;

#[derive(cp, Debug)]
#[command(name = "pulldown-cmark-event-view", author = "nth233", about)]
struct Cli {
    #[clap(long, short = 'i')]
    input_file: PathBuf,
    #[clap(long, short = 's')]
    final_stage: bool,
}

fn main() {
    let cli = Cli::parse();

    println!("[config]\n{:?}", cli);

    let mut options = Options::empty();
    options.insert(Options::ENABLE_MATH);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_SUPERSCRIPT);
    options.insert(Options::ENABLE_SUBSCRIPT);

    let content = std::fs::read(&cli.input_file).unwrap();
    let content_utf8 = String::from_utf8(content).unwrap();

    let parser = Parser::new_ext(&content_utf8, options);

    let math_event_processor = Box::new(RefCell::new(MathEventProcessor::new()));
    let highlight_event_processor = Box::new(RefCell::new(HighlightEventProcessor::new()));
    let image_event_processor = Box::new(RefCell::new(ImageEventProcessor::new()));
    let debug_event_processor = Box::new(RefCell::new(DebugEventProcessor {}));

    if cli.final_stage {
        let html = render_pipeline!(
            parser,
            math_event_processor,
            highlight_event_processor,
            image_event_processor,
            debug_event_processor
        );
        println!("[output]\n{}", html);
    } else {
        let html = render_pipeline!(
            parser,
            debug_event_processor
        );
        println!("[output]\n{}", html);
    }
}
