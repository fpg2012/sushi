use std::path::PathBuf;
use subprocess::Exec;

pub struct Converter {
    pub name: String,
    pub path: PathBuf,
}

impl Converter {
    pub fn convert(&self, content: String) -> String {
        let cur_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&self.path.parent().unwrap()).unwrap();
        let mut temp_dir = PathBuf::from(".");
        temp_dir.push(&self.path.file_name().unwrap());
        // debug!("invoking {:?}", temp_dir);
        let read_content = Exec::cmd(temp_dir)
            .stdin(content.as_bytes().to_vec())
            .capture()
            .expect("converter error")
            .stdout_str();
        std::env::set_current_dir(cur_dir).unwrap();
        read_content
    }
}
