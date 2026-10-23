//! Bounded CLI filesystem reads and shared invalid-data errors.
use std::{
    error::Error,
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

pub(crate) fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}
pub(crate) fn read(path: &Path, left: &mut usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(*left as u64 + 1)
        .read_to_end(&mut bytes)?;
    *left = left
        .checked_sub(bytes.len())
        .ok_or_else(|| invalid("aggregate tree/bundle input byte limit"))?;
    Ok(bytes)
}
pub(crate) fn read_release_input(
    path: &Path,
    remaining: &mut usize,
    maximum: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("release input must be a regular file"));
    }
    let mut bounded = (*remaining).min(maximum);
    let bytes = read(path, &mut bounded)?;
    *remaining -= bytes.len();
    Ok(bytes)
}
