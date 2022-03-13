extern crate core;

mod converters;
mod layout;
mod page;
mod site;

use crate::site::Site;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    let mut site = Site::parse_site_dir(".".into());
    site.generate_site();
}
