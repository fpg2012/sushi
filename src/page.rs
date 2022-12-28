use chrono::{DateTime, Local, NaiveDateTime, NaiveTime, TimeZone};
use itertools::Itertools;
use log::debug;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

pub type PageRef = Rc<RefCell<Page>>;
pub type PageId = String;

#[derive(Debug, Clone)]
pub struct Page {
    pub front_matter: HashMap<String, serde_yaml::Value>,
    // pub other_attributes: HashMap<String, serde_yaml::Value>,
    pub url: String,
    pub path: PathBuf,
    date: chrono::DateTime<Local>,
    next: Option<PageRef>,
    last: Option<PageRef>,
    page_id: PageId,
    pub to_ext: Option<String>,
}

impl Page {
    pub fn new(
        front_matter: HashMap<String, serde_yaml::Value>,
        url: String,
        path: PathBuf,
        to_ext: Option<String>,
    ) -> Self {
        // get or gen date
        let date = if let Some(serde_yaml::Value::String(date)) = front_matter.get("date") {
            match chrono::DateTime::parse_from_rfc3339(date) {
                Ok(date) => DateTime::<Local>::from(date),
                Err(_) => match chrono::NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S") {
                    Ok(date) => Local.from_local_datetime(&date).unwrap(),
                    Err(_) => match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
                        Ok(date) => Local
                            .from_local_datetime(&NaiveDateTime::new(
                                date,
                                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                            ))
                            .unwrap(),
                        Err(_) => DateTime::<Local>::from(SystemTime::now()),
                    },
                },
            }
        } else {
            debug!("date is not defined in front_matter, use system time");
            chrono::DateTime::from(chrono::DateTime::<Local>::from(SystemTime::now()))
        };
        debug!("date: {}", date);
        // get or gen id
        let page_id = if let Some(serde_yaml::Value::String(id)) = front_matter.get("page_id") {
            id.clone()
        } else {
            let mut s = DefaultHasher::new();
            url.hash(&mut s);
            // date.hash(&mut s);
            s.finish().to_string()
        };
        Self {
            front_matter,
            url,
            date,
            next: None,
            last: None,
            page_id,
            path,
            to_ext,
        }
    }

    pub fn get_front_matter(&self) -> &HashMap<String, serde_yaml::Value> {
        &self.front_matter
    }

    pub fn get_page_config_object(&self) -> serde_yaml::Mapping {
        serde_yaml::Mapping::from_iter(
            self.get_page_config()
                .iter()
                .map(|(k, v)| (serde_yaml::Value::String(k.clone()), v.clone())),
        )
    }

    pub fn get_page_config(&self) -> HashMap<String, serde_yaml::Value> {
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
        if config.get("page_id") == None {
            config.insert(
                "page_id".to_string(),
                serde_yaml::Value::String(self.page_id.clone()),
            );
        }
        if config.get("next") == None {
            if let Some(next) = &self.next {
                config.insert(
                    "next".to_string(),
                    serde_yaml::Value::String(next.borrow().get_page_id().clone()),
                );
            }
        }
        if config.get("last") == None {
            if let Some(last) = &self.last {
                config.insert(
                    "last".to_string(),
                    serde_yaml::Value::String(last.borrow().get_page_id().clone()),
                );
            }
        }
        config.insert(
            "path".to_string(),
            serde_yaml::Value::String(self.path.to_string_lossy().to_string()),
        );
        config
    }

    pub fn get_page_id(&self) -> &String {
        &self.page_id
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

    pub fn date(&self) -> &chrono::DateTime<Local> {
        &self.date
    }

    pub fn paginate_info(&self) -> Option<(String, usize)> {
        match (
            self.front_matter.get("paginate"),
            self.front_matter.get("paginate_batch"),
        ) {
            (Some(serde_yaml::Value::String(exp)), Some(serde_yaml::Value::Number(num))) => {
                if num.is_u64() {
                    Some((exp.clone(), num.as_u64().unwrap() as usize))
                } else {
                    None
                }
            }
            (_, _) => None,
        }
    }

    pub fn belongs_to_kind(&self, taxo: &String) -> Vec<String> {
        if let Some(t) = self.front_matter.get(taxo) {
            if let serde_yaml::Value::Sequence(sq) = t {
                sq.iter()
                    .filter_map(|x| {
                        if let serde_yaml::Value::String(s) = x {
                            Some(s.clone())
                        } else {
                            None
                        }
                    })
                    .collect_vec()
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    }
}
