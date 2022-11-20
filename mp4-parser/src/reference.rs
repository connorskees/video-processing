use std::{
    io::{self, BufRead, Seek},
    marker::PhantomData,
};

use crate::{BaseSampleDescriptionTable, Mp4, Parse, SampleDescriptionTable};

#[derive(Debug, Clone)]
pub struct Reference<P: Parse> {
    pub offset: u64,
    pub len: u64,
    _a: PhantomData<P>,
}

impl<P: Parse + Clone> Copy for Reference<P> {}

impl<P: Parse> Parse for Reference<P> {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let offset = mp4.reader.buffer.stream_position()?;
        let len = P::peek_len(mp4)?;
        mp4.reader.skip(len as i64)?;
        Ok(Reference {
            offset,
            len,
            _a: PhantomData,
        })
    }

    fn peek_len<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<u64> {
        P::peek_len(mp4)
    }
}

impl<P: Parse> Reference<P> {
    pub fn new(offset: u64, len: u64) -> Self {
        Reference {
            offset,
            len,
            _a: PhantomData,
        }
    }

    pub fn parse<R: Seek + BufRead>(self, mp4: &mut Mp4<'_, R>) -> io::Result<P> {
        mp4.jump_to(self.offset).unwrap();
        P::parse(mp4)
    }
}

impl Reference<BaseSampleDescriptionTable> {
    pub fn parse_sample_description<R: Seek + BufRead>(
        self,
        mp4: &mut Mp4<'_, R>,
        subtype: [u8; 4],
    ) -> io::Result<SampleDescriptionTable> {
        mp4.parse_sample_description(self, subtype)
    }
}
