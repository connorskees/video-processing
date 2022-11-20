use std::{
    io::{self, Seek, SeekFrom, BufRead},
};

use atom_macro::{mp4_atom, mp4_container_atom};

use crate::{data_structures::{Matrix}, Fixed16, Fixed32, Mp4, Parse, Reference};

pub use header::*;
pub use media_data_type::*;
pub use track::*;

mod header;
mod media_data_type;
mod track;

#[derive(Debug, Clone)]
struct UnparsedAtom {
    offset: u64,
    len: u64,
    #[allow(dead_code)]
    header: Header,
}

impl Parse for UnparsedAtom {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let offset = mp4.reader.buffer.stream_position()?;
        let len = mp4.read_atom_len()?;
        let header = Header(mp4.reader.read_bytes_const::<4>()?);

        let current_pos = mp4.reader.buffer.stream_position()?;
        mp4.reader
            .buffer
            .seek(SeekFrom::Current((len + offset - current_pos) as i64))?;

        Ok(Self {
            offset,
            len,
            header,
        })
    }
}

impl UnparsedAtom {
    pub fn into_ref<P: Parse>(self) -> Reference<P> {
        Reference::new(self.offset, self.len)
    }
}

#[mp4_atom]
pub struct Clip {}

#[mp4_atom]
pub struct Udta {}

#[mp4_atom]
pub struct Ctab {
    pub seed: u32,
    pub flags: u16,
    pub size: u16,
    // todo: maybe use u8 for speed reading
    pub color: Vec<u16>,
}

#[mp4_atom]
pub struct Cmov {}

#[mp4_container_atom]
pub struct Rmra {
    reference_movie_descriptors: Vec<Reference<Rmda>>,
}

#[mp4_atom]
pub struct Prfl {}
#[mp4_atom]
pub struct Mdhd {
    pub version: u8,
    pub flags: [u8; 3],
    pub creation_time: u32,
    pub modification_time: u32,
    pub time_scale: u32,
    pub duration: u32,
    pub language: u16,
    pub quality: u16,
}

#[mp4_atom]
pub struct Elng {}

#[mp4_container_atom]
pub struct Mdia {
    pub mdhd: Reference<Mdhd>,
    pub elng: Option<Reference<Elng>>,
    pub hdlr: Option<Reference<Hdlr>>,
    pub minf: Option<Reference<Minf>>,
    pub udta: Option<Reference<Udta>>,
}

#[mp4_atom]
pub struct Hdlr {
    pub version: u8,
    pub flags: [u8; 3],
    pub component_type: [u8; 4],
    pub component_subtype: [u8; 4],
    pub component_manufacturer: u32,
    pub component_flags: u32,
    pub component_flags_mask: u32,
    pub component_name: String,
}

#[mp4_container_atom]
pub struct Minf {
    pub vmhd: Option<Reference<Vmhd>>,
    pub smhd: Option<Reference<Smhd>>,
    pub hdlr: Option<Reference<Hdlr>>,
    pub gmhd: Option<Reference<Gmhd>>,
    pub dinf: Option<Reference<Dinf>>,
    pub stbl: Option<Reference<Stbl>>,
}

#[mp4_atom]
pub struct Vmhd {
    pub version: u8,
    pub flags: [u8; 3],
    pub graphics_mode: u16,
    pub op_color: [u8; 6],
}

#[mp4_atom]
pub struct Smhd {
    pub version: u8,
    pub flags: [u8; 3],
    pub balance: u16,
    pub reserved: u16,
}

#[mp4_atom]
pub struct Gmhd {}

#[mp4_container_atom]
pub struct Dinf {
    data_reference: Reference<Dref>,
}

#[mp4_container_atom]
pub struct Stbl {
    sample_description: Option<Reference<Stsd>>,
    time_to_sample: Option<Reference<Stts>>,
    composition_offset: Option<Reference<Ctts>>,
    cslg: Option<Reference<Cslg>>,
    sync_sample: Option<Reference<Stss>>,
    stps: Option<Reference<Stps>>,
    sample_to_chunk: Option<Reference<Stsc>>,
    sample_size: Option<Reference<Stsz>>,
    chunk_offset: Option<Reference<Stco>>,
    shadow_sync: Option<Reference<Stsh>>,
    sgpd: Option<Reference<Sgpd>>,
    sbgp: Option<Reference<Sbgp>>,
    sdtp: Option<Reference<Sdtp>>,
}

#[mp4_atom]
pub struct Gmin {}
#[mp4_atom]
pub struct Text {}

#[mp4_atom]
pub struct Dref {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub data: Vec<DataRef>,
}

#[derive(Debug, Clone, Copy)]
pub enum DataRef {
    Alis(Reference<Alis>),
    Rsrc(Reference<Rsrc>),
    Url(Reference<Url>),
}

impl Parse for DataRef {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = mp4.peek_header()?;

        Ok(match header {
            Alis::HEADER => DataRef::Alis(<Reference<Alis> as Parse>::parse(mp4)?),
            Rsrc::HEADER => DataRef::Rsrc(<Reference<Rsrc> as Parse>::parse(mp4)?),
            Url::HEADER => DataRef::Url(<Reference<Url> as Parse>::parse(mp4)?),
            _ => panic!(),
        })
    }
}

#[mp4_atom]
pub struct Alis {
    pub version: u8,
    pub flags: [u8; 3],
    pub data: Vec<u8>,
}
#[mp4_atom]
pub struct Rsrc {
    pub version: u8,
    pub flags: [u8; 3],
    pub data: Vec<u8>,
}
#[mp4_atom]
pub struct Url {
    pub version: u8,
    pub flags: [u8; 3],
    pub data: Vec<u8>,
}

#[mp4_atom]
pub struct Stsd {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub entries: Vec<Reference<BaseSampleDescriptionTable>>,
}

#[mp4_atom]
pub struct Stts {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub time_to_sample_table: Vec<u8>,
}

#[mp4_atom]
pub struct Ctts {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub composition_offset_table: Vec<CompositionOffsetEntry>,
}

#[derive(Debug, Clone, Copy)]
pub struct CompositionOffsetEntry {
    pub sample_count: u32,
    pub composition_offset: i32,
}

impl Parse for CompositionOffsetEntry {
    fn parse<R: Seek + BufRead>(mp4: &mut Mp4<'_, R>) -> io::Result<Self>
        where
            Self: Sized {
        let sample_count = mp4.reader.read_u32()?;
        let composition_offset = mp4.reader.read_i32()?;

        Ok(Self { sample_count, composition_offset })
    }
}

#[mp4_atom]
pub struct Cslg {
    pub version: u8,
    pub flags: [u8; 3],
    pub composition_offset_to_display_offset_shift: u32,
    pub least_display_offset: i32,
    pub greatest_display_offset: i32,
    pub display_start_time: i32,
    pub display_end_time: i32,
}

#[mp4_atom]
pub struct Stss {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub sync_sample_table: Vec<u8>,
}

#[mp4_atom]
pub struct Stps {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub partial_sync_sample_table: Vec<u8>,
}
#[mp4_atom]
pub struct Stsc {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub sample_to_chunk_table: Vec<u8>,
}
#[mp4_atom]
pub struct Stsz {
    pub version: u8,
    pub flags: [u8; 3],
    pub sample_size: u32,
    pub number_of_entries: u32,
    pub sample_size_table: Vec<u8>,
}
#[mp4_atom]
pub struct Stsh {}
#[mp4_atom]
pub struct Sgpd {}
#[mp4_atom]
pub struct Sbgp {}
#[mp4_atom]
pub struct Sdtp {
    pub version: u8,
    pub flags: [u8; 3],
    pub sample_dependency_flags_table: Vec<u8>,
}
#[mp4_container_atom]
pub struct Rmda {
    rdrf: Option<Reference<Rdrf>>,
    rmdr: Option<Reference<Rmdr>>,
    rmcs: Option<Reference<Rmcs>>,
    rmvc: Option<Reference<Rmvc>>,
    rmcd: Option<Reference<Rmcd>>,
    rmqu: Option<Reference<Rmqu>>,
}
#[mp4_atom]
pub struct Rdrf {}
#[mp4_atom]
pub struct Rmdr {}
#[mp4_atom]
pub struct Rmcs {}
#[mp4_atom]
pub struct Rmvc {}
#[mp4_atom]
pub struct Rmcd {}
#[mp4_atom]
pub struct Rmqu {}
#[mp4_atom]
pub struct Crgn {}
#[mp4_atom]
pub struct Kmat {}
#[mp4_atom]
pub struct Elst {}
#[mp4_atom]
pub struct Clef {}
#[mp4_atom]
pub struct Prof {}
#[mp4_atom]
pub struct Enof {}
#[mp4_atom]
pub struct Ssrc {}
#[mp4_atom]
pub struct Obid {}

#[derive(Debug, Clone)]
enum InternalElement<T> {
    Searched(T),
    NotSearched,
}

#[mp4_container_atom]
pub struct Moov {
    pub movie_header: Reference<Mvhd>,
    pub clip: Option<Reference<Clip>>,
    pub trak: Vec<Reference<Trak>>,
    pub udta: Reference<Udta>,
    pub ctab: Reference<Udta>,
    pub cmov: Reference<Cmov>,
    pub rmra: Reference<Rmra>,
}

#[mp4_atom]
pub struct Ftyp {
    pub major_brand: [u8; 4],
    pub major_brand_version: [u8; 4],
    pub compatible_brands: Vec<u8>,
}

#[mp4_atom]
pub struct Mvhd {
    pub version: u8,
    pub flags: [u8; 3],
    pub creation_time: u32,
    pub modification_time: u32,
    pub time_scale: u32,
    pub duration: u32,
    pub preferred_rate: Fixed32,
    pub preferred_volume: Fixed16,
    pub reserved: [u8; 10],
    pub matrix: Matrix,
    pub preview_time: u32,
    pub preview_duration: u32,
    pub poster_time: u32,
    pub selection_time: u32,
    pub selection_duration: u32,
    pub current_time: u32,
    pub next_track_id: u32,
}

#[mp4_atom]
pub struct Stco {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub chunk_offset_table: Vec<u32>,
}

#[mp4_atom]
pub struct Co64 {
    pub version: u8,
    pub flags: [u8; 3],
    pub number_of_entries: u32,
    pub chunk_offset_table: Vec<u64>,
}

#[mp4_atom]
pub struct Mdat {
    pub bytes: Vec<u8>,
}
