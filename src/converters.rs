use std::path::PathBuf;
use subprocess::Exec;

#[allow(dead_code)]
pub struct Converter {
    pub name: String,
    pub path: PathBuf,
}

impl Converter {
    pub fn convert(&self, content: Vec<u8>) -> Vec<u8> {
        let cur_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&self.path.parent().unwrap()).unwrap();
        let mut temp_dir = PathBuf::from(".");
        temp_dir.push(&self.path.file_name().unwrap());
        // debug!("invoking {:?}", temp_dir);
        let read_content = Exec::cmd(temp_dir)
            .stdin(content)
            .capture()
            .expect("converter error")
            .stdout;
        std::env::set_current_dir(cur_dir).unwrap();
        read_content
    }
}
