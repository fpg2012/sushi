use liquid::partials::{EagerCompiler, InMemorySource};
use liquid::ParserBuilder;
use log::{debug, error};
use serde_yaml::Value;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::fs::DirEntry;
use std::option::Option;
use std::path::PathBuf;
use std::string::String;
use std::cell::RefCell;
use std::rc::Rc;

use crate::converters::{ExternalConverter, Converter, DummyConverter};
use crate::markdown_parser::MarkdownParser;
use crate::extract_frontmatter::extract_front_matter;
use crate::layout::Layout;

pub fn parse_config_file(path: PathBuf) -> HashMap<String, Value> {
    let raw_config = fs::read(&path).expect("cannot read config file");

    let mut is_toml = false;
    if let Some(ext_name) = path.extension() {
        if ext_name == "toml" {
            is_toml = true;
        }
    }

    let mut config: HashMap<String, Value> = if is_toml {
        toml::from_slice(raw_config.as_slice()).expect("cannot parse toml config file")
    } else {
        serde_yaml::from_slice(raw_config.as_slice()).expect("cannot parse yaml config file")
    };
    config.insert(
        String::from("time"),
        Value::String(chrono::Local::now().to_rfc3339()),
    );
    debug!("{:?}", config);
    config
}

pub fn string_from_config(
    key: &str,
    config: &HashMap<String, serde_yaml::Value>,
) -> Option<String> {
    if let Some(serde_yaml::Value::String(s)) = config.get(key) {
        Some(s.clone())
    } else {
        None
    }
}

pub fn parse_includes(path: PathBuf) -> HashMap<String, PathBuf> {
    let mut partial_list: HashMap<String, PathBuf> = HashMap::new();
    if let Ok(dir) = fs::read_dir(path) {
        for entry in dir {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "liquid" {
                        partial_list.insert(
                            entry
                                .path()
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            entry.path().clone(),
                        );
                        debug!(
                            "[discover] partial: \"{}\"",
                            entry.path().file_stem().unwrap().to_string_lossy()
                        );
                    }
                }
            }
        }
    }
    partial_list
}

pub fn compile_partials(partial_list: HashMap<String, PathBuf>) -> EagerCompiler<InMemorySource> {
    let mut compiler = EagerCompiler::<InMemorySource>::empty();
    for (partial_name, partial_path) in partial_list {
        let content =
            fs::read_to_string(&partial_path).expect("cannot open liquid partial in include");
        compiler.add(partial_name, content);
        debug!(
            "[compile] partial: \"{}\"",
            &partial_path.file_stem().unwrap().to_string_lossy()
        );
    }
    compiler
}

pub fn parse_templates(path: PathBuf) -> HashMap<String, PathBuf> {
    let mut template_list: HashMap<String, PathBuf> = HashMap::new();
    for entry in fs::read_dir(path).expect("cannot open _template dir") {
        if let Ok(entry) = entry {
            if let Some(ext) = entry.path().extension() {
                if ext == "liquid" {
                    template_list.insert(
                        entry
                            .path()
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .to_string(),
                        entry.path(),
                    );
                    debug!(
                        "[discover] template: \"{}\"",
                        entry.path().file_stem().unwrap().to_string_lossy()
                    );
                }
            }
        }
    }
    template_list
}

pub fn compile_templates(
    partials: EagerCompiler<InMemorySource>,
    template_list: HashMap<String, PathBuf>,
) -> HashMap<String, Layout> {
    let mut templates = HashMap::new();
    let parser = ParserBuilder::with_stdlib()
        .partials(partials)
        .build()
        .unwrap();
    for (template_name, template_path) in template_list {
        let (fm, real_content) = extract_front_matter(&template_path);
        let fm = match fm {
            Some(fm) => fm,
            None => HashMap::new(),
        };
        let template = parser.parse(real_content.as_str());
        if let Err(e) = template {
            error!("{}", e);
            panic!("compile template error");
        }
        let template = template.unwrap();
        let layout = Layout::new(fm, template);
        debug!("[compile] template: \"{}\"", &template_name);
        templates.insert(template_name, layout);
    }
    templates
}

pub fn parse_converters(path: PathBuf) -> HashMap<String, PathBuf> {
    let mut converter_list: HashMap<String, PathBuf> = HashMap::new();
    for entry in fs::read_dir(path).expect("cannot open _converter dir") {
        if let Ok(entry) = entry {
            converter_list.insert(
                entry.file_name().to_string_lossy().to_string(),
                entry.path(),
            );
            debug!(
                "[discover] converter: \"{}\"",
                entry.file_name().to_string_lossy().to_string()
            );
        }
    }
    converter_list
}

pub fn load_converters(
    converter_list: HashMap<String, PathBuf>,
) -> HashMap<String, Rc<RefCell<dyn Converter>>> {
    let mut converters: HashMap<String, Rc<RefCell<dyn Converter>>> = HashMap::new();
    for (converter_name, converter_path) in converter_list {
        debug!("[compile] converter: \"{}\"", &converter_name);
        // external converter
        converters.insert(
            converter_name.clone(),
            Rc::new(RefCell::new(ExternalConverter {
                name: converter_name,
                path: converter_path,
            })),
        );
    }
    // dummy converter for copy
    converters.insert(
        "__copy__".to_string(),
        Rc::new(RefCell::new(DummyConverter {})),
    );
    // internal converter (markdown only for now)
    converters.insert(
        "__internal__".to_string(),
        Rc::new(RefCell::new(MarkdownParser::new())),
    );
    converters
}

pub fn find_dir(base_dir: &PathBuf, dir_name: &String) -> Option<Result<DirEntry, std::io::Error>> {
    let entry = fs::read_dir(base_dir.clone()).unwrap().find(|x| {
        if let Ok(file) = x {
            file.file_name() == OsString::from(&dir_name) && file.path().is_dir()
        } else {
            false
        }
    });
    entry
}

pub fn find_dir_or_panic(base_dir: &PathBuf, dir_name: &String) -> fs::DirEntry {
    let entry = find_dir(&base_dir, &dir_name)
        .expect(format!("cannot find {} dir", &dir_name).as_str())
        .unwrap();
    entry
}
