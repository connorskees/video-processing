use atom_macro::mp4_media_data_type_atom;

use crate::{data_structures::PascalString, Fixed32};

#[mp4_media_data_type_atom]
pub struct BaseSampleDescriptionTable {
    pub data_format: [u8; 4],
    pub reserved: [u8; 6],
    pub data_reference_index: u16,
    pub rest: Vec<u8>,
}

#[derive(Debug, Clone)]
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
    pub compressor_name: PascalString,
    pub depth: u16,
    pub color_table_id: u16,

    pub remainder: Vec<u8>,
}

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
