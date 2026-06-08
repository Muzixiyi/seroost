use std::{fs::File, io, path::Path};

use crate::index::TermFreqIndex;

pub fn read_index(file_path: &Path) -> Result<TermFreqIndex, io::Error> {
    let file = File::open(file_path)?;
    let index = serde_json::from_reader(file)?;
    Ok(index)
}
