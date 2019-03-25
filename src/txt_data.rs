use crate::error::{Error, Result};
use std::io::prelude::*;
use std::path::Path;

#[derive(Debug)]
pub struct DataFile {
    buf: String,
}

impl DataFile {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<DataFile> {
        let mut file = std::fs::File::open(path)?;
        let mut buf = String::new();
        file.read_to_string(&mut buf);

        Ok(DataFile { buf: buf })
    }

    pub fn from_reader<B: Read>(reader: &mut B) -> Result<DataFile> {
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(DataFile { buf: String::from_utf8_lossy(& buf).into_owned() })
    }
}

struct Span(usize, usize);

impl DataFile {
    pub fn records<'a, 'b>(&'a self, ty: &'b str, delimiters: &'b [(usize, usize)]) -> RecordIter<'a, 'b> {
        let delimiters = delimiters.iter().map(|&(p, l)| Span(p, p + l)).collect::<Vec<_>>();
        //assert!(ty.len() <= delimiters[0].1);
        RecordIter {
            lines: self.buf.lines(),
            ty,
            delimiters
        }
    }
}


use std::str::Lines;
pub struct RecordIter<'a, 'b> {
    lines: Lines<'a>,
    ty: &'b str,
    delimiters: Vec<Span>
}

impl<'a, 'b> Iterator for RecordIter<'a, 'b> {
    type Item = Record<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.lines.next()?;
            let Span(l, r) = self.delimiters[0];
            if &line[l..r] == self.ty {
                break Some(Record {
                    fields: self.delimiters.iter().map(|&Span(l,r)| line[l..r].trim()).collect::<Vec<_>>()
                });
            }
        }
    }
}

#[derive(Debug)]
pub struct Record<'a> {
    fields: Vec<&'a str>
}

use std::ops::Index;
impl<'a> Index<usize> for Record<'a> {
    type Output = &'a str;

    fn index(&self, i: usize) -> &Self::Output {
        &self.fields[i]
    }
}