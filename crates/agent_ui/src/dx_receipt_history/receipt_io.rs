use serde_json::Value;
use std::{
    fs::File,
    io::{Read, Take},
    path::Path,
};

const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

pub(super) fn read_json(path: &Path) -> Option<Value> {
    let file = File::open(path).ok()?;
    serde_json::from_reader(receipt_reader(file)).ok()
}

fn receipt_reader(file: File) -> Take<File> {
    file.take(MAX_RECEIPT_BYTES)
}
