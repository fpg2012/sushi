mod converters;
mod layout;
mod page;
mod site;
mod batch_iterator;
mod extract_frontmatter;
mod paginator;

use std::fs;
use std::fs::read_dir;
use std::path::PathBuf;
use clap::Parser;
use log::{error, info};
use crate::site::Site;
use simple_logger::SimpleLogger;

#[derive(clap::Parser)]
#[clap(name = "sūshì", author = "nth233", version, about)]
struct Cli {
    #[clap(long)]
    debug: bool,
    #[clap(long, short = 'q')]
    quiet: bool,
    #[clap(long, short = 'A')]
    regen_all: bool,
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
    Build,
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
        if let Some(project_dir) = directories::ProjectDirs::from("io", "github", "sushi") {
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
    sushi_init_copy(&theme, &path);
    info!("{:?} created", path.clone());
}

fn sushi_init_copy(from: &PathBuf, to: &PathBuf) {
    if from.is_dir() {
        info!("[gen] {:?}", &to);
        fs::create_dir(to.clone()).unwrap();
        for entry in read_dir(from).unwrap() {
            let mut to2 = to.clone();
            let from = entry.unwrap().path();
            to2.push(&from.file_stem().unwrap());
            sushi_init_copy(&from, &to2)
        }
    } else if from.is_file() {
        info!("[copy] {:?}", &to);
        fs::copy(from, to).unwrap();
    } else {
        panic!("Unknown file type");
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
        Command::Init { site_name, theme, path } => {
            initialize_site(site_name, theme, path);
        },
        Command::Build => {
            let mut site = Site::parse_site_dir(".".into());
            site.generate_site();
        },
    }
}
