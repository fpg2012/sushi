use crate::converters::Converter;
use crate::site::SiteTreeNode::*;
use liquid::partials::{EagerCompiler, InMemorySource};
use liquid::{ParserBuilder, Template};
use log::debug;
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::option::Option;
use std::path::PathBuf;
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;

type NodeRef = Rc<RefCell<SiteTreeNode>>;

#[derive(Debug)]
pub enum SiteTreeNode {
    Unknown,
    NormalDir {
        children: Vec<NodeRef>,
        path: PathBuf,
        index: Option<NodeRef>,
    },
    NormalFile {
        path: PathBuf,
        page: Option<Page>,
    },
}

#[derive(Debug)]
pub struct Page {
    front_matter: HashMap<String, serde_yaml::Value>,
    original_content: Rc<Vec<u8>>,
}

pub struct Site {
    site_dir: PathBuf,
    config: HashMap<String, serde_yaml::Value>,
    templates: HashMap<String, liquid::Template>,
    converters: HashMap<String, Converter>,
    gen_dir: PathBuf,
    site_tree: NodeRef,

    convert_ext: HashSet<String>,
    converter_choice: HashMap<String, String>,
    taxonomies: HashSet<String>,
}

impl Site {
    pub fn parse_site_dir(site_dir: PathBuf) -> Self {
        // search for _site.yml
        let temp_config = fs::read_dir(site_dir.clone())
            .expect("cannot open site directory.")
            .find(|x| {
                if let Ok(file) = x {
                    file.file_name() == OsString::from("_site.yml") && file.path().is_file()
                } else {
                    false
                }
            })
            .expect("cannot find configuration file: _site.yml")
            .unwrap();
        let config = Self::_parse_config_file(temp_config.path());

        // search for _includes
        let temp_includes = fs::read_dir(site_dir.clone()).unwrap().find(|x| {
            if let Ok(file) = x {
                file.file_name() == OsString::from("_includes") && file.path().is_dir()
            } else {
                false
            }
        });
        let partial_compiler = if let Some(Ok(temp)) = temp_includes {
            Self::_parse_includes(temp.path())
        } else {
            debug!("no include template found");
            liquid::partials::EagerCompiler::empty()
        };

        // search for _templates
        let temp_templates = fs::read_dir(site_dir.clone())
            .unwrap()
            .find(|x| {
                if let Ok(file) = x {
                    file.file_name() == OsString::from("_templates") && file.path().is_dir()
                } else {
                    false
                }
            })
            .expect("cannot find _template dir")
            .unwrap();
        let templates = Self::_parse_templates(temp_templates.path(), partial_compiler);

        // search for _converters
        let temp_converters = fs::read_dir(site_dir.clone())
            .unwrap()
            .find(|x| {
                if let Ok(file) = x {
                    file.file_name() == OsString::from("_converters") && file.path().is_dir()
                } else {
                    false
                }
            })
            .expect("cannot find _converter dir")
            .unwrap();
        let converters = Self::_parse_converters(temp_converters.path());

        // parse dir
        let mut site_tree = Rc::new(RefCell::new(NormalDir {
            children: vec![],
            path: site_dir.clone(),
            index: None,
        }));
        Self::_gen_tree(site_tree.clone());
        debug!("{:?}", site_tree);

        let gen_dir = site_dir.clone();

        let (convert_ext, converter_choice, taxonomies) = Self::_extract_important_config(&config);
        debug!("{:?}", convert_ext);
        debug!("{:?}", converter_choice);
        debug!("{:?}", taxonomies);

        Site {
            site_dir,
            config,
            templates,
            converters,
            gen_dir,
            site_tree,
            convert_ext,
            converter_choice,
            taxonomies,
        }
    }
    fn _extract_important_config(
        config: &HashMap<String, Value>,
    ) -> (HashSet<String>, HashMap<String, String>, HashSet<String>) {
        let mut convert_ext = HashSet::new();
        if let Some(Value::Sequence(ext)) = config.get("convert_ext") {
            convert_ext.extend(ext.iter().filter_map(|x| {
                if let Value::String(s) = x {
                    Some(s.clone())
                } else {
                    None
                }
            }));
        }

        let mut converter_choice = HashMap::new();
        if let Some(Value::Mapping(choice)) = config.get("converter_choice") {
            for (f, t) in choice.iter() {
                if let (Value::String(ext), Value::String(conv)) = (f, t) {
                    converter_choice.insert(ext.clone(), conv.clone());
                }
            }
        }

        let mut taxonomies = HashSet::new();
        if let Some(Value::Sequence(taxo)) = config.get("taxonomies") {
            taxonomies.extend(taxo.iter().filter_map(|x| {
                if let Value::String(s) = x {
                    Some(s.clone())
                } else {
                    None
                }
            }));
        }

        (convert_ext, converter_choice, taxonomies)
    }

    fn _gen_tree(current_node: NodeRef) {
        let cnode = current_node.clone();
        match &mut *cnode.borrow_mut() {
            NormalDir {
                children,
                path,
                index,
            } => {
                debug!("scan {}", path.to_string_lossy());
                for entry in fs::read_dir(path.clone()).unwrap() {
                    if let Ok(entry) = entry {
                        if entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(|x: char| x == '.' || x == '_')
                        {
                            debug!("ignore file {}", entry.file_name().to_string_lossy());
                            continue;
                        }
                        if entry.path().is_dir() {
                            let new_node = Rc::new(RefCell::new(SiteTreeNode::NormalDir {
                                children: vec![],
                                path: entry.path(),
                                index: None,
                            }));
                            Self::_gen_tree(new_node.clone());
                            children.push(new_node);
                        } else if entry.path().is_file() {
                            debug!("scan file {}", entry.path().to_string_lossy());
                            let new_node = Rc::new(RefCell::new(SiteTreeNode::NormalFile {
                                path: entry.path(),
                                page: None,
                            }));
                            children.push(new_node);
                        } else {
                            panic!("unknown file");
                        }
                    } else {
                        debug!("cannot open dir {}", path.to_string_lossy());
                    }
                }
            }
            _ => (),
        };
    }
    fn _parse_config_file(path: PathBuf) -> HashMap<String, Value> {
        let raw_config = fs::read(path).expect("cannot read config file");
        let config: HashMap<String, Value> =
            serde_yaml::from_slice(raw_config.as_slice()).expect("cannot parse config file");
        debug!("{:?}", config);
        config
    }
    fn _parse_includes(path: PathBuf) -> EagerCompiler<InMemorySource> {
        let mut compiler = EagerCompiler::<InMemorySource>::empty();
        if let Ok(dir) = fs::read_dir(path) {
            for entry in dir {
                if let Ok(entry) = entry {
                    if let Some(ext) = entry.path().extension() {
                        if ext == "liquid" {
                            let content = fs::read_to_string(entry.path())
                                .expect("cannot open liquid partial in include");
                            compiler
                                .add(entry.path().file_stem().unwrap().to_string_lossy(), content);
                            debug!(
                                "add {} to partials",
                                entry.path().file_stem().unwrap().to_string_lossy()
                            );
                        }
                    }
                }
            }
        }
        compiler
    }
    fn _parse_templates(
        path: PathBuf,
        partials: EagerCompiler<InMemorySource>,
    ) -> HashMap<String, Template> {
        let mut templates = HashMap::new();
        let parser = ParserBuilder::with_stdlib()
            .partials(partials)
            .build()
            .unwrap();
        for entry in fs::read_dir(path).expect("cannot open _template dir") {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "liquid" {
                        let content =
                            fs::read_to_string(entry.path()).expect("cannot open template file");
                        let template = parser
                            .parse(content.as_str())
                            .expect("compiler template faild");
                        templates.insert(
                            entry
                                .path()
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            template,
                        );
                        debug!(
                            "add {} to templates",
                            entry.path().file_stem().unwrap().to_string_lossy()
                        );
                    }
                }
            }
        }
        templates
    }
    fn _parse_converters(path: PathBuf) -> HashMap<String, Converter> {
        let mut converters = HashMap::new();
        for entry in fs::read_dir(path).expect("cannot open _converter dir") {
            if let Ok(entry) = entry {
                converters.insert(
                    entry.file_name().to_string_lossy().to_string(),
                    Converter {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry.path(),
                    },
                );
                debug!(
                    "find converter {}",
                    entry.file_name().to_string_lossy().to_string()
                );
            }
        }
        converters
    }
}
