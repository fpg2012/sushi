use log::{info, warn};
use serde_yaml::Value;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::PathBuf;
use std::string::String;

use crate::configuration_loader as confld;

#[allow(dead_code)]
pub struct Theme {
    pub theme_dir: PathBuf,
    pub config: HashMap<String, Value>,
    pub partial_list: HashMap<String, PathBuf>,
    pub converter_list: HashMap<String, PathBuf>,
    pub template_list: HashMap<String, PathBuf>,
    pub theme_name: String,
}

impl Theme {
    pub fn new(theme_dir: PathBuf) -> Theme {
        let temp_config = fs::read_dir(theme_dir.clone())
            .expect("cannot open theme directory.")
            .find(|x| {
                if let Ok(file) = x {
                    file.file_name() == OsString::from("_site.yml") && file.path().is_file()
                } else {
                    false
                }
            })
            .expect(format! {"cannot find theme configuration file: {}", "_site.yml"}.as_str())
            .unwrap();
        let config = confld::parse_config_file(temp_config.path());

        let theme_name = Self::_get_theme_name(&config);

        info!("[theme] theme_name: {}", &theme_name);

        let theme_converters_dir = confld::string_from_config("converters_dir", &config);
        let theme_templates_dir = confld::string_from_config("templates_dir", &config);
        let theme_includes_dir = confld::string_from_config("includes_dir", &config);

        let _converters_dir =
            Self::_decide_theme_config(theme_converters_dir, "_converters".to_string());
        let _templates_dir =
            Self::_decide_theme_config(theme_templates_dir, "_templates".to_string());
        let _includes_dir = Self::_decide_theme_config(theme_includes_dir, "_includes".to_string());

        // search for _includes
        let temp_includes = confld::find_dir(&theme_dir, &_includes_dir);
        let partial_list = if let Some(Ok(temp)) = temp_includes {
            confld::parse_includes(temp.path())
        } else {
            warn!("no theme include template found");
            HashMap::new()
        };

        // search for _templates
        let temp_templates = confld::find_dir_or_panic(&theme_dir, &_templates_dir);
        let template_list = confld::parse_templates(temp_templates.path());

        // search for _converters
        let temp_converters = confld::find_dir_or_panic(&theme_dir, &_converters_dir);
        let converter_list = confld::parse_converters(temp_converters.path());

        Theme {
            theme_dir,
            config,
            partial_list,
            converter_list,
            template_list,
            theme_name,
        }
    }

    fn _decide_theme_config(yml_config: Option<String>, default_config: String) -> String {
        // command line configuration is prior to _site.yml configuration
        if let Some(yml_str) = yml_config {
            yml_str
        } else {
            default_config
        }
    }

    fn _get_theme_name(config: &HashMap<String, Value>) -> String {
        if let Some(Value::String(name)) = config.get("theme_name") {
            name.clone()
        } else {
            panic!("theme_name not found in theme _site.yml");
        }
    }
}
