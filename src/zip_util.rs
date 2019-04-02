use std::io::prelude::*;
use std::io::Cursor;
use zip::read::{ZipArchive, ZipFile};

type PseudoFile = Cursor<Vec<u8>>;

pub fn zip_to_pseudofile(mut zip: ZipFile) -> Result<PseudoFile, zip::result::ZipError> {
    let mut tmp = Cursor::new(Vec::with_capacity(zip.size() as usize));
    zip.read_to_end(tmp.get_mut())?;

    let mut inner = ZipArchive::new(tmp)?;
    let mut inner_file = inner.by_index(0)?;
    let mut tmp2 = Cursor::new(Vec::with_capacity(inner_file.size() as usize));
    inner_file.read_to_end(tmp2.get_mut())?;

    Ok(tmp2)
}

pub fn pseudofile_to_str(file: &PseudoFile) -> &str {
    use std::str;
    str::from_utf8(&file.get_ref()[..]).expect("File should be valid UTF-8!")
}
