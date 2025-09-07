use super::event_processor::EventProcessor;
use pulldown_cmark::{Event, LinkType, Tag, TagEnd};

#[derive(Debug, Eq, PartialEq)]
pub enum State {
    NotImage,
    InParagraph,
    InImage,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ImageProperties {
    pub link_type: LinkType,
    pub dest_url: String,
    pub title: String,
    pub id: String,
}

pub struct ImageEventProcessor {
    pub state: State,
    image_properties: Option<ImageProperties>,
    caption: String,
}

impl ImageEventProcessor {
    pub fn new() -> ImageEventProcessor {
        ImageEventProcessor {
            state: State::NotImage,
            image_properties: None,
            caption: String::new(),
        }
    }

    fn format_html(image_properties: ImageProperties, caption: &str) -> String {
        format!(
            "<figure><img src='{}' alt='{}' id='{}' title='{}' /><figcaption>{}</figcaption></figure>",
            image_properties.dest_url,
            caption,
            image_properties.id,
            image_properties.title,
            caption,
        )
    }

    pub fn process_image_event<'a>(&mut self, event: Event<'a>) -> Vec<Event<'a>> {
        match event {
            Event::Start(Tag::Paragraph) => {
                self.state = State::InParagraph;
                vec![]
            }
            Event::Start(Tag::Image {
                link_type,
                dest_url,
                title,
                id,
            }) => {
                self.state = State::InImage;
                self.image_properties = Some(ImageProperties {
                    link_type,
                    dest_url: dest_url.to_string(),
                    title: title.to_string(),
                    id: id.to_string(),
                });

                vec![Event::Start(Tag::HtmlBlock)]
            }
            Event::Text(t) => {
                if self.state == State::InImage {
                    // append text to caption
                    self.caption.push_str(&t);
                    vec![]
                } else {
                    vec![Event::Text(t)]
                }
            }
            Event::End(TagEnd::Image) => {
                let image_properties = self.image_properties.clone().unwrap();
                let html = Self::format_html(image_properties, self.caption.as_str());

                self.image_properties = None;
                self.caption = String::new();

                vec![Event::Html(html.into())]
            }
            Event::End(TagEnd::Paragraph) => {
                if self.state == State::InImage {
                    self.state = State::NotImage;
                    vec![]
                } else {
                    vec![event]
                }
            }
            _ => {
                if self.state == State::InParagraph {
                    self.state = State::NotImage;
                    vec![Event::Start(Tag::Paragraph), event]
                } else {
                    vec![event]
                }
            }
        }
    }
}

impl EventProcessor for ImageEventProcessor {
    fn apply<'a>(
        &'a mut self,
        iter: impl Iterator<Item = Event<'a>>,
    ) -> impl Iterator<Item = Event<'a>> {
        iter.map(move |event| self.process_image_event(event))
            .flat_map(|event| event.into_iter())
    }
}
