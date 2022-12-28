mod batch_iterator;
mod converters;
mod extract_frontmatter;
mod layout;
mod page;
mod paginator;
mod site;
mod existing_tree;

use crate::site::{Site, SiteConfigs};
use clap::Parser;
use log::{error, info};
use simple_logger::SimpleLogger;
use std::path::PathBuf;

#[derive(clap::Parser)]
#[clap(name = "sūshì", author = "nth233", version, about)]
struct Cli {
    #[clap(long)]
    debug: bool,
    #[clap(long, short = 'q')]
    quiet: bool,
    #[clap(subcommand)]
    commands: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    #[clap(arg_required_else_help = true)]
    Init {
        site_name: String,
        #[clap(long, default_value = "default")]
        theme: PathBuf,
        #[clap(long, default_value = ".")]
        path: PathBuf,
    },
    Build {
        #[clap(long, short = 'A')]
        regen_all: bool,
        #[clap(long, short = 'c', default_value = "_site.yml")]
        config: String,
        #[clap(long, short = 'g')]
        gen: Option<String>,
        #[clap(long)]
        includes: Option<String>,
        #[clap(long)]
        converters: Option<String>,
        #[clap(long)]
        templates: Option<String>,
    }
}

fn initialize_site(site_name: &String, theme: &PathBuf, path: &PathBuf) {
    // check for existence
    let mut path = path.clone();
    path.push(site_name);
    if path.exists() {
        error!("path {:?} exists", &path);
        panic!();
    }
    // look for theme
    let original_theme = theme;
    let mut theme = theme.clone();
    if !theme.exists() {
        if let Some(project_dir) = directories::ProjectDirs::from("io", "github", "sushi-gen") {
            let mut theme_dir = PathBuf::from(project_dir.config_dir());
            theme_dir.push(theme.clone());
            if theme_dir.exists() {
                theme = theme_dir.clone()
            }
        }
    }
    if !&theme.exists() {
        error!("theme {:?} or {:?} does not exists", original_theme, &theme);
        panic!()
    }
    info!("[discover] {:?}", &theme);
    if !&theme.is_dir() {
        error!("{:?} is not a valid directory", theme);
    }
    // sushi_init_copy(&theme, &path);
    let mut copy_options = fs_extra::dir::CopyOptions::new();
    copy_options.copy_inside = true;
    let result = fs_extra::dir::copy(&theme, &path, &copy_options);
    match result {
        Ok(_) => {
            info!("[copy] from {:?}", &theme);
            info!("{:?} created", path.clone());
        }
        Err(e) => {
            error!("cannot copy from {:?}, error: {}", &theme, e);
            panic!();
        }
    }
}

fn main() {
    let cli = Cli::parse();
    let mut level = log::LevelFilter::Info;
    if cli.quiet {
        level = log::LevelFilter::Error;
    } else if cli.debug {
        level = log::LevelFilter::Debug;
    }
    SimpleLogger::new().with_level(level).init().unwrap();

    match &cli.commands {
        Command::Init {
            site_name,
            theme,
            path,
        } => {
            initialize_site(site_name, theme, path);
        }
        Command::Build { regen_all, config, gen, includes, converters, templates } => {
            let site_configs = SiteConfigs {
                config: config.clone(),
                gen: gen.clone(),
                converters: converters.clone(),
                includes: includes.clone(),
                templates: templates.clone(),
            };
            let mut site = Site::parse_site_dir(".".into(), regen_all.clone(), site_configs);
            site.generate_site();
        }
    }
}
