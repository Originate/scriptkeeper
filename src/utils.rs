use crate::R;
use std::path::Path;

pub fn path_to_string(path: &Path) -> R<&str> {
    Ok(path
        .to_str()
        .ok_or_else(|| format!("invalid utf8 sequence: {:?}", &path))?)
}
