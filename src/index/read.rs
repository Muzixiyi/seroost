use std::{
    fs::File,
    io::{self, BufReader},
    path::Path,
};

use crate::index::Model;

pub fn read_index(file_path: &Path) -> Result<Model, io::Error> {
    let file = File::open(file_path)?;
    let index = serde_json::from_reader(BufReader::new(file))?;
    Ok(index)
}
