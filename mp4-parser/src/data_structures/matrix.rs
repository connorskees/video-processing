use std::io::{self, BufRead, Seek};

use crate::{Fixed32, Parse};

#[derive(Debug, Clone)]
pub struct Matrix {
    pub a: Fixed32,
    pub b: Fixed32,
    pub u: Fixed32,
    pub c: Fixed32,
    pub d: Fixed32,
    pub v: Fixed32,
    pub x: Fixed32,
    pub y: Fixed32,
    pub w: Fixed32,
}

impl Parse for Matrix {
    fn parse<R: Seek + BufRead>(mp4: &mut crate::Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        mp4.read_matrix()
    }

    fn peek_len<R: Seek + BufRead>(_mp4: &mut crate::Mp4<'_, R>) -> io::Result<u64> {
        Ok(36)
    }
}
