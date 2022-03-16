use crate::batch_iterator::BatchIterator;
use liquid::model::Value;
use std::path::PathBuf;
use std::slice::Iter;

pub struct Paginator {
    seq: Vec<Value>,
    batch_size: usize,
    base_path: PathBuf,
    batch_paths: Vec<PathBuf>,
}

impl Paginator {
    pub fn new(seq: &Vec<Value>, batch_size: usize, base_path: PathBuf) -> Self {
        let batch_paths =
            Self::gen_batch_paths(Self::calc_batch_num(seq.len(), batch_size), &base_path);
        Self {
            seq: seq.clone(),
            batch_size,
            base_path,
            batch_paths,
        }
    }
    pub fn from_expression_and_object<'a>(
        globals: &liquid::Object,
        expression: &'a String,
        batch_size: usize,
        base_path: PathBuf,
    ) -> Result<Self, &'a str> {
        let exp_spl = expression.split('.');
        let mut temp = globals;
        let mut seq = vec![];
        let mut flag = false;
        for key in exp_spl {
            if flag {
                return Err("Invalid expression of array");
            }
            match temp.get(key) {
                Some(liquid::model::Value::Object(obj)) => {
                    temp = obj;
                }
                Some(liquid::model::Value::Array(arr)) => {
                    flag = true;
                    seq.extend(arr.iter().map(|x| x.clone()));
                }
                _ => return Err("Invalid expression of array"),
            }
        }
        let batch_paths =
            Self::gen_batch_paths(Self::calc_batch_num(seq.len(), batch_size), &base_path);
        Ok(Self {
            seq,
            batch_size,
            base_path,
            batch_paths,
        })
    }
    pub fn batch_iter(&self) -> BatchIterator<Iter<'_, Value>, Iter<'_, PathBuf>> {
        let temp = self.seq.iter().clone();
        BatchIterator::new(temp, self.batch_paths.iter(), self.batch_size)
    }
    pub fn base_url_dir(&self) -> PathBuf {
        let mut temp_prefix = PathBuf::from(&self.base_path);
        let stem: String = temp_prefix
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        temp_prefix.pop();
        temp_prefix.push(stem);
        temp_prefix
    }
    pub fn calc_batch_num(seq_len: usize, batch_size: usize) -> usize {
        (seq_len as f64 / batch_size as f64).ceil() as usize
    }
    pub fn batch_num(&self) -> usize {
        Self::calc_batch_num(self.seq.len(), self.batch_size)
    }
    pub fn gen_paginator_object(&self) -> liquid::Object {
        liquid::object!({
            "items": self.seq,
            "batch_num": self.batch_num(),
        })
    }
    pub fn batch_paths(&self) -> &Vec<PathBuf> {
        &self.batch_paths
    }
    fn gen_batch_paths(batch_num: usize, base_path: &PathBuf) -> Vec<PathBuf> {
        let mut batch_paths = vec![base_path.clone()];
        let mut temp_prefix = PathBuf::from(base_path);
        let stem: String = temp_prefix
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        temp_prefix.pop();
        temp_prefix.push(stem);
        for i in 1..batch_num {
            temp_prefix.push(i.to_string() + ".html");
            batch_paths.push(temp_prefix.clone());
            temp_prefix.pop();
        }
        batch_paths
    }
}
