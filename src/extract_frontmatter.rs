use log::trace;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

enum ExtractorState {
    Start,
    InYaml,
    OutYaml,
}

pub fn extract_front_matter(path: &PathBuf) -> (HashMap<String, serde_yaml::Value>, String) {
    let f = File::open(path).expect("cannot open file");
    let f = BufReader::new(f);
    let mut front_matter = String::new();
    let mut content = String::new();
    let mut state = ExtractorState::Start;
    let delim = regex::Regex::new(r"^-{3,}\s*$").unwrap();
    let space = regex::Regex::new(r"^\s*$").unwrap();
    for line in f.lines() {
        let line = line.unwrap();
        match state {
            ExtractorState::Start => {
                state = if space.is_match(&line) {
                    ExtractorState::Start
                } else if delim.is_match(&line) {
                    ExtractorState::InYaml
                } else {
                    content.push_str(&line);
                    content.push('\n');
                    ExtractorState::OutYaml
                }
            }
            ExtractorState::InYaml => {
                if delim.is_match(&line) {
                    state = ExtractorState::OutYaml;
                } else {
                    front_matter.push_str(&line);
                    front_matter.push('\n');
                }
            }
            ExtractorState::OutYaml => {
                content.push_str(&line);
                content.push('\n');
            }
        }
    }
    trace!("{:?}, {:?}", &front_matter, &content);
    let fm: HashMap<String, serde_yaml::Value> =
        serde_yaml::from_str(front_matter.as_str()).unwrap_or(HashMap::new());
    (fm, content)
}
