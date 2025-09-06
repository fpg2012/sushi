use katex;
use pulldown_cmark::Event;

pub struct MathEventProcessor {
    display_style_opts: katex::opts::Opts,
}

impl MathEventProcessor {
    pub fn new() -> MathEventProcessor {
        let opts = katex::Opts::builder().display_mode(true).build().unwrap();
        MathEventProcessor {
            display_style_opts: opts,
        }
    }

    pub fn process_math_event<'a>(&'a self, event: Event<'a>) -> Event<'a> {
        match event {
            Event::InlineMath(math_exp) => {
                Event::InlineHtml(katex::render(&math_exp).unwrap().into())
            }
            Event::DisplayMath(math_exp) => Event::Html(
                katex::render_with_opts(&math_exp, &self.display_style_opts)
                    .unwrap()
                    .into(),
            ),
            _ => event,
        }
    }
}
