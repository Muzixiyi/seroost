use std::{fs::File, io};

use crate::index::TermFreqIndex;

pub fn read_index(path: &str) -> Result<TermFreqIndex, io::Error> {
    let file = File::open(path)?;
    let index = serde_json::from_reader(file)?;
    Ok(index)
}
