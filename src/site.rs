use crate::converters::Converter;
use crate::layout::Layout;
use crate::page::{Page, PageRef};
use crate::site::SiteTreeNode::*;
use chrono;
use itertools::Itertools;
use liquid::partials::{EagerCompiler, InMemorySource};
use liquid::ParserBuilder;
use log::{debug, error, info, warn};
use serde_frontmatter;
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::string::String;
use std::vec::Vec;

type NodeRef = Rc<RefCell<SiteTreeNode>>;
type SiteTreeObject = serde_yaml::Value;

#[derive(Debug)]
pub enum SiteTreeNode {
    Unknown,
    NormalDir {
        children: Vec<NodeRef>,
        path: PathBuf,
        index: Option<PageRef>,
    },
    PageFile {
        path: PathBuf,
        page: PageRef,
    },
    StaticFile {
        path: PathBuf,
    },
}

enum SiteTreeObjectType {
    Unknown,
    Dir(String),                                 // dir name
    DirWithIndexPage(String, serde_yaml::Value), // dir name, page object
    Page,
}

pub struct Site {
    site_dir: PathBuf,
    config: HashMap<String, serde_yaml::Value>,
    templates: HashMap<String, Layout>,
    converters: HashMap<String, Converter>,
    gen_dir: PathBuf,
    site_tree: Option<NodeRef>,

    convert_ext: HashSet<String>,
    converter_choice: HashMap<String, String>,
    taxonomies: HashMap<String, HashMap<String, RefCell<Vec<SiteTreeObject>>>>,

    site_tree_object: Option<serde_yaml::Value>,
    taxo_object: Option<serde_yaml::Value>,
    pages: Vec<PageRef>,
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
            warn!("no include template found");
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
            site_tree: None,
            convert_ext,
            converter_choice,
            taxonomies,
            site_tree_object: None,
            taxo_object: None,
            pages: vec![],
        }
    }
    pub fn generate_site(&mut self) {
        let (site_tree, _) = self._gen_site_tree(&self.site_dir.clone());
        self.site_tree = Some(site_tree);
        let (site_tree_object, _) = self._gen_site_tree_object(self.site_tree.clone().unwrap());
        self.site_tree_object = site_tree_object;
        self._gen_taxo_object();
        let temp = serde_yaml::to_string(&self.taxo_object).unwrap_or("error".to_string());
        debug!("{}", temp);
        self._generate(self.site_tree.clone().unwrap());
    }

    fn _generate(&self, current_node: NodeRef) {
        match &mut *current_node.clone().borrow_mut() {
            SiteTreeNode::NormalDir { children, path, .. } => {
                let dest_path = self._get_dest_path(path, false);
                info!("[create]\t {:?}", dest_path);
                fs::create_dir_all(dest_path).expect("cannot create dir");
                for child in children.iter() {
                    self._generate(child.clone());
                }
            }
            SiteTreeNode::PageFile { path, page } => {
                info!("[gen]\t {}", path.clone().to_string_lossy());
                self.convert_page(path, page.clone());
            }
            SiteTreeNode::StaticFile { path } => {
                let dest_path = self._get_dest_path(path, false);
                info!("[copy]\t {}", path.clone().to_string_lossy());
                fs::copy(path.clone(), dest_path).unwrap();
            }
            _ => panic!("unknown node type"),
        }
    }

    fn _gen_site_tree(&mut self, path: &PathBuf) -> (NodeRef, Option<PageRef>) {
        if path.is_dir() {
            let mut children: Vec<NodeRef> = vec![];
            let mut index: Option<PageRef> = None;
            for entry in path.read_dir().unwrap() {
                if let Ok(entry) = entry {
                    if entry
                        .path()
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(|ch: char| ch == '.' || ch == '_')
                    {
                        continue;
                    }
                    let (child, index_) = self._gen_site_tree(&entry.path());
                    index = index_;
                    children.push(child);
                }
            }
            // set page.next and page.last
            let mut list = children
                .iter()
                .filter_map(|x| match &*x.borrow() {
                    PageFile { page, .. } => Some(page.clone()),
                    NormalDir { index, .. } => index.clone(),
                    _ => None,
                })
                .collect_vec();
            // sort according to date
            // TODO: add more criteria for flexibility
            list.sort_by(|a, b| a.borrow().date().cmp(b.borrow().date()));
            for (i, n) in list.iter().enumerate() {
                if i as i64 - 1 >= 0 {
                    if let Some(p) = list.get(i - 1) {
                        n.clone().borrow_mut().set_last(Some(p.clone()));
                    }
                }
                if let Some(p) = list.get(i + 1) {
                    n.clone().borrow_mut().set_next(Some(p.clone()));
                }
            }
            // return node
            let node = Rc::new(RefCell::new(SiteTreeNode::NormalDir {
                children,
                path: path.clone(),
                index,
            }));
            (node, None)
        } else if path.is_file() {
            // check whether it is page file by extension name
            if self.check_page(path) {
                let content = fs::read_to_string(path).expect("cannot open page file");
                let result = serde_frontmatter::deserialize::<HashMap<String, serde_yaml::Value>>(
                    content.as_str(),
                );
                let (fm, _) = result.unwrap_or((HashMap::new(), "".to_string()));
                let url = self._get_page_url(path);
                let page = Rc::new(RefCell::new(Page::new(fm, url)));
                // add page to self.pages
                self.pages.push(page.clone());
                // return node and ref of index
                let node = Rc::new(RefCell::new(SiteTreeNode::PageFile {
                    path: path.clone(),
                    page: page.clone(),
                }));
                let index = if self.check_index(path) {
                    Some(page.clone())
                } else {
                    None
                };
                (node, index)
            } else {
                let node = Rc::new(RefCell::new(SiteTreeNode::StaticFile {
                    path: path.clone(),
                }));
                (node, None)
            }
        } else {
            error!("unknown type");
            panic!();
        }
    }

    fn check_page(&self, path: &PathBuf) -> bool {
        self.convert_ext.contains(
            &path
                .extension()
                .unwrap_or(OsStr::new(""))
                .to_string_lossy()
                .to_string(),
        )
    }

    fn check_index(&self, path: &PathBuf) -> bool {
        path.file_stem()
            .unwrap_or(OsStr::new(""))
            .to_string_lossy()
            .to_string()
            == "index"
    }

    pub fn convert_page(&self, path: &PathBuf, page: PageRef) {
        let dest_path = self._get_dest_path(path, true);
        let full_page = fs::read_to_string(path).expect("cannot read file");
        let (_, content) =
            serde_frontmatter::deserialize::<HashMap<String, Value>>(full_page.as_str())
                .unwrap_or((HashMap::new(), full_page));

        let mut converted = content;
        let mut converter_choice = String::new();
        if let Some(choice) = self
            .converter_choice
            .get(path.extension().unwrap_or(OsStr::new("")).to_str().unwrap())
        {
            converter_choice = choice.clone();
        }

        if let Some(converter) = self.converters.get(&converter_choice) {
            converted = converter.convert(converted);
        } else {
            warn!("no converter set, copy by default");
        }

        let page_config = page.borrow().get_page_config(true);
        let layout = page_config.get("layout");
        let mut rendered = converted;
        if let Some(Value::String(layout_str)) = layout {
            // debug!("try to use layout {}", layout_str);
            let mut current_layout = layout_str;
            while let Some(template) = self.templates.get(current_layout) {
                // debug!("current template {}", current_layout);
                let mut globals = liquid::object!({
                    "site": self.config,
                    "page": page_config,
                    "sitetree": self.site_tree_object,
                    "taxo": self.taxo_object,
                    "content": rendered,
                });
                let render_result = template.render(&mut globals);
                if render_result.is_err() {
                    error!("{}", render_result.err().unwrap());
                    panic!("render failed");
                }
                let current_rendered = render_result.unwrap();
                rendered = current_rendered;
                current_layout = template.get_parent();
            }
        } else {
            warn!("no layout set, copy by default");
        }

        match fs::write(&dest_path, rendered) {
            Ok(_) => (),
            Err(_) => error!("cannot write to {:?}", dest_path),
        }
    }

    fn _gen_site_tree_object(&self, node: NodeRef) -> (Option<SiteTreeObject>, SiteTreeObjectType) {
        match &*node.borrow() {
            NormalDir {
                children,
                path,
                index,
            } => {
                let mut list = serde_yaml::Sequence::new();
                let mut object = serde_yaml::Mapping::new();
                for child in children.iter() {
                    let (child_object, child_type) = self._gen_site_tree_object(child.clone());
                    match child_type {
                        SiteTreeObjectType::Dir(dirname) => {
                            object.insert(
                                serde_yaml::Value::from(dirname),
                                child_object.clone().unwrap(),
                            );
                        }
                        SiteTreeObjectType::DirWithIndexPage(dirname, page) => {
                            object.insert(
                                serde_yaml::Value::from(dirname),
                                child_object.clone().unwrap(),
                            );
                            list.push(page);
                        }
                        SiteTreeObjectType::Page => {
                            list.push(child_object.clone().unwrap());
                        }
                        _ => (),
                    }
                }
                object.insert(
                    serde_yaml::Value::from("_list"),
                    serde_yaml::Value::from(list),
                );
                let object_type = if path == Path::new(".") {
                    SiteTreeObjectType::Dir("_home".to_string())
                } else if let Some(page) = index {
                    SiteTreeObjectType::DirWithIndexPage(
                        path.file_stem().unwrap().to_string_lossy().to_string(),
                        serde_yaml::Value::Mapping(page.borrow().get_page_config_object(false)),
                    )
                } else {
                    SiteTreeObjectType::Dir(path.file_stem().unwrap().to_string_lossy().to_string())
                };

                (Some(serde_yaml::Value::Mapping(object)), object_type)
            }
            PageFile { page, .. } => {
                let map = page.borrow().get_page_config_object(false);
                (
                    Some(serde_yaml::Value::Mapping(map)),
                    SiteTreeObjectType::Page,
                )
            }
            _ => (None, SiteTreeObjectType::Unknown),
        }
    }

    fn _gen_taxo_object(&mut self) {
        for page in self.pages.iter() {
            for (taxo, v) in self.taxonomies.iter_mut() {
                for kind in page.borrow().belongs_to_kind(taxo).iter() {
                    if let None = v.get(kind) {
                        v.insert(kind.clone(), RefCell::new(vec![]));
                    }
                    v[kind].borrow_mut().push(serde_yaml::Value::Mapping(
                        page.borrow().get_page_config_object(false),
                    ));
                }
            }
        }

        let mut taxo_to_kind = serde_yaml::Mapping::new();
        for (taxo, v) in self.taxonomies.iter() {
            let mut kind_to_vec = serde_yaml::Mapping::new();
            for (kind, pages) in v.iter() {
                let mut seq = serde_yaml::Sequence::new();
                seq.extend(pages.borrow().iter().map(|x| {
                    x.clone()
                }));
                kind_to_vec.insert(
                    serde_yaml::Value::String(kind.clone()),
                    serde_yaml::Value::Sequence(seq)
                );
            }
            kind_to_vec.insert(
                serde_yaml::Value::String("_keys".to_string()),
                serde_yaml::Value::Sequence(serde_yaml::Sequence::from_iter(
                    v.keys().map(|x| {
                        debug!("{}", x.clone());
                        serde_yaml::Value::String(x.clone())
                    })
                ))
            );
            taxo_to_kind.insert(
                serde_yaml::Value::String(taxo.clone()),
                serde_yaml::Value::Mapping(kind_to_vec),
            );
        }
        taxo_to_kind.insert(
            serde_yaml::Value::String("_keys".to_string()),
            serde_yaml::Value::Sequence(serde_yaml::Sequence::from_iter(self.taxonomies.keys().map(|x| {
                serde_yaml::Value::String(x.clone())
            })))
        );

        self.taxo_object = Some(serde_yaml::Value::Mapping(taxo_to_kind));
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
    ) -> (
        HashSet<String>,
        HashMap<String, String>,
        HashMap<String, HashMap<String, RefCell<Vec<SiteTreeObject>>>>,
    ) {
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

        let mut taxonomies = HashMap::new();
        if let Some(Value::Sequence(taxo)) = config.get("taxonomies") {
            taxonomies.extend(taxo.iter().filter_map(|x| {
                if let Value::String(s) = x {
                    Some((s.clone(), HashMap::new()))
                } else {
                    None
                }
            }));
        }
        (convert_ext, converter_choice, taxonomies)
    }

    fn _parse_config_file(path: PathBuf) -> HashMap<String, Value> {
        let raw_config = fs::read(path).expect("cannot read config file");
        let mut config: HashMap<String, Value> =
            serde_yaml::from_slice(raw_config.as_slice()).expect("cannot parse config file");
        config.insert(
            String::from("time"),
            Value::String(chrono::Local::now().to_rfc3339()),
        );
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
                            info!(
                                "find partial: \"{}\"",
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
                        info!(
                            "find template: \"{}\"",
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
                info!(
                    "find converter: \"{}\"",
                    entry.file_name().to_string_lossy().to_string()
                );
            }
        }
        converters
    }
}
