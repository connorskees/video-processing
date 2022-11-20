use std::io::{self, BufRead, Seek};

use crate::{Mp4, Parse};

#[derive(Debug, Clone)]
pub struct PascalString(String);

impl Parse for PascalString {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len = mp4.reader.read_u32()?;
        Ok(PascalString(
            String::from_utf8(mp4.reader.read_bytes_dyn(len as usize)?).unwrap(),
        ))
    }
}
