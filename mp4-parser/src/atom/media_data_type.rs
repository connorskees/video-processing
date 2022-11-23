use std::io;

use atom_macro::{mp4_atom, mp4_media_data_type_atom};

use crate::{Fixed32, Parse};

use super::header::*;

#[mp4_media_data_type_atom]
pub struct BaseSampleDescriptionTable {
    pub data_format: [u8; 4],
    pub reserved: [u8; 6],
    pub data_reference_index: u16,
    pub rest: Vec<u8>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SampleDescriptionTable {
    Video(SampleVideoDescriptionTable),
}

#[mp4_media_data_type_atom]
pub struct SampleVideoDescriptionTable {
    pub data_format: [u8; 4],
    pub reserved: [u8; 6],
    pub data_reference_index: u16,

    pub version: u16,
    pub revision_level: u16,
    pub vendor: u32,
    pub temporal_quality: u32,
    pub spatial_quality: u32,
    pub width: u16,
    pub height: u16,
    pub horizontal_resolution: Fixed32,
    pub vertical_resolution: Fixed32,
    pub data_size: u32,
    pub frame_count: u16,
    pub compressor_name: [u8; 32],
    pub depth: u16,
    pub color_table_id: i16,

    pub extensions: Vec<VideoSampleExtension>,
}

impl SampleVideoDescriptionTable {
    pub fn avcc(&self) -> Option<&AvcC> {
        self.extensions
            .iter()
            .filter_map(|ext| match ext {
                VideoSampleExtension::AvcC(a) => Some(a),
                _ => None,
            })
            .next()
    }
}

#[derive(Debug, Clone)]
pub enum VideoSampleExtension {
    Gama,
    Fiel,
    Mjqt,
    Mjht,
    Esds,
    AvcC(AvcC),
    Pasp,
    Colr,
    Clap,
}

impl Parse for VideoSampleExtension {
    fn parse<R: io::Seek + io::BufRead>(mp4: &mut crate::Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        Ok(match mp4.peek_header()? {
            GAMA => todo!(),
            FIEL => todo!(),
            MJQT => todo!(),
            MJHT => todo!(),
            ESDS => todo!(),
            AVCC => VideoSampleExtension::AvcC(AvcC::parse(mp4)?),
            PASP => todo!(),
            COLR => todo!(),
            CLAP => todo!(),
            _ => todo!(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AvcC {
    pub configuration_version: u8,
    pub avc_profile_indication: u8,
    pub profile_compatibility: u8,
    pub avc_level_indication: u8,
    pub length_size_minus_one: u8,
    pub sequence_parameter_sets: Vec<SequenceParameterSet>,
    pub picture_parameter_sets: Vec<PictureParameterSet>,
    pub extension: Option<AvcCExtension>,
}

impl Parse for AvcC {
    fn parse<R: io::Seek + io::BufRead>(mp4: &mut crate::Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let offset = mp4.reader.buffer.stream_position()?;
        let len = mp4.reader.read_u32()?;
        mp4.expect_header(AVCC)?;
        let configuration_version = mp4.reader.read_u8()?;
        let avc_profile_indication = mp4.reader.read_u8()?;
        let profile_compatibility = mp4.reader.read_u8()?;
        let avc_level_indication = mp4.reader.read_u8()?;
        let length_size_minus_one = mp4.reader.read_u8()? & 0b0000_0011;
        let num_of_sequence_parameter_sets = mp4.reader.read_u8()? & 0b0001_1111;
        let sps = (0..num_of_sequence_parameter_sets)
            .map(|_| SequenceParameterSet::parse(mp4))
            .collect::<io::Result<Vec<_>>>()?;
        let num_of_picture_parameter_sets = mp4.reader.read_u8()?;
        let pps = (0..num_of_picture_parameter_sets)
            .map(|_| PictureParameterSet::parse(mp4))
            .collect::<io::Result<Vec<_>>>()?;

        let extension = if [100, 110, 122, 144].contains(&profile_compatibility) {
            Some(AvcCExtension::parse(mp4)?)
        } else {
            None
        };

        assert_eq!(
            offset,
            mp4.reader
                .buffer
                .stream_position()?
                .saturating_sub(len as u64)
        );

        Ok(Self {
            configuration_version,
            avc_profile_indication,
            profile_compatibility,
            avc_level_indication,
            length_size_minus_one,
            sequence_parameter_sets: sps,
            picture_parameter_sets: pps,
            extension,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AvcCExtension {
    pub chroma_format: u8,
    pub bit_depth_luma_minus8: u8,
    pub bit_depth_chroma_minus8: u8,
    pub sequence_parameter_set_extension: Vec<SequenceParameterSet>,
}

impl Parse for AvcCExtension {
    fn parse<R: io::Seek + io::BufRead>(mp4: &mut crate::Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let chroma_format = mp4.reader.read_u8()? & 0b0000_0011;
        let bit_depth_luma_minus8 = mp4.reader.read_u8()? & 0b0000_0111;
        let bit_depth_chroma_minus8 = mp4.reader.read_u8()? & 0b0000_0111;
        let num_of_sequence_parameter_set_ext = mp4.reader.read_u8()?;
        let sequence_parameter_set_extension = (0..num_of_sequence_parameter_set_ext)
            .map(|_| SequenceParameterSet::parse(mp4))
            .collect::<io::Result<Vec<_>>>()?;

        Ok(Self {
            chroma_format,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            sequence_parameter_set_extension,
        })
    }
}

#[derive(Debug, Clone)]
pub struct SequenceParameterSet {
    pub len: u16,
    pub nal_unit: Vec<u8>,
}

impl Parse for SequenceParameterSet {
    fn parse<R: io::Seek + io::BufRead>(mp4: &mut crate::Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len = mp4.reader.read_u16()?;
        let nal_unit = mp4.reader.read_bytes_dyn(len as usize)?;

        Ok(Self { len, nal_unit })
    }
}

#[derive(Debug, Clone)]
pub struct PictureParameterSet {
    pub len: u16,
    pub nal_unit: Vec<u8>,
}

impl Parse for PictureParameterSet {
    fn parse<R: io::Seek + io::BufRead>(mp4: &mut crate::Mp4<'_, R>) -> io::Result<Self>
    where
        Self: Sized,
    {
        let len = mp4.reader.read_u16()?;
        let nal_unit = mp4.reader.read_bytes_dyn(len as usize)?;

        Ok(Self { len, nal_unit })
    }
}

#[mp4_atom]
pub struct Gama {}
#[mp4_atom]
pub struct Fiel {}
#[mp4_atom]
pub struct Mjqt {}
#[mp4_atom]
pub struct Mjht {}
#[mp4_atom]
pub struct Esds {}
#[mp4_atom]
pub struct Pasp {}
#[mp4_atom]
pub struct Colr {}
#[mp4_atom]
pub struct Clap {}

#[mp4_media_data_type_atom]
pub struct SampleSoundVersion0DescriptionTable {
    pub data_format: [u8; 4],
    pub reserved: [u8; 6],
    pub data_reference_index: u16,

    pub version: u16,
    pub revision_level: u16,
    pub vendor: u32,
    pub number_of_channels: u16,
    pub sample_size: u16,
    pub compression_id: u16,
    pub packet_size: u16,
    pub sample_rate: Fixed32,
}

#[mp4_media_data_type_atom]
pub struct SampleSoundVersion1DescriptionTable {
    pub data_format: [u8; 4],
    pub reserved: [u8; 6],
    pub data_reference_index: u16,

    pub version: u16,
    pub revision_level: u16,
    pub vendor: u32,
    pub number_of_channels: u16,
    pub sample_size: u16,
    pub compression_id: u16,
    pub packet_size: u16,
    pub sample_rate: Fixed32,
}
