mod converters;
mod layout;
mod page;
mod site;

use clap::Parser;
use crate::site::Site;
use simple_logger::SimpleLogger;

#[derive(clap::Parser)]
#[clap(name = "sūshì", author = "nth233", version, about)]
struct Cli {
    #[clap(long)]
    debug: bool,
    #[clap[long]]
    quiet: bool,
    #[clap(long, short='A')]
    regen_all: bool,
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
    let mut site = Site::parse_site_dir(".".into());
    site.generate_site();
}
