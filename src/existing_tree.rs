use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

pub type ETNodeRef = Rc<RefCell<ExistingTreeNode>>;

#[derive(Debug)]
pub enum ExistingTreeNode {
    Unknown,
    NormalDir {
        children: Vec<ETNodeRef>,
        path: PathBuf,
        // timestamp: SystemTime,
    },
    File {
        path: PathBuf,
        timestamp: SystemTime,
    },
}