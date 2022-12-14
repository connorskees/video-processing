use std::io::{self, BufRead, Seek};

use crate::{Mp4, Parse};

#[derive(Debug, Clone)]
pub struct CString(String);

impl Parse for CString {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(CString(
            String::from_utf8(mp4.reader.read_until(b'\0')?).unwrap(),
        ))
    }
}
