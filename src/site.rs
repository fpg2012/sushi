use crate::converters::Converter;
use crate::site::SiteTreeNode::*;
use crate::layout::Layout;
use liquid::partials::{EagerCompiler, InMemorySource};
use liquid::{ParserBuilder, Template};
use log::{debug, info};
use serde_frontmatter;
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::option::Option;
use std::path::PathBuf;
use std::rc::Rc;
use std::string::String;
use chrono;
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
}

pub struct Site {
    site_dir: PathBuf,
    config: HashMap<String, serde_yaml::Value>,
    templates: HashMap<String, Layout>,
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
        let site_tree = Rc::new(RefCell::new(NormalDir {
            children: vec![],
            path: site_dir.clone(),
            index: None,
        }));
        Self::_gen_tree(site_tree.clone());
        debug!("{:?}", site_tree);

        let mut gen_dir = site_dir.clone();
        gen_dir.push("_gen");

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
    pub fn generate_site(&self) {
        self._generate(self.site_tree.clone());
    }

    pub fn _generate(&self, current_node: NodeRef) -> bool {
        match &mut *current_node.clone().borrow_mut() {
            SiteTreeNode::NormalDir {
                children,
                path,
                index,
            } => {
                let dest_path = self._get_dest_path(path, false);
                debug!("try create dir {:?}", dest_path);
                fs::create_dir_all(dest_path).expect("cannot create dir");
                for child in children.iter() {
                    if self._generate(child.clone()) {
                        *index = Some(child.clone());
                        debug!("{:?} has index", path);
                    }
                }
                false
            }
            SiteTreeNode::NormalFile { path, page } => {
                if self.convert_ext.contains(
                    &path
                        .extension()
                        .unwrap_or(OsString::from("no_ext").as_os_str())
                        .to_string_lossy()
                        .to_string(),
                ) {
                    debug!("gen {}", path.clone().to_string_lossy());
                    self.convert_page(path, page);
                } else {
                    let dest_path = self._get_dest_path(path, false);
                    debug!("copy {}", path.clone().to_string_lossy());
                    fs::copy(path.clone(), dest_path).unwrap();
                }
                path.file_stem().unwrap_or(OsStr::new("")) == OsStr::new("index")
                    && self.convert_ext.contains(
                        &path
                            .extension()
                            .unwrap_or(OsStr::new(""))
                            .to_string_lossy()
                            .to_string(),
                    )
            }
            _ => panic!("unknown node type"),
        }
    }

    pub fn convert_page(&self, path: &PathBuf, page: &mut Option<Page>) {
        let dest_path = self._get_dest_path(path, true);
        let full_page = fs::read_to_string(path).expect("cannot read file");
        let (mut fm, content) =
            serde_frontmatter::deserialize::<HashMap<String, Value>>(full_page.as_str())
                .unwrap_or((HashMap::new(), full_page));

        let mut converted = content;
        let mut converter_choice = String::new();
        if let Some(choice) = self.converter_choice
            .get(path.extension().unwrap_or(OsStr::new("")).to_str().unwrap())
        {
            converter_choice = choice.clone();
        }

        if let Some(converter) = self.converters.get(&converter_choice) {
            converted = converter.convert(converted);
        } else {
            debug!("no converter set");
        }

        fm.insert(String::from("url"), Value::String(self._get_page_url(path)));
        let layout = fm.get("layout");
        let mut rendered = converted;
        if let Some(Value::String(s)) = layout {
            debug!("try to use layout {}", s);
            let mut l = s;
            while let Some(template) = self.templates.get(l) {
                debug!("current template {}", l);
                let mut globals = liquid::object!({
                    "site": self.config,
                    "page": fm,
                    "content": rendered,
                });
                let ren = template.render(&mut globals).expect("render failed");
                rendered = ren;
                l = template.get_parent();
            }
        } else {
            debug!("no layout set");
        }

        *page = Some(Page { front_matter: fm });
        match fs::write(&dest_path, rendered) {
            Ok(_) => (),
            Err(_) => info!("cannot write to {:?}", dest_path),
        }
    }

    fn _get_page_url(&self, path: &PathBuf) -> String {
        let temp = path.strip_prefix(&self.site_dir).unwrap();
        String::from("/") + temp.to_str().unwrap()
    }

    fn _get_converter_dir(&self) -> PathBuf {
        let mut temp = PathBuf::from(&self.site_dir);
        temp.push("_converters");
        temp
    }

    fn _get_dest_path(&self, path: &PathBuf, is_page: bool) -> PathBuf {
        let mut dest = PathBuf::from(&self.gen_dir);
        dest.push(path.strip_prefix(&self.site_dir).unwrap());
        if is_page {
            dest.set_extension("html");
        }
        dest
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
                ..
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
        let mut config: HashMap<String, Value> =
            serde_yaml::from_slice(raw_config.as_slice()).expect("cannot parse config file");
        config.insert(String::from("time"), Value::String(chrono::Local::now().to_rfc3339()));
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
    ) -> HashMap<String, Layout> {
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
                        let (fm, real_content) = serde_frontmatter::deserialize(content.as_str())
                            .unwrap_or((HashMap::new(), content));
                        let template = parser
                            .parse(real_content.as_str())
                            .expect("compiler template faild");
                        let layout = Layout::new(fm, template);
                        templates.insert(
                            entry
                                .path()
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            layout,
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
