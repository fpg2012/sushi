mod converters;
mod site;

use crate::site::Site;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new().init().unwrap();
    let site = Site::parse_site_dir(".".into());
    site.generate_site();
}
