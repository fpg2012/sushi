use chrono::{FixedOffset, Local};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::SystemTime;
use itertools::Itertools;

pub type PageRef = Rc<RefCell<Page>>;

#[derive(Debug, Clone)]
pub struct Page {
    pub front_matter: HashMap<String, serde_yaml::Value>,
    // pub other_attributes: HashMap<String, serde_yaml::Value>,
    pub url: String,
    date: chrono::DateTime<FixedOffset>,
    next: Option<PageRef>,
    last: Option<PageRef>,
}

impl Page {
    pub fn new(front_matter: HashMap<String, serde_yaml::Value>, url: String) -> Self {
        let date = if let Some(serde_yaml::Value::String(date)) = front_matter.get("date") {
            chrono::DateTime::parse_from_rfc3339(date).unwrap_or(chrono::DateTime::from(
                chrono::DateTime::<Local>::from(SystemTime::now()),
            ))
        } else {
            chrono::DateTime::from(chrono::DateTime::<Local>::from(SystemTime::now()))
        };
        Self {
            front_matter,
            url,
            date,
            next: None,
            last: None,
        }
    }

    pub fn get_front_matter(&self) -> &HashMap<String, serde_yaml::Value> {
        &self.front_matter
    }

    pub fn get_page_config_object(&self, with_siblings: bool) -> serde_yaml::Mapping {
        serde_yaml::Mapping::from_iter(self.get_page_config(with_siblings).iter().map(|(k, v)| {
            (serde_yaml::Value::String(k.clone()), v.clone())
        }))
    }

    pub fn get_page_config(&self, with_siblings: bool) -> HashMap<String, serde_yaml::Value> {
        let mut config = HashMap::new();
        config.extend(
            self.front_matter
                .iter()
                .map(|(k, v)| (k.clone(), v.clone())),
        );
        if config.get("url") == None {
            config.insert(
                "url".to_string(),
                serde_yaml::Value::String(self.url.clone()),
            );
        }
        if config.get("date") == None {
            config.insert(
                "date".to_string(),
                serde_yaml::Value::String(self.date.to_string()),
            );
        }
        if with_siblings {
            if config.get("next") == None {
                if let Some(next) = &self.next {
                    let temp = next.clone();
                    let temp = temp.borrow().get_page_config(false);
                    config.insert(
                        "next".to_string(),
                        serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
                            temp.iter()
                                .map(|(k, v)| (serde_yaml::Value::String(k.clone()), v.clone())),
                        )),
                    );
                }
            }
        }
        config
    }

    pub fn set_next(&mut self, next: Option<PageRef>) {
        self.next = next;
    }

    pub fn set_last(&mut self, last: Option<PageRef>) {
        self.last = last;
    }

    pub fn next(&self) -> &Option<PageRef> {
        &self.next
    }

    pub fn last(&self) -> &Option<PageRef> {
        &self.last
    }

    pub fn date(&self) -> &chrono::DateTime<FixedOffset> {
        &self.date
    }

    pub fn belongs_to_kind(&self, taxo: &String) -> Vec<String> {
        if let Some(t) = self.front_matter.get(taxo) {
            if let serde_yaml::Value::Sequence(sq) = t {
                sq.iter().filter_map(|x| {
                    if let serde_yaml::Value::String(s) = x {
                        Some(s.clone())
                    } else {
                        None
                    }
                }).collect_vec()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}
