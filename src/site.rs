use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use serde_yaml::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::option::Option;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::string::String;
use std::time::SystemTime;
use std::vec::Vec;

use crate::converters::Converter;
use crate::extract_frontmatter::extract_front_matter;
use crate::layout::Layout;
use crate::page::{Page, PageRef};
use crate::paginator::Paginator;
use crate::site::SiteTreeNode::*;
use crate::existing_tree::{ETNodeRef, ExistingTreeNode};
use crate::existing_tree::ExistingTreeNode::File;
use crate::theme::Theme;
use crate::configuration_loader as confld;

type NodeRef = Rc<RefCell<SiteTreeNode>>;
type SiteTreeObject = serde_yaml::Value;

#[derive(Debug)]
pub enum SiteTreeNode {
    NormalDir {
        children: Vec<NodeRef>,
        path: PathBuf,
        index: Option<PageRef>,
        gen_path: PathBuf,
    },
    PageFile {
        path: PathBuf,
        page: PageRef,
    },
    StaticFile {
        path: PathBuf,
        gen_path: PathBuf,
        timestamp: SystemTime,
    },
}

enum SiteTreeObjectType {
    Unknown,
    Dir(String),
    // dir name
    DirWithIndexPage(String, serde_yaml::Value, PageRef),
    // dir name, page object
    Page(PageRef),
}

pub struct Site {
    site_dir: PathBuf,
    config: HashMap<String, serde_yaml::Value>,
    site_url: Option<String>,
    templates: HashMap<String, Layout>,
    converters: HashMap<String, Converter>,
    gen_dir: PathBuf,
    site_tree: Option<NodeRef>,
    existing_map: Rc<RefCell<HashMap<PathBuf, ETNodeRef>>>,

    convert_ext: HashSet<String>,
    converter_choice: HashMap<String, String>,
    convert_to_ext: HashMap<String, String>,
    taxonomies: HashMap<String, HashMap<String, RefCell<Vec<PageRef>>>>,
    pages: Vec<PageRef>,
    id_to_page: HashMap<String, PageRef>,

    site_tree_object: Option<serde_yaml::Value>,
    taxo_object: Option<serde_yaml::Value>,
    id_to_page_object: Option<serde_yaml::Value>,
    all_pages_object: Option<serde_yaml::Value>,

    regen_all: bool,
    naive_skip: bool,

    theme: Option<Theme>,
    subpath: Option<Vec<PathBuf>>,
}

pub struct SiteConfigs {
    pub config: String,
    pub gen: Option<String>,
    pub converters: Option<String>,
    pub includes: Option<String>,
    pub templates: Option<String>,
    pub theme: Option<String>,
    pub subpath: Option<Vec<String>>,
    pub naive_skip: bool,
}

impl Site {
    pub fn parse_site_dir(site_dir: PathBuf, regen_all: bool, site_configs: SiteConfigs) -> Self {
        // search for _site.yml
        let temp_config = fs::read_dir(site_dir.clone())
            .expect("cannot open site directory.")
            .find(|x| {
                if let Ok(file) = x {
                    file.file_name() == OsString::from(&site_configs.config) && file.path().is_file()
                } else {
                    false
                }
            })
            .expect(format!{"cannot find configuration file: {}", &site_configs.config}.as_str())
            .unwrap();
        let mut config = confld::parse_config_file(temp_config.path());

        let site_url = confld::string_from_config("url", &config);
        let site_gen_dir = confld::string_from_config("gen_dir", &config);
        let site_converters_dir = confld::string_from_config("converters_dir", &config);
        let site_templates_dir = confld::string_from_config("templates_dir", &config);
        let site_includes_dir = confld::string_from_config("includes_dir", &config);
        let site_theme_dir = confld::string_from_config("theme_dir", &config);

        let _gen_dir = Self::_decide_site_config(site_configs.gen.clone(), site_gen_dir, "_gen".to_string());
        let _converters_dir = Self::_decide_site_config(site_configs.converters.clone(), site_converters_dir, "_converters".to_string());
        let _templates_dir = Self::_decide_site_config(site_configs.templates.clone(), site_templates_dir, "_templates".to_string());
        let _includes_dir = Self::_decide_site_config(site_configs.includes.clone(), site_includes_dir, "_includes".to_string());
        let _theme_dir = if site_configs.theme != None {
            site_configs.theme.clone() // command line param
        } else {
            site_theme_dir // config file
        };

        // search for _includes
        let temp_includes = confld::find_dir(&site_dir, &_includes_dir);
        let mut partial_list = if let Some(Ok(temp)) = temp_includes {
            confld::parse_includes(temp.path())
        } else {
            warn!("no include found");
            HashMap::new()
        };

        // search for _templates
        let temp_templates = confld::find_dir(&site_dir, &_templates_dir);
        let mut template_list = if let Some(Ok(temp)) = temp_templates {
            confld::parse_templates(temp.path())
        } else {
            warn!("no template found");
            HashMap::new()
        };

        // search for _converters
        let temp_converters = confld::find_dir(&site_dir, &_converters_dir);
        let mut converter_list = if let Some(Ok(temp)) = temp_converters {
            confld::parse_converters(temp.path())
        } else {
            warn!("no converters found");
            HashMap::new()
        };

        let mut theme = None;

        // load theme
        if let Some(theme_dir) = _theme_dir {
            let temp_theme = confld::find_dir_or_panic(&site_dir, &theme_dir);
            let real_theme =Theme::new(temp_theme.path());

            // combine list and config
            for (name, path) in real_theme.partial_list.iter() {
                if partial_list.get(name) == None {
                    partial_list.insert(name.clone(), path.clone());
                }
            }

            for (name, path) in real_theme.template_list.iter() {
                if template_list.get(name) == None {
                    template_list.insert(name.clone(), path.clone());
                }
            }

            for (name, path) in real_theme.converter_list.iter() {
                if converter_list.get(name) == None {
                    converter_list.insert(name.clone(), path.clone());
                }
            }

            for (name, value) in real_theme.config.iter() {
                if config.get(name) == None {
                    config.insert(name.clone(), value.clone());
                }
            }

            theme = Some(real_theme);
        }

        // compile & load
        let partial_compiler = confld::compile_partials(partial_list);
        let templates = confld::compile_templates(partial_compiler, template_list);
        let converters = confld::load_converters(converter_list);

        // parse dir
        let mut gen_dir = site_dir.clone();
        gen_dir.push(_gen_dir);
        let existing_map = Rc::new(RefCell::new(HashMap::new()));
        Self::_parse_gen(&gen_dir, existing_map.clone());
        // debug!("{:?}", &existing_map);

        let (convert_ext, converter_choice, convert_to_ext, taxonomies) = Self::_extract_important_config(&config);
        debug!("{:?}", convert_ext);
        debug!("{:?}", converter_choice);
        debug!("{:?}", taxonomies);

        let naive_skip = site_configs.naive_skip;

        let mut subpath = None;

        if let Some(sp) = site_configs.subpath {
            let mut real_subpath = vec![];
            for p in sp {
                real_subpath.push(PathBuf::from(p));
            }
            subpath = Some(real_subpath);
        }

        Site {
            site_dir,
            config,
            site_url,
            templates,
            converters,
            gen_dir,
            site_tree: None,
            existing_map,
            convert_ext,
            converter_choice,
            convert_to_ext,
            taxonomies,
            pages: vec![],
            id_to_page: HashMap::new(),
            site_tree_object: None,
            taxo_object: None,
            all_pages_object: None,
            id_to_page_object: None,
            regen_all,
            naive_skip,
            theme,
            subpath,
        }
    }

    pub fn generate_site(&mut self) {
        // gen site tree
        let gen_dir = self.gen_dir.clone();
        let (site_tree, _) = self._gen_site_tree(&self.site_dir.clone(), &gen_dir);

        if let Some(theme) = &self.theme {
            if let SiteTreeNode::NormalDir { children, path: _, gen_path: _, index } = &mut *site_tree.borrow_mut() {
                self._merge_theme_site_tree(theme.theme_dir.clone(), &gen_dir, children, index.clone());
            }
        }

        let indent = String::from("");
        Self::_print_site_tree(site_tree.clone(), &indent);

        self.site_tree = Some(site_tree);

        // gen sitetree object based on self.site_tree
        let (site_tree_object, _) = self._gen_site_tree_object(self.site_tree.clone().unwrap());
        self.site_tree_object = site_tree_object;

        // gen taxo object based on self.taxonomies
        self._gen_taxo_object();
        // let temp = serde_yaml::to_string(&self.taxo_object).unwrap_or("error".to_string());
        // debug!("{}", temp);

        // gen id_to_page object
        let id_to_page_object = self._gen_id_to_page_object();
        self.id_to_page_object = Some(id_to_page_object);

        // gen all_pages object
        // sort all_pages
        self.pages
            .sort_by(|a, b| match a.borrow().date().cmp(b.borrow().date()) {
                std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
                std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
                std::cmp::Ordering::Equal => std::cmp::Ordering::Equal,
            });
        let all_pages_object = self._gen_all_pages_object();
        self.all_pages_object = Some(all_pages_object);

        // assemble global object
        let globals = liquid::object!({
            "site": self.config,
            "sitetree": self.site_tree_object,
            "taxo": self.taxo_object,
            "all_pages": self.all_pages_object,
            "id_to_page": self.id_to_page_object,
        });

        // gen _gen
        self._generate(self.site_tree.clone().unwrap(), globals);
    }

    fn _generate(&self, current_node: NodeRef, mut globals: liquid::Object) -> liquid::Object {
        match &mut *current_node.clone().borrow_mut() {
            SiteTreeNode::NormalDir { children, path: _, gen_path, .. } => {
                // let dest_path = self._get_dest_path(path, false, None);
                let dest_path = gen_path.clone();
                // match path.file_name() {
                //     Some(file_name) => {
                //         dest_path.push(file_name);
                //     },
                //     None => ()
                // }
                debug!("[mkdir]  {:?}", dest_path);
                fs::create_dir_all(&dest_path).expect("cannot create dir");
                let mut globals = globals;
                for child in children.iter() {
                    globals = self._generate(child.clone(), globals)
                }
                globals
            }
            SiteTreeNode::PageFile { path: _, page} => {
                self.gen_page(page.clone(), &mut globals);
                globals
            }
            SiteTreeNode::StaticFile { path, gen_path, timestamp } => {
                // let dest_path = self._get_dest_path(path, false, None);
                let dest_path = gen_path.clone();
                // check whether skip copy
                let src_timestamp = timestamp;
                let mut do_copy = self._decide_not_skip_static(&dest_path, src_timestamp);
                if self.subpath != None {
                    if self.naive_skip {
                        do_copy = self._in_subpath(&path);
                    } else {
                        do_copy &= self._in_subpath(&path);
                    }
                }
                if do_copy {
                    info!("[copy]  {} -> {}", path.clone().to_string_lossy(), &dest_path.to_string_lossy());
                    fs::copy(path.clone(), dest_path).unwrap();
                } else {
                    debug!("[skip]  {}", path.clone().to_string_lossy());
                }
                globals
            }
        }
    }

    fn _gen_site_tree(&mut self, path: &PathBuf, gen_path: &PathBuf) -> (NodeRef, Option<PageRef>) {
        if path.is_dir() {
            let mut children: Vec<NodeRef> = vec![];
            let mut index: Option<PageRef> = None;
            let mut new_gen_path = gen_path.clone();
            match path.file_name() {
                None => (),
                Some(file_name) => {
                    new_gen_path.push(file_name);
                }
            }
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
                    let (child, index_) = self._gen_site_tree(&entry.path(), &new_gen_path);
                    match &*child.borrow() {
                        PageFile { .. } => index = index_,
                        _ => (),
                    }
                    children.push(child);
                }
            }
            Self::_sort_children(&mut children);
            // return node
            let node = Rc::new(RefCell::new(SiteTreeNode::NormalDir {
                children,
                path: path.clone(),
                gen_path: new_gen_path,
                index,
            }));
            (node, None)
        } else if path.is_file() {
            self._load_file(&path, &gen_path)
        } else {
            error!("unknown type");
            panic!();
        }
    }

    fn _load_file(&mut self, path: &PathBuf, gen_path: &PathBuf) -> (NodeRef, Option<PageRef>) {
        // check whether it is page file by extension name
        let timestamp = if let Ok(metadata) = path.metadata() {
            metadata.modified().unwrap_or(SystemTime::now())
        } else {
            SystemTime::now()
        };
        if self.is_page(path) {
            let ext = path.extension().unwrap_or(OsStr::new("")).to_string_lossy().to_string();

            let (fm, content) = extract_front_matter(path);

            let mut fm_file_name = OsString::from("_");
            fm_file_name.push(path.file_name().unwrap());
            fm_file_name.push(".yml");
            let fm_file_path = path.with_file_name(fm_file_name);
            let mut fm_from_file: HashMap<String, serde_yaml::Value> = HashMap::new();
            if fm_file_path.exists() {
                let fm_content = fs::read_to_string(fm_file_path).expect("Unable to read front matter file");
                fm_from_file = serde_yaml::from_str(&fm_content).expect("Unable to parse front matter file");
            }

            let mut fm = match fm {
                Some(fm) => fm,
                None => HashMap::new(),
            };

            for (key, value) in fm_from_file {
                fm.entry(key).or_insert(value);
            }

            let fm = if fm.is_empty() {
                None
            } else {
                Some(fm)
            };

            let fm = match fm {
                Some(fm) => fm,
                None => {
                    let mut temp = HashMap::new();
                    temp.insert(
                        "to_ext".to_string(),
                        serde_yaml::Value::from("html")
                    ); // html by default
                    temp.insert(
                        "nolist".to_string(),
                        serde_yaml::Value::from(true)
                    ); // nolist by default
                    temp
                }
            };

            // get expected extension name
            let to_ext = match fm.get("to_ext") {
                Some(Value::String(t_e)) => t_e.clone(),
                _ => {
                    match self.convert_to_ext.get(&ext.clone()) {
                        Some(t_e) => t_e.clone(),
                        _ => "html".to_string() // html by default
                    }
                }
            };

            let mut new_gen_path = gen_path.clone();
            new_gen_path.push(path.file_stem().unwrap());
            new_gen_path.set_extension(&to_ext);

            let url = self.get_page_url(path, to_ext.clone());
            let page = Rc::new(RefCell::new(Page::new(
                fm, 
                url, 
                path.clone(), 
                Some(to_ext), 
                content, 
                timestamp.clone(),
                new_gen_path
            )));
            // check whether page_id is unique
            let page_id = page.borrow().get_page_id().clone();
            if self.id_to_page.contains_key(&page_id) {
                error!("id \"{}\" is not unique!", page_id);
                panic!();
            }
            // add page to self.pages and self.id_to_page
            self.pages.push(page.clone());
            self.id_to_page.insert(page_id, page.clone());
            // return node and ref of index
            let node = Rc::new(RefCell::new(SiteTreeNode::PageFile {
                path: path.clone(),
                page: page.clone(),
            }));
            let index = if self.is_index(path) {
                Some(page.clone())
            } else {
                None
            };
            (node, index)
        } else {
            let mut new_gen_path = gen_path.clone();
            new_gen_path.push(path.file_name().unwrap());
            let node = Rc::new(RefCell::new(SiteTreeNode::StaticFile {
                path: path.clone(),
                gen_path: new_gen_path,
                timestamp,
            }));
            (node, None)
        }
    }

    fn _merge_theme_site_tree(&mut self, path: PathBuf, gen_path: &PathBuf, children: &mut Vec<NodeRef>, mut index: Option<PageRef>) {
        for entry in path.read_dir().unwrap() {
            if let Ok(entry) = entry {
                if entry
                    .path()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(|ch: char| ch == '.' || ch == '_') {
                    continue;
                }
                if entry.path().is_dir() {
                    let correspond_node = children.iter().find(|x| {
                        if let SiteTreeNode::NormalDir { children: _, path, ..} = &*x.borrow() {
                            path.file_name() == entry.path().file_name()
                        } else {
                            false
                        }
                    });
                    match correspond_node {
                        Some(node) => {
                            if let SiteTreeNode::NormalDir { children, path: _, gen_path, index } = &mut *node.borrow_mut() {
                                self._merge_theme_site_tree(entry.path(), &gen_path, children, index.clone());
                            };
                        },
                        None => {
                            let (new_node, _) = self._gen_site_tree(&entry.path(), &gen_path);
                            children.push(new_node.clone());
                        }
                    };
                } else if entry.path().is_file() {
                    let correspond_node = children.iter().find(|x| {
                        match &*x.borrow() {
                            SiteTreeNode::PageFile { path, .. } => path.file_name() == entry.path().file_name(),
                            SiteTreeNode::StaticFile { path, .. } => path.file_name() == entry.path().file_name(),
                            _ => false
                        }
                    });
                    match correspond_node  {
                        None => {
                            let (new_node, new_index) = self._load_file(&entry.path(), &gen_path);
                            children.push(new_node.clone());
                            match index {
                                None => index = new_index,
                                _ => ()
                            }
                        },
                        _ => ()
                    }
                } else {
                    error!("unknown type");
                    panic!();
                }
            }
        }
        Self::_sort_children(children);
    }

    fn _sort_children(children: &mut Vec<NodeRef>) {
        // set page.next and page.last
        let mut list = children
            .iter()
            .filter_map(|x| match &*x.borrow() {
                PageFile { page, .. } => Some(page.clone()),
                NormalDir { index, .. } => index.clone(),
                    _ => None,
            }).collect_vec();
        // sort according to date
        // TODO: add more criteria for flexibility
        list.sort_by(|a, b| match a.borrow().date().cmp(b.borrow().date()) {
            std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
            std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
            std::cmp::Ordering::Equal => std::cmp::Ordering::Equal,
        });
        for (i, n) in list.iter().enumerate() {
            if i as i64 - 1 >= 0 {
                if let Some(p) = list.get(i - 1) {
                    n.borrow_mut().set_last(Some(p.clone()));
                }
            }
            if let Some(p) = list.get(i + 1) {
                n.borrow_mut().set_next(Some(p.clone()));
            }
        }
    }

    fn is_page(&self, path: &PathBuf) -> bool {
        self.convert_ext.contains(
            &path
                .extension()
                .unwrap_or(OsStr::new(""))
                .to_string_lossy()
                .to_string(),
        )
    }

    fn is_index(&self, path: &PathBuf) -> bool {
        let filename = path.file_stem().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        let ext = path.extension().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        filename == "index" && self.convert_ext.get(ext.as_str()) != None
    }

    pub fn gen_page(&self, page: PageRef, base_globals: &mut liquid::Object) {
        let dest_path = page.borrow().gen_path.clone();
        // let mut dest_path = path.clone();
        // dest_path.set_extension(page.borrow().to_ext.clone().unwrap());
        // let dest_path = self._get_dest_path(path, true, page.borrow().to_ext.clone());
        
        let paginator = page.borrow().paginate_info();
        let mut do_gen = self._decide_not_skip_page(page.clone());
        if let Some(_) = paginator {
            if !self.naive_skip {
                do_gen = true;
            }
        }
        if !do_gen {
            debug!("[skip]  {}", page.borrow().path.clone().to_string_lossy());
            return;
        }

        //let (_, content) = extract_front_matter(path);
        let content = page.borrow().content.clone().as_bytes().to_vec();

        let mut converted = content;
        let mut converter_choice = String::new();
        if let Some(choice) = self
            .converter_choice
            .get(page.borrow().path.extension().unwrap_or(OsStr::new("")).to_str().unwrap())
        {
            converter_choice = choice.clone();
        }

        if let Some(converter) = self.converters.get(&converter_choice) {
            converted = converter.convert(converted);
        } else {
            debug!("no converter set, copy by default");
        }

        let page_config = page.borrow().get_page_config();

        match paginator {
            None => {
                let layout = page_config.get("layout");
                let mut rendered = converted;
                if let Some(Value::String(layout_str)) = layout {
                    let mut rendered_str = String::from_utf8(rendered).expect("Invalid UTF-8 sequence");
                    debug!("try to use layout {}", layout_str);
                    let mut current_layout = layout_str;
                    while let Some(template) = self.templates.get(current_layout) {
                        debug!("current template {}", current_layout);
                        base_globals.insert(
                            "page".parse().unwrap(),
                            liquid::model::to_value(&page_config).unwrap(),
                        );
                        base_globals.insert(
                            "content".parse().unwrap(),
                            liquid::model::to_value(&rendered_str).unwrap(),
                        );
                        let render_result = template.render(base_globals);
                        if render_result.is_err() {
                            error!("{}", render_result.err().unwrap());
                            panic!("render failed");
                        }
                        let current_rendered = render_result.unwrap();
                        rendered_str = current_rendered;
                        current_layout = template.get_parent();
                    }
                    rendered = rendered_str.as_bytes().to_vec();
                } else {
                    debug!("no layout set, copy by default");
                }
                info!("[conv]  {}", page.borrow().path.clone().to_string_lossy());
                debug!("[conv] to {:?}", &dest_path);
                match fs::write(&dest_path, rendered) {
                    Ok(_) => (),
                    Err(_) => error!("cannot write to {:?}", dest_path),
                }
            }
            Some((exp, batch_size)) => {
                info!("[conv]  {}", page.borrow().path.clone().to_string_lossy());
                match Paginator::from_expression_and_object(
                    base_globals,
                    &exp,
                    batch_size,
                    dest_path.clone(),
                ) {
                    Ok(p) => {
                        fs::remove_dir(p.base_url_dir())
                            .unwrap_or(trace!("cannot remove {:?}", p.base_url_dir()));
                        fs::create_dir(p.base_url_dir())
                            .unwrap_or(trace!("cannot create {:?}", p.base_url_dir()));
                        let mut rendered = String::from_utf8(converted).expect("Invalid UTF-8 sequence");
                        let mut paginator_object = p.gen_paginator_object();
                        let batch_urls = p
                            .batch_paths()
                            .iter()
                            .map(|x| self._get_batch_url_from_dest(x))
                            .collect_vec();
                        paginator_object.insert(
                            "batch_urls".parse().unwrap(),
                            liquid::model::to_value(&batch_urls).unwrap(),
                        );
                        for (i, (dest_path, batch)) in p.batch_iter().enumerate() {
                            let layout = page_config.get("layout");
                            paginator_object.insert(
                                "current_batch".parse().unwrap(),
                                liquid::model::to_value(&batch).unwrap(),
                            );
                            paginator_object.insert(
                                "current_batch_num".parse().unwrap(),
                                liquid::model::to_value(&i).unwrap(),
                            );
                            if i > 0 {
                                paginator_object.insert(
                                    "last_batch_num".parse().unwrap(),
                                    liquid::model::to_value(&(i - 1)).unwrap(),
                                );
                            } else {
                                paginator_object.remove("last_batch_num");
                            }
                            if i < batch_urls.len() - 1 {
                                paginator_object.insert(
                                    "next_batch_num".parse().unwrap(),
                                    liquid::model::to_value(&(i + 1)).unwrap(),
                                );
                            } else {
                                paginator_object.remove("next_batch_num");
                            }
                            if let Some(Value::String(layout_str)) = layout {
                                debug!("try to use layout {}", layout_str);
                                let mut current_layout = layout_str;
                                while let Some(template) = self.templates.get(current_layout) {
                                    debug!("current template {}", current_layout);
                                    base_globals.insert(
                                        "page".parse().unwrap(),
                                        liquid::model::to_value(&page_config).unwrap(),
                                    );
                                    base_globals.insert(
                                        "content".parse().unwrap(),
                                        liquid::model::to_value(&rendered).unwrap(),
                                    );
                                    base_globals.insert(
                                        "paginator".parse().unwrap(),
                                        liquid::model::to_value(&paginator_object).unwrap(),
                                    );
                                    let render_result = template.render(base_globals);
                                    if render_result.is_err() {
                                        error!("{}", render_result.err().unwrap());
                                        panic!("render failed");
                                    }
                                    let current_rendered = render_result.unwrap();
                                    rendered = current_rendered;
                                    current_layout = template.get_parent();
                                }
                            } else {
                                trace!("no layout set, copy by default");
                            }
                            match fs::write(&dest_path, &rendered) {
                                Ok(_) => (),
                                Err(_) => error!("cannot write to {:?}", dest_path),
                            }
                        }
                    }
                    Err(_) => {
                        error!("cannot parse {:?} to a list", &exp);
                    }
                }
            }
        }
    }

    fn _gen_site_tree_object(&self, node: NodeRef) -> (Option<SiteTreeObject>, SiteTreeObjectType) {
        match &*node.borrow() {
            NormalDir {
                children,
                path,
                gen_path: _,
                index,
            } => {
                let mut list = vec![];
                // serde_yaml::Sequence::new()
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
                        SiteTreeObjectType::DirWithIndexPage(dirname, page, node) => {
                            object.insert(
                                serde_yaml::Value::from(dirname),
                                child_object.clone().unwrap(),
                            );
                            list.push((page, node.clone()));
                        }
                        SiteTreeObjectType::Page(node) => {
                            list.push((child_object.clone().unwrap(), node.clone()));
                        }
                        _ => (),
                    }
                }
                list.sort_by(|a, b| {
                    let ((_, a_), (_, b_)) = (a, b);
                    match a_.clone().borrow().date().cmp(b_.clone().borrow().date()) {
                        std::cmp::Ordering::Greater => std::cmp::Ordering::Less,
                        std::cmp::Ordering::Less => std::cmp::Ordering::Greater,
                        std::cmp::Ordering::Equal => std::cmp::Ordering::Equal,
                    }
                });
                let list = list.iter().map(|(a, _)| a.clone()).collect_vec();
                object.insert(
                    serde_yaml::Value::from("_list"),
                    serde_yaml::Value::from(list),
                );
                let object_type = if path == Path::new(".") {
                    SiteTreeObjectType::Dir("_home".to_string())
                } else if let Some(page) = index {
                    debug!("with index: {}", path.file_stem().unwrap().to_string_lossy().to_string());
                    SiteTreeObjectType::DirWithIndexPage(
                        path.file_stem().unwrap().to_string_lossy().to_string(),
                        serde_yaml::Value::String(page.borrow().get_page_id().clone()),
                        page.clone(),
                    )
                } else {
                    SiteTreeObjectType::Dir(path.file_stem().unwrap().to_string_lossy().to_string())
                };

                (Some(serde_yaml::Value::Mapping(object)), object_type)
            }
            PageFile { page, .. } => (
                Some(serde_yaml::Value::String(
                    page.borrow().get_page_id().clone(),
                )),
                SiteTreeObjectType::Page(page.clone()),
            ),
            _ => (None, SiteTreeObjectType::Unknown),
        }
    }

    fn _gen_taxo_object(&mut self) {
        // gen self.taxonomies
        for page in self.pages.iter() {
            for (taxo, v) in self.taxonomies.iter_mut() {
                for kind in page.borrow().belongs_to_kind(taxo).iter() {
                    if let None = v.get(kind) {
                        v.insert(kind.clone(), RefCell::new(vec![]));
                    }
                    v[kind].borrow_mut().push(page.clone());
                }
            }
        }

        // gen self.taxo_object based on self.taxonomies
        let mut taxo_to_kind = serde_yaml::Mapping::new();
        for (taxo, v) in self.taxonomies.iter() {
            let mut kind_to_vec = serde_yaml::Mapping::new();
            for (kind, pages) in v.iter() {
                let mut seq = serde_yaml::Sequence::new();
                seq.extend(
                    pages
                        .borrow()
                        .iter()
                        .map(|x| serde_yaml::Value::String(x.borrow().get_page_id().clone())),
                );
                kind_to_vec.insert(
                    serde_yaml::Value::String(kind.clone()),
                    serde_yaml::Value::Sequence(seq),
                );
            }
            kind_to_vec.insert(
                serde_yaml::Value::String("_keys".to_string()),
                serde_yaml::Value::Sequence(serde_yaml::Sequence::from_iter(v.keys().map(|x| {
                    // debug!("{}", x.clone());
                    serde_yaml::Value::String(x.clone())
                }))),
            );
            taxo_to_kind.insert(
                serde_yaml::Value::String(taxo.clone()),
                serde_yaml::Value::Mapping(kind_to_vec),
            );
        }
        taxo_to_kind.insert(
            serde_yaml::Value::String("_keys".to_string()),
            serde_yaml::Value::Sequence(serde_yaml::Sequence::from_iter(
                self.taxonomies
                    .keys()
                    .map(|x| serde_yaml::Value::String(x.clone())),
            )),
        );

        self.taxo_object = Some(serde_yaml::Value::Mapping(taxo_to_kind));
    }

    fn _gen_id_to_page_object(&self) -> serde_yaml::Value {
        let mut obj = serde_yaml::Mapping::new();
        for (k, v) in self.id_to_page.iter() {
            obj.insert(
                serde_yaml::Value::String(k.clone()),
                serde_yaml::Value::Mapping(v.borrow().get_page_config_object()),
            );
        }
        serde_yaml::Value::Mapping(obj)
    }

    fn _gen_all_pages_object(&self) -> serde_yaml::Value {
        let mut obj = serde_yaml::Sequence::new();
        for p in self.pages.iter() {
            obj.push(serde_yaml::Value::String(p.borrow().get_page_id().clone()))
        }
        serde_yaml::Value::Sequence(obj)
    }

    pub fn get_page_url(&self, path: &PathBuf, to_ext: String) -> String {
        let mut temp = PathBuf::from(path.strip_prefix(&self.site_dir).unwrap());
        let stem = temp.clone();
        let stem = stem.file_stem().unwrap();
        temp.pop();
        temp.push(stem);
        if let Some(s) = &self.site_url {
            s.to_string() + "/" + temp.to_str().unwrap() + "." + to_ext.as_str()
        } else {
            String::from("/") + temp.to_str().unwrap() + "." + to_ext.as_str()
        }
    }

    fn _get_batch_url_from_dest(&self, path: &PathBuf) -> String {
        let mut temp = PathBuf::from(path.strip_prefix(&self.gen_dir).unwrap());
        let stem = temp.clone();
        let stem = stem.file_stem().unwrap();
        temp.pop();
        temp.push(stem);
        if let Some(s) = &self.site_url {
            s.to_string() + "/" + temp.to_str().unwrap() + ".html"
        } else {
            String::from("/") + temp.to_str().unwrap() + ".html"
        }
    }

    fn _get_converter_dir(&self) -> PathBuf {
        let mut temp = PathBuf::from(&self.site_dir);
        temp.push("_converters");
        temp
    }

    // fn _get_dest_path(&self, path: &PathBuf, is_page: bool, to_ext: Option<String>) -> PathBuf {
    //     let mut dest = PathBuf::from(&self.gen_dir);
    //     dest.push(path.strip_prefix(&self.site_dir).unwrap());
    //     if is_page {
    //         if let Some(ext) = to_ext {
    //             dest.set_extension(ext);
    //         }
    //     }
    //     dest
    // }

    fn _extract_important_config(
        config: &HashMap<String, Value>,
    ) -> (
        HashSet<String>,
        HashMap<String, String>,
        HashMap<String, String>,
        HashMap<String, HashMap<String, RefCell<Vec<PageRef>>>>,
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

        let mut convert_to_ext = HashMap::new();
        if let Some(Value::Mapping(to_ext)) = config.get("convert_to_ext") {
            for (f, t) in to_ext.iter() {
                if let (Value::String(ext), Value::String(t_ext)) = (f, t) {
                    convert_to_ext.insert(ext.clone(), t_ext.clone());
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
        (convert_ext, converter_choice, convert_to_ext, taxonomies)
    }

    fn _parse_gen(path: &PathBuf, existing_map: Rc<RefCell<HashMap<PathBuf, ETNodeRef>>>) -> Option<ETNodeRef> {
        if path.is_dir() {
            let mut children: Vec<ETNodeRef> = vec![];
            for entry in path.read_dir().unwrap() {
                if let Ok(entry) = entry {
                    let child = Self::_parse_gen(&entry.path(), existing_map.clone());
                    if let Some(child) = child {
                        children.push(child);
                    }
                }
            }
            debug!("scan _gen: dir {:?}", path);
            let ret = Rc::new(RefCell::new(ExistingTreeNode::NormalDir {
                children,
                path: path.clone(),
            }));
            existing_map.borrow_mut().insert(path.clone(), ret.clone());
            Some(ret)
        } else if path.is_file() {
            if let Ok(metadata) = path.metadata() {
                let time = metadata.modified().unwrap_or(SystemTime::now());
                debug!("scan _gen: file {:?} at {:?}", path, time);
                let ret = Rc::new(RefCell::new(ExistingTreeNode::File {
                    path: path.clone(),
                    timestamp: time,
                }));
                existing_map.borrow_mut().insert(path.clone(), ret.clone());
                Some(ret)
            } else {
                None
            }
        } else {
            info!("cannot scan {}", &path.to_string_lossy());
            None
        }
    }

    fn lookup_existing_map(&self, path: &PathBuf) -> Option<ETNodeRef> {
        if let Some(et) = self.existing_map.borrow().get(path) {
            Some(et.clone())
        } else {
            None
        }
    }

    fn _decide_not_skip_static(&self, dest_path: &PathBuf, src_timestamp: &SystemTime) -> bool {
        if self.regen_all {
            return true;
        }
        self.is_src_newer(dest_path, src_timestamp)
    }

    fn _decide_not_skip_page(&self, page: PageRef) -> bool {
        if self.regen_all {
            return true;
        }

        if self.subpath != None {
            return self._in_subpath(&page.borrow().path);
        }

        let self_is_newer = self.is_src_newer(&page.borrow().gen_path, page.borrow().gen_time());

        if self.naive_skip {
            return self_is_newer;
        }

        let mut next_is_newer = false;
        let mut last_is_newer = false;
        if let Some(next_page) = page.borrow().next() {
            next_is_newer = self.is_src_newer(&next_page.borrow().gen_path, next_page.borrow().gen_time());
        }
        if let Some(last_page) = page.borrow().last() {
            last_is_newer = self.is_src_newer(&last_page.borrow().gen_path, last_page.borrow().gen_time());
        }
        self_is_newer || next_is_newer || last_is_newer
    }

    fn is_src_newer(&self, dest_path: &PathBuf, src_timestamp: &SystemTime) -> bool {
        if let Some(et) = self.lookup_existing_map(dest_path) {
            // check time
            match &*et.clone().borrow() {
                File { path: _, timestamp } => {
                    if timestamp < src_timestamp {
                        true
                    } else {
                        // dest is newer than src
                        false
                    }
                }
                _ => true,
            }
        } else {
            true
        }
    }

    fn _decide_site_config(cli_config: Option<String>, yml_config: Option<String>, default_config: String) -> String {
        // command line configuration is prior to _site.yml configuration
        if let Some(cli_str) = cli_config {
            cli_str
        } else {
            if let Some(yml_str) = yml_config {
                yml_str
            } else {
                default_config
            }
        }
    }

    fn _in_subpath(&self, path: &PathBuf) -> bool {
        match &self.subpath {
            None => true,
            Some(real_subpath) => {
                for p in real_subpath {
                    if p.is_dir() {
                        if path.canonicalize().unwrap().starts_with(p.canonicalize().expect(format!("improper subpath {}", p.to_str().unwrap()).as_str())) {
                            return true;
                        }
                    } else if p.is_file() {
                        if p.canonicalize().expect(format!("improper subpath {}", p.to_str().unwrap()).as_str()) == path.canonicalize().unwrap() {
                            return true;
                        }
                    }
                }
                false
            }
        }
    }

    fn _print_site_tree(current_node: NodeRef, indent: &String) {
        match &*current_node.borrow() {
            SiteTreeNode::NormalDir { children, path, .. } => {
                let new_indent = indent.clone() + " |";
                debug!("{}-+ {}", &indent, path.file_name().unwrap_or(OsString::from("").as_os_str()).to_string_lossy());
                for child in children.iter() {
                    Self::_print_site_tree(child.clone(), &new_indent);
                }
            },
            SiteTreeNode::PageFile { path, .. } => {
                debug!("{}-- {} (page)", &indent, path.file_name().unwrap().to_string_lossy());
            },
            SiteTreeNode::StaticFile { path, .. } => {
                debug!("{}-- {} (static)", &indent, path.file_name().unwrap().to_string_lossy());
            },
        }
    }
}
