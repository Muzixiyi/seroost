use std::{
    fs::{self, File},
    io::{self, BufWriter},
    path::Path,
};

use crate::index::Model;

pub fn write_index(model: &Model, path: &Path) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    serde_json::to_writer_pretty(BufWriter::new(file), model)?;
    Ok(())
}
