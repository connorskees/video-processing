#![feature(seek_stream_len)]
#![feature(drain_filter)]

// todo: fn for getting all unknown chunks, method for getting atom by header

extern crate atom_macro;

use std::{
    io::{self, BufRead, Seek, SeekFrom},
    marker::PhantomData,
};

pub use atom::*;
use data_structures::Matrix;
pub use reference::*;

mod atom;
pub mod data_structures;
mod reference;

pub type Fixed16 = u16;
pub type Fixed32 = u32;

pub trait Parse {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized;

    fn peek_len<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<u64> {
        let len = mp4.read_atom_len()?;

        mp4.reader.go_backwards(4)?;

        Ok(len)
    }
}

impl<const N: usize> Parse for [u8; N] {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        mp4.reader.read_bytes_const::<N>()
    }

    fn peek_len<R: Seek + BufRead>(_mp4: &mut Mp4<'_, R>) -> io::Result<u64> {
        Ok(N as u64)
    }
}

pub struct Mp4<'a, R: BufRead + Seek> {
    _a: PhantomData<&'a ()>,
    reader: Reader<R>,
}

impl<'a, R: BufRead + Seek> Mp4<'a, R> {
    pub fn new(buffer: R) -> Self {
        Self {
            reader: Reader::new(buffer),
            _a: PhantomData,
        }
    }

    #[track_caller]
    fn expect_header(&mut self, header: Header) -> io::Result<()> {
        assert_eq!(Header(self.reader.read_bytes_const::<4>()?), header);

        Ok(())
    }

    fn read_atom_len(&mut self) -> io::Result<u64> {
        let len = self.reader.read_u32()?;

        // allowed only for a top-level atom, designates the last atom in the
        // file and indicates that the atom extends to the end of the file
        assert_ne!(len, 0);
        // the actual size is given in the extended size field, an optional 64-bit
        // field that follows the type field
        assert_ne!(len, 1);

        Ok(len as u64)
    }

    pub fn jump_to(&mut self, offset: u64) -> io::Result<u64> {
        self.reader.buffer.seek(SeekFrom::Start(offset))
    }

    pub fn read_matrix(&mut self) -> io::Result<Matrix> {
        let a = self.reader.read_u32()?;
        let b = self.reader.read_u32()?;
        let u = self.reader.read_u32()?;
        let c = self.reader.read_u32()?;
        let d = self.reader.read_u32()?;
        let v = self.reader.read_u32()?;
        let x = self.reader.read_u32()?;
        let y = self.reader.read_u32()?;
        let w = self.reader.read_u32()?;

        Ok(Matrix {
            a,
            b,
            u,
            c,
            d,
            v,
            x,
            y,
            w,
        })
    }

    pub fn read_parseable<P: Parse>(&mut self, offset: usize) -> io::Result<P> {
        self.reader.buffer.seek(SeekFrom::Start(offset as u64))?;

        P::parse(self)
    }

    pub fn parse_sample_description(
        &mut self,
        base: Reference<BaseSampleDescriptionTable>,
        subtype: [u8; 4],
    ) -> io::Result<SampleDescriptionTable> {
        self.jump_to(base.offset)?;
        Ok(match &subtype {
            b"vide" => SampleDescriptionTable::Video(SampleVideoDescriptionTable::parse(self)?),
            b"soun" => todo!(),
            b"subt" => todo!(),
            b"meta" => todo!(),
            b"twen" => todo!(),
            b"sprt" => todo!(),
            b"MPEG" => todo!(),
            b"musi" => todo!(),
            b"sbtl" => todo!(),
            b"clcp" => todo!(),
            b"text" => todo!(),
            b"qd3d" => todo!(),
            b"strm" => todo!(),
            _ => unimplemented!(),
        })
    }

    pub fn skip_chunk(&mut self) -> io::Result<u64> {
        let len = self.reader.read_u32()?;

        self.reader.skip(len as i64 - 4)
    }

    pub fn peek_header(&mut self) -> io::Result<Header> {
        let offset = self.reader.buffer.stream_position()?;
        self.read_atom_len()?;

        let header = Header(self.reader.read_bytes_const::<4>()?);

        self.reader.buffer.seek(SeekFrom::Start(offset))?;

        Ok(header)
    }
}

trait FromBeBytes<const N: usize> {
    fn from_bytes_be(bytes: [u8; N]) -> Self;
}

macro_rules! impl_from_bytes {
    ($type:ty, $len:literal) => {
        impl FromBeBytes<$len> for $type {
            fn from_bytes_be(bytes: [u8; $len]) -> Self {
                Self::from_be_bytes(bytes)
            }
        }

        impl Parse for $type {
            fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
            where
                Self: Sized,
            {
                mp4.reader.read_bytes::<$len, $type>()
            }

            fn peek_len<R: Seek + BufRead>(_mp4: &mut Mp4<'_, R>) -> io::Result<u64> {
                Ok($len as u64)
            }
        }
    };
}

impl_from_bytes!(u8, 1);
impl_from_bytes!(u16, 2);
impl_from_bytes!(u32, 4);
impl_from_bytes!(i32, 4);
impl_from_bytes!(u64, 8);

struct Reader<R: BufRead + Seek> {
    buffer: R,
}

impl<R: BufRead + Seek> Reader<R> {
    pub fn new(buffer: R) -> Self {
        Self { buffer }
    }

    fn read_bytes<const BYTES: usize, N: FromBeBytes<BYTES>>(&mut self) -> io::Result<N> {
        let bytes = self.read_bytes_const::<BYTES>()?;

        Ok(N::from_bytes_be(bytes))
    }

    #[allow(dead_code)]
    pub fn read_u8(&mut self) -> io::Result<u8> {
        self.read_bytes::<1, u8>()
    }

    #[allow(dead_code)]
    pub fn read_u16(&mut self) -> io::Result<u16> {
        self.read_bytes::<2, u16>()
    }

    pub fn read_u32(&mut self) -> io::Result<u32> {
        self.read_bytes::<4, u32>()
    }

    pub fn read_i32(&mut self) -> io::Result<i32> {
        self.read_bytes::<4, i32>()
    }

    #[allow(dead_code)]
    pub fn read_u64(&mut self) -> io::Result<u64> {
        self.read_bytes::<8, u64>()
    }

    pub fn read_bytes_const<const BYTES: usize>(&mut self) -> io::Result<[u8; BYTES]> {
        let mut bytes = [0; BYTES];
        self.buffer.read_exact(&mut bytes)?;

        Ok(bytes)
    }

    pub fn skip(&mut self, n: i64) -> io::Result<u64> {
        self.buffer.seek(SeekFrom::Current(n))
    }

    pub fn go_backwards(&mut self, n: u64) -> io::Result<u64> {
        self.buffer.seek(SeekFrom::Current(-(n as i64)))
    }

    pub fn read_bytes_dyn(&mut self, n: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0; n];
        self.buffer.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read_until(&mut self, b: u8) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        self.buffer.read_until(b, &mut buf)?;
        Ok(buf)
    }
}
