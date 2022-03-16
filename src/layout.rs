use liquid;
use log::trace;
use serde_yaml::Value;
use std::collections::HashMap;

pub struct Layout {
    parent: String,
    template: liquid::Template,
    front_matter: HashMap<String, Value>,
}

impl Layout {
    pub fn new(fm: HashMap<String, Value>, template: liquid::Template) -> Self {
        let mut parent = String::new();
        if let Some(Value::String(p)) = fm.get("layout") {
            parent = p.clone();
        }
        Self {
            parent,
            template,
            front_matter: fm,
        }
    }

    pub fn get_parent(&self) -> &String {
        &self.parent
    }

    pub fn render(&self, globals: &mut liquid::Object) -> Result<String, liquid::Error> {
        if let Some(liquid::model::Value::Object(page)) = globals.get_mut("page") {
            let temp = liquid::object!(self.front_matter);
            page.extend(temp.iter().filter_map(|(k, v)| {
                if k == "layout" {
                    None
                } else {
                    Some((k.clone(), v.clone()))
                }
            }));
        } else {
            globals.insert(
                "page".parse().unwrap(),
                liquid::model::value!(self.front_matter),
            );
        }
        let temp = serde_yaml::to_string(&globals).unwrap_or("error".to_string());
        trace!("globals {}", temp);
        self.template.render(globals)
    }
}
