mod batch_iterator;
mod configuration_loader;
mod converters;
mod existing_tree;
mod extract_frontmatter;
mod layout;
mod markdown_parser;
mod page;
mod paginator;
mod site;
mod theme;

use crate::site::{Site, SiteConfigs};
use clap::{CommandFactory, Parser};
use log::{error, info};
use shadow_rs::shadow;
use simple_logger::SimpleLogger;
use std::path::PathBuf;

shadow!(build);

#[derive(clap::Parser)]
#[clap(name = "sūshì", author = "nth233", about)]
struct Cli {
    #[clap(long)]
    debug: bool,
    #[clap(long, short = 'q')]
    quiet: bool,
    #[clap(long, short = 'v')]
    verbose: bool,
    #[clap(subcommand)]
    commands: Option<Command>,
    #[clap(long, short = 'V')]
    version: bool,
}

#[derive(clap::Subcommand)]
enum Command {
    #[clap(arg_required_else_help = false)]
    Init {
        site_name: String,
        #[clap(long, default_value = "default")]
        theme: PathBuf,
        #[clap(long, default_value = ".")]
        path: PathBuf,
    },
    Build {
        #[clap(long, short = 'A', help = "regenerate all files")]
        regen_all: bool,
        #[clap(long, short = 'c', default_value = "_site.yml")]
        config: String,
        #[clap(long, short = 'g', help = "generated files directory (_gen)")]
        gen: Option<String>,
        #[clap(long, help = "includes directory (_includes)")]
        includes: Option<String>,
        #[clap(long, help = "converters directory (_converters)")]
        converters: Option<String>,
        #[clap(long, help = "templates directory (_templates)")]
        templates: Option<String>,
        #[clap(long, help = "theme directory")]
        theme: Option<String>,
        #[clap(long, short = 's', help = "generate only a subpath")]
        subpath: Option<Vec<String>>,
        #[clap(long, help = "skip all unmodified files naively")]
        naive_skip: bool,
    },
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
            info!("[--copy] from {:?}", &theme);
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
    if cli.version {
        println!(
            "sūshì v{} ({})",
            env!("CARGO_PKG_VERSION"),
            build::SHORT_COMMIT
        );
        return;
    }
    let mut level = log::LevelFilter::Warn;
    if cli.quiet {
        level = log::LevelFilter::Error;
    } else if cli.debug {
        level = log::LevelFilter::Debug;
    } else if cli.verbose {
        level = log::LevelFilter::Info;
    }
    SimpleLogger::new().with_level(level).init().unwrap();

    match cli.commands {
        None => {
            Cli::command()
                .print_help()
                .unwrap_or_else(|e| eprintln!("{}", e));
        }
        Some(Command::Init {
            site_name,
            theme,
            path,
        }) => {
            initialize_site(&site_name, &theme, &path);
        }
        Some(Command::Build {
            regen_all,
            config,
            gen,
            includes,
            converters,
            templates,
            theme,
            subpath,
            naive_skip,
        }) => {
            let site_configs = SiteConfigs {
                config: config.clone(),
                gen: gen.clone(),
                converters: converters.clone(),
                includes: includes.clone(),
                templates: templates.clone(),
                theme: theme.clone(),
                subpath: subpath.clone(),
                naive_skip: naive_skip.clone(),
            };
            let mut site = Site::parse_site_dir(".".into(), regen_all.clone(), site_configs);
            site.generate_site();
        }
    }
}
