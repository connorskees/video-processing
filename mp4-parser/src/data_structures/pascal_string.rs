use std::io::{Seek, BufRead, self};

use crate::{Parse, Mp4};


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
