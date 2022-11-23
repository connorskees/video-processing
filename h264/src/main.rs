// #![allow(dead_code, unused)]

use std::{
    borrow::Cow,
    fs,
    io::{self, BufReader},
    mem::{self, MaybeUninit},
    ops::{Add, AddAssign, Sub},
};

// search:
// _flag == 0

// use h264_reader::rbsp::BitReader;
// use openh264::nal_units;
// use openh264::{decoder::Decoder, to_bitstream_with_001_be};

use bitvec::{field::BitField, macros::internal::funty::Integral, slice::BitSlice};
use mp4_parser::{Mdat, Moov, Mp4, Parse, SampleDescriptionTable};

use num_traits::{One, Pow, Signed, Unsigned};
use sei::{SeiMessage, UserDataUnregistered};
use slice_layer::*;
use sps::{Hrd, SeqParameterSetData, Vui};

use crate::sps::ScalingList;

mod sei;
mod slice_layer;
mod sps;

trait Integer:
    One
    + Add<Self, Output = Self>
    + Sub<Self, Output = Self>
    + AddAssign
    + Pow<u8, Output = Self>
    + Integral
{
}
impl<T> Integer for T where
    T: One
        + Add<Self, Output = Self>
        + Sub<Self, Output = Self>
        + AddAssign
        + Pow<u8, Output = Self>
        + Integral
{
}

#[allow(dead_code)]
#[derive(Debug)]
enum NalU {
    NonIdrSliceLayerWithoutPartitioning,
    SliceDataPartitionALayer,
    SliceDataPartitionBLayer,
    SliceDataPartitionCLayer,
    IdrSliceLayerWithoutPartitioning,
    Sei(Vec<SeiMessage>),
    SeqParameterSet,
    PicParameterSet,
    AccessUnitDelimiter,
    EndOfSeq,
    EndOfStream,
    FillerData,
    SeqParameterSetExtension,
    PrefixNalUnit,
    SubsetSeqParameterSet,
    AuxiliarySliceLayerWithoutPartitioning,
    SliceLayerExtension,
    DepthViewSliceLayerExtension,
}

struct NalUParser<'a, 'b> {
    buffer: &'a BitSlice<u8, bitvec::order::Msb0>,
    // todo: should we wrap in cell?
    cursor: usize,
    sps: &'b SeqParameterSetData,
}

fn strip_emulation_prevention_three_byte<'a>(b: &'a [u8]) -> Vec<u8> {
    let aho =
        aho_corasick::AhoCorasick::new(&[[0, 0, 3, 0], [0, 0, 3, 1], [0, 0, 3, 2], [0, 0, 3, 3]]);

    aho.replace_all_bytes(b, &[[0, 0, 0], [0, 0, 1], [0, 0, 2], [0, 0, 3]])
}

pub type Uuid = u128;

type NalUResult<T> = Result<T, ()>;

// Public API
impl<'a, 'b> NalUParser<'a, 'b> {
    pub fn new(buffer: &'a [u8], sps: &'b SeqParameterSetData) -> Self {
        Self {
            buffer: BitSlice::from_slice(buffer),
            cursor: 0,
            sps,
        }
    }

    pub fn parse(mut self) -> NalUResult<NalU> {
        let first_byte = self.read_u8()?;

        assert_eq!(first_byte & 0b1000_0000, 0, "forbidden bit set");
        let nal_ref_idc = (first_byte & 0b0110_0000) >> 5;

        let nal_unit_type = first_byte & 0b0001_1111;

        match nal_unit_type {
            // 1 => Self::NonIdrSliceLayerWithoutPartitioning,
            // 2 => Self::SliceDataPartitionALayer,
            // 3 => Self::SliceDataPartitionBLayer,
            // 4 => Self::SliceDataPartitionCLayer,
            5 => {
                assert_ne!(nal_ref_idc, 0);
                self.slice_layer_without_partitioning_rbsp(nal_unit_type)
                // Self::IdrSliceLayerWithoutPartitioning
            }
            6 => dbg!(self.parse_sei()),
            // 7 => Self::SeqParameterSet,
            // 8 => Self::PicParameterSet,
            // 9 => Self::AccessUnitDelimiter,
            // 10 => Self::EndOfSeq,
            // 11 => Self::EndOfStream,
            // 12 => Self::FillerData,
            // 13 => Self::SeqParameterSetExtension,
            // 14 => Self::PrefixNalUnit,
            // 15 => Self::SubsetSeqParameterSet,
            // 19 => Self::AuxiliarySliceLayerWithoutPartitioning,
            // 20 => Self::SliceLayerExtension,
            // 21 => Self::DepthViewSliceLayerExtension,
            v => todo!("unrecognized nal unit type: {:?}", v),
        }
    }
}

/// Parsing primitives
impl<'a, 'b> NalUParser<'a, 'b> {
    #[track_caller]
    fn read_bits<N: Ord + Integral>(&mut self, bits: u8) -> N {
        assert!(
            usize::from(bits) <= mem::size_of::<N>() * 8,
            "bit count {bits} was greater than size of integer ({})",
            mem::size_of::<N>()
        );

        if bits == 0 {
            return N::ZERO;
        }

        let n = self.buffer[self.cursor..self.cursor + bits as usize].load_be::<N>();

        self.cursor += usize::from(bits);

        n
    }

    fn more_rbsp_data(&mut self) -> bool {
        let cursor = self.cursor;

        if self.cursor >= self.buffer.len() {
            return false;
        }

        if self.peek_bit() == Some(0) {
            return true;
        }

        self.next_bit().unwrap();
        while self.cursor < self.buffer.len() - 1 {
            if self.next_bit() == Ok(1) {
                self.cursor = cursor;
                return true;
            }
        }

        self.cursor = cursor;

        return false;
    }

    #[track_caller]
    fn next_bit(&mut self) -> NalUResult<u8> {
        let bit = self.peek_bit().unwrap();
        self.cursor += 1;
        Ok(bit)
    }

    fn peek_bit(&self) -> Option<u8> {
        match self.buffer.get(self.cursor) {
            Some(b) => Some(*b as u8),
            _ => None,
        }
    }

    fn rbsp_trailing_bits(&mut self) -> NalUResult<()> {
        if !self.byte_aligned() {
            assert_eq!(self.next_bit()?, 1);

            while !self.byte_aligned() {
                assert_eq!(self.next_bit()?, 0);
            }
        }

        Ok(())
    }

    fn read_u8(&mut self) -> NalUResult<u8> {
        Ok(self.read_bits::<u8>(8))
    }

    fn read_u16(&mut self) -> NalUResult<u16> {
        Ok(self.read_bits::<u16>(16))
    }

    fn read_u32(&mut self) -> NalUResult<u32> {
        Ok(self.read_bits::<u32>(32))
    }

    fn peek_u8(&mut self) -> Option<u8> {
        if self.buffer.len() < self.cursor + 8 {
            return None;
        }

        Some(self.buffer[self.cursor..self.cursor + 8].load_be::<u8>())
    }

    fn byte_aligned(&self) -> bool {
        self.cursor % 8 == 0
    }
}

#[allow(unused)]
/// Base data types
impl<'a, 'b> NalUParser<'a, 'b> {
    /// context-adaptive arithmetic entropy-coded syntax element
    fn ae_v(&mut self) -> () {
        todo!()
    }

    /// byte having any pattern of bit string
    fn b_8(&mut self) -> u8 {
        todo!()
    }

    /// context-adaptive variable-length entropy-coded syntax element with the left bit first
    fn ce_v(&mut self) -> u8 {
        todo!()
    }

    /// fixed-pattern bit string using n bits written (from left to right) with the left bit first
    fn f_n(&mut self, n: u8) -> u8 {
        todo!()
    }

    /// signed integer using n bits. When n is "v" in the syntax table, the number of bits varies in a manner dependent on the value of other syntax elements
    fn i_n(&mut self, n: u8) -> u8 {
        todo!()
    }

    /// mapped Exp-Golomb-coded syntax element with the left bit first
    fn me_v(&mut self) -> u8 {
        todo!()
    }

    #[track_caller]
    /// signed integer Exp-Golomb-coded syntax element with the left bit first
    fn se_v<N: Integer + Signed>(&mut self) -> NalUResult<N> {
        todo!()
    }

    /// null-terminated string encoded as universal coded character set (UCS) transmission format-8 (UTF-8)
    fn st_v(&mut self) -> u8 {
        todo!()
    }

    /// truncated Exp-Golomb-coded syntax element with left bit first
    fn te_v(&mut self) -> u8 {
        todo!()
    }

    /// unsigned integer using n bits
    #[track_caller]
    fn u_n<N: Integer + Unsigned>(&mut self, bits: u8) -> N {
        self.read_bits::<N>(bits)
    }

    #[track_caller]
    /// unsigned integer Exp-Golomb-coded syntax element with the left bit first
    fn ue_v<N: Integer>(&mut self) -> NalUResult<N> {
        let mut leading_zero_bits = 0;

        let mut b = 0;

        while b == 0 {
            leading_zero_bits += 1;
            b = self.next_bit()?;
        }

        leading_zero_bits -= 1;

        let code_num: N = Pow::pow(N::one() + N::one(), leading_zero_bits) - N::one()
            + self.read_bits::<N>(leading_zero_bits);

        Ok(code_num)
    }

    fn read_uuid(&mut self) -> NalUResult<Uuid> {
        let uuid = self.buffer[self.cursor..self.cursor + 128].load_be::<u128>();

        self.cursor += 128;

        Ok(uuid)
    }
}

/// Slice layer
impl<'a, 'b> NalUParser<'a, 'b> {
    fn slice_layer_without_partitioning_rbsp(&mut self, nal_unit_type: u8) -> NalUResult<NalU> {
        let header = self.slice_header(nal_unit_type)?;
        let data = self.slice_data();
        self.rbsp_slice_trailing_bits();

        todo!()
    }

    fn slice_header(&mut self, nal_unit_type: u8) -> NalUResult<SliceHeader> {
        let first_mb_in_slice = self.ue_v::<u32>()?;
        let slice_type = self.ue_v::<u8>()?;
        let pic_parameter_set_id = self.ue_v::<u8>()?;

        let colour_plane_id = if self.sps.separate_colour_plane_flag != 0 {
            self.read_bits::<u8>(2)
        } else {
            0
        };

        // todo: sps
        let bottom_field_pic_order_in_frame_present_flag = 0;
        let redundant_pic_cnt_present_flag = 0;
        let weighted_pred_flag = 0;
        let weighted_bipred_idc = 0;
        let deblocking_filter_control_present_flag = 0;
        let entropy_coding_mode_flag = 0;
        let num_slice_groups_minus1 = 0;
        let slice_group_map_type = 0;

        let frame_num = self.u_n(self.sps.log2_max_frame_num_minus4 + 4);

        let mut field_pic_flag = 0;
        let mut bottom_field_flag = 0;

        if self.sps.frame_mbs_only_flag != 0 {
            field_pic_flag = self.next_bit()?;
            if field_pic_flag != 0 {
                bottom_field_flag = self.next_bit()?;
            }
        }

        let idr_pic_id = if self.idr_pic_flag() {
            self.ue_v::<u16>()?
        } else {
            0
        };

        let mut pic_order_cnt_lsb = 0;
        let mut delta_pic_order_cnt_bottom = 0;

        if self.sps.pic_order_cnt_type != 0 {
            pic_order_cnt_lsb = self.u_n::<u16>(self.sps.log2_max_pic_order_cnt_lsb_minus4 + 4);

            if bottom_field_pic_order_in_frame_present_flag != 0 && field_pic_flag == 0 {
                delta_pic_order_cnt_bottom = self.se_v()?;
            }
        }

        let mut delta_pic_order_cnt = [0; 2];

        if self.sps.pic_order_cnt_type == 1 && self.sps.delta_pic_order_always_zero_flag == 0 {
            delta_pic_order_cnt[0] = self.se_v()?;

            if bottom_field_pic_order_in_frame_present_flag != 0 && field_pic_flag == 0 {
                delta_pic_order_cnt[1] = self.se_v()?;
            }
        }

        let redundant_pic_cnt = if redundant_pic_cnt_present_flag != 0 {
            self.ue_v()?
        } else {
            0
        };

        let mut direct_spatial_mv_pred_flag = 0;

        if SliceType::is(slice_type as u8, SliceType::B) {
            direct_spatial_mv_pred_flag = self.next_bit()?;
        }

        let mut num_ref_idx_active_override_flag = 0;
        let mut num_ref_idx_l0_active_minus1 = 0;
        let mut num_ref_idx_l1_active_minus1 = 0;

        if SliceType::is_any(
            slice_type as u8,
            &[SliceType::B, SliceType::P, SliceType::SP],
        ) {
            num_ref_idx_active_override_flag = self.next_bit()?;

            if num_ref_idx_active_override_flag != 0 {
                num_ref_idx_l0_active_minus1 = self.ue_v()?;

                if SliceType::is(slice_type as u8, SliceType::B) {
                    num_ref_idx_l1_active_minus1 = self.ue_v()?;
                }
            }
        }

        let mut ref_pic_list_modification = None;
        if nal_unit_type == 20 || nal_unit_type == 21 {
            self.ref_pic_list_mvc_modification();
        } else {
            ref_pic_list_modification = Some(self.ref_pic_list_modification(slice_type)?);
        }

        if (weighted_pred_flag != 0
            && SliceType::is_any(slice_type as u8, &[SliceType::P, SliceType::SP]))
            || (weighted_bipred_idc == 1 && SliceType::is(slice_type as u8, SliceType::B))
        {
            self.pred_weight_table();
        }

        let dec_ref_pic_marking = if self.nal_ref_idc() != 0 {
            Some(self.dec_ref_pic_marking()?)
        } else {
            None
        };

        let mut cabac_init_idc = 0;

        if entropy_coding_mode_flag != 0
            && !SliceType::is_any(slice_type as u8, &[SliceType::I, SliceType::SI])
        {
            cabac_init_idc = self.ue_v()?;
        }

        let slice_qp_delta = self.se_v()?;

        let mut sp_for_switch_flag = 0;
        let mut slice_qs_delta = 0;
        if SliceType::is_any(slice_type as u8, &[SliceType::SP, SliceType::SI]) {
            if SliceType::is(slice_type as u8, SliceType::SP) {
                sp_for_switch_flag = self.next_bit()?;
            }

            slice_qs_delta = self.se_v()?;
        }

        let mut disable_deblocking_filter_idc = 0;
        let mut slice_alpha_c0_offset_div2 = 0;
        let mut slice_beta_offset_div2 = 0;
        if deblocking_filter_control_present_flag != 0 {
            disable_deblocking_filter_idc = self.ue_v()?;

            if disable_deblocking_filter_idc != 0 {
                slice_alpha_c0_offset_div2 = self.se_v()?;
                slice_beta_offset_div2 = self.se_v()?;
            }
        }

        let mut slice_group_change_cycle = 0;
        if num_slice_groups_minus1 > 0 && slice_group_map_type >= 3 && slice_group_map_type <= 5 {
            todo!()
            // Ceil( Log2( PicSizeInMapUnits ÷ SliceGroupChangeRate + 1 ) )
            // slice_group_change_cycle = self.u_n()?;
        }

        Ok(SliceHeader {
            first_mb_in_slice,
            slice_type,
            pic_parameter_set_id,
            colour_plane_id,
            frame_num,
            field_pic_flag,
            bottom_field_flag,
            idr_pic_id,
            pic_order_cnt_lsb,
            delta_pic_order_cnt_bottom,
            delta_pic_order_cnt,
            redundant_pic_cnt,
            direct_spatial_mv_pred_flag,
            num_ref_idx_active_override_flag,
            num_ref_idx_l0_active_minus1,
            num_ref_idx_l1_active_minus1,
            ref_pic_list_modification,
            dec_ref_pic_marking,
            cabac_init_idc,
            slice_qp_delta,
            sp_for_switch_flag,
            slice_qs_delta,
            disable_deblocking_filter_idc,
            slice_alpha_c0_offset_div2,
            slice_beta_offset_div2,
            slice_group_change_cycle,
        })
    }

    fn ref_pic_list_mvc_modification(&mut self) {
        todo!()
    }

    fn ref_pic_list_modification(&mut self, slice_type: u8) -> NalUResult<RefPicListModification> {
        let mut ref_pic_list_modification_flag_l0 = 0;
        let mut rplm_l0 = Vec::new();
        if slice_type % 5 != 2 && slice_type % 5 != 4 {
            ref_pic_list_modification_flag_l0 = self.next_bit()?;

            if ref_pic_list_modification_flag_l0 != 0 {
                rplm_l0 = self.parse_rplm_list()?;
            }
        }

        let mut ref_pic_list_modification_flag_l1 = 0;
        let mut rplm_l1 = Vec::new();
        if slice_type % 5 == 1 {
            ref_pic_list_modification_flag_l1 = self.next_bit()?;

            if ref_pic_list_modification_flag_l1 != 0 {
                rplm_l1 = self.parse_rplm_list()?;
            }
        }

        Ok(RefPicListModification {
            ref_pic_list_modification_flag_l0,
            ref_pic_list_modification_flag_l1,
            rplm_l0,
            rplm_l1,
        })
    }

    fn parse_rplm_list(&mut self) -> NalUResult<Vec<Rplm>> {
        let mut rplms = Vec::new();
        loop {
            let modification_of_pic_nums_idc = self.ue_v()?;

            let mut abs_diff_pic_num_minus1 = 0;
            let mut long_term_pic_num = 0;

            match modification_of_pic_nums_idc {
                0 | 1 => abs_diff_pic_num_minus1 = self.ue_v::<i32>()?,
                2 => long_term_pic_num = self.ue_v::<u8>()?,
                _ => {}
            }

            rplms.push(Rplm {
                modification_of_pic_nums_idc,
                abs_diff_pic_num_minus1,
                long_term_pic_num,
            });

            if modification_of_pic_nums_idc == 3 {
                break;
            }
        }

        Ok(rplms)
    }

    fn pred_weight_table(&mut self) {
        todo!()
    }

    fn dec_ref_pic_marking(&mut self) -> NalUResult<DecRefPicMarking> {
        let mut no_output_of_prior_pics_flag = 0;
        let mut long_term_reference_flag = 0;
        let mut adaptive_ref_pic_marking_mode_flag = 0;
        let mut mmco = Vec::new();
        if self.idr_pic_flag() {
            no_output_of_prior_pics_flag = self.next_bit()?;
            long_term_reference_flag = self.next_bit()?;
        } else {
            adaptive_ref_pic_marking_mode_flag = self.next_bit()?;

            if adaptive_ref_pic_marking_mode_flag != 0 {
                loop {
                    let memory_management_control_operation = self.ue_v()?;

                    let difference_of_pic_nums_minus1 = if memory_management_control_operation == 1
                        || memory_management_control_operation == 3
                    {
                        self.ue_v()?
                    } else {
                        0
                    };

                    let long_term_pic_num = if memory_management_control_operation == 2 {
                        self.ue_v()?
                    } else {
                        0
                    };

                    let long_term_frame_idx = if memory_management_control_operation == 3
                        || memory_management_control_operation == 6
                    {
                        self.ue_v()?
                    } else {
                        0
                    };

                    let max_long_term_frame_idx_plus1 = if memory_management_control_operation == 4
                    {
                        self.ue_v()?
                    } else {
                        0
                    };

                    mmco.push(Mmco {
                        memory_management_control_operation,
                        difference_of_pic_nums_minus1,
                        long_term_pic_num,
                        long_term_frame_idx,
                        max_long_term_frame_idx_plus1,
                    });

                    if memory_management_control_operation == 0 {
                        break;
                    }
                }
            }
        }

        Ok(DecRefPicMarking {
            no_output_of_prior_pics_flag,
            long_term_reference_flag,
            adaptive_ref_pic_marking_mode_flag,
            mmco,
        })
    }

    fn slice_data(&mut self) {
        todo!()
    }

    fn rbsp_slice_trailing_bits(&mut self) {
        todo!()
    }
}

/// Sps
impl<'a, 'b> NalUParser<'a, 'b> {
    pub fn parse_sps(buffer: &'a [u8]) -> NalUResult<SeqParameterSetData> {
        std::fs::write("./garr", buffer).unwrap();
        let fake_sps = unsafe { MaybeUninit::uninit().assume_init() };

        let mut parser = NalUParser {
            buffer: BitSlice::from_slice(buffer),
            cursor: 0,
            sps: &fake_sps,
        };

        let sps = parser.seq_parameter_set_rbsp()?;

        unsafe {
            std::mem::transmute::<SeqParameterSetData, MaybeUninit<SeqParameterSetData>>(fake_sps);
        }

        Ok(sps)
    }

    fn seq_parameter_set_rbsp(&mut self) -> NalUResult<SeqParameterSetData> {
        // let len = self.read_u32();
        self.read_u8()?;
        // assert_eq!(self.read_u8()? & 0b0001_1111, 7);
        let data = self.seq_parameter_set_data()?;
        self.rbsp_trailing_bits()?;
        Ok(data)
    }

    fn seq_parameter_set_data(&mut self) -> NalUResult<SeqParameterSetData> {
        let profile_idc = self.read_u8()?;
        let constraint_set_flags = self.read_u8()? >> 2;
        let level_idc = self.read_u8()?;
        let seq_parameter_set_id = self.ue_v::<u8>()?;
        debug_assert!(seq_parameter_set_id <= 31);

        // assert_eq!(level_idc, 30);

        let mut chroma_format_idc = 1;
        let mut separate_colour_plane_flag = 0;
        let mut bit_depth_luma_minus8 = 0;
        let mut bit_depth_chroma_minus8 = 0;
        let mut qpprime_y_zero_transform_bypass_flag = 0;
        let mut seq_scaling_matrix_present_flag = 0;
        let mut seq_scaling_list_present_flag = [0; 12];
        if [100, 110, 122, 244, 44, 83, 86, 118, 128, 138, 139, 134, 135].contains(&profile_idc) {
            chroma_format_idc = self.ue_v()?;
            debug_assert!(bit_depth_luma_minus8 <= 3);

            if chroma_format_idc == 3 {
                separate_colour_plane_flag = self.next_bit()?
            }

            bit_depth_luma_minus8 = self.ue_v()?;
            debug_assert!(bit_depth_luma_minus8 <= 6);
            bit_depth_chroma_minus8 = self.ue_v()?;
            debug_assert!(bit_depth_chroma_minus8 <= 6);
            qpprime_y_zero_transform_bypass_flag = self.next_bit()?;
            seq_scaling_matrix_present_flag = self.next_bit()?;

            if seq_scaling_matrix_present_flag != 0 {
                let len = if chroma_format_idc != 3 { 8 } else { 12 };
                for i in 0..len {
                    seq_scaling_list_present_flag[i] = self.next_bit()?;

                    if seq_scaling_list_present_flag[i] != 0 {
                        if i < 6 {
                            todo!()
                            // todo: ignment of mnemonic names to scaling list indices and
                            // self.scaling_list(
                            //     ScalingList4x4[i],
                            //     16,
                            //     UseDefaultScalingMatrix4x4Flag[i]
                            // );
                        } else {
                            todo!()
                            // self.scaling_list(
                            //     ScalingList8x8[i − 6],
                            //     64,
                            //     UseDefaultScalingMatrix8x8Flag[i − 6]
                            // );
                        }
                    }
                }
            }
        }

        let log2_max_frame_num_minus4 = self.ue_v()?;
        debug_assert!(log2_max_frame_num_minus4 <= 12);
        let pic_order_cnt_type = self.ue_v()?;
        debug_assert!(pic_order_cnt_type <= 2);

        let mut log2_max_pic_order_cnt_lsb_minus4 = 0;
        let mut delta_pic_order_always_zero_flag = 0;
        let mut offset_for_non_ref_pic = 0;
        let mut offset_for_top_to_bottom_field = 0;
        let mut num_ref_frames_in_pic_order_cnt_cycle = 0;
        let mut offset_for_ref_frame = [0; 256];
        if pic_order_cnt_type == 0 {
            log2_max_pic_order_cnt_lsb_minus4 = self.ue_v()?;
            debug_assert!(log2_max_pic_order_cnt_lsb_minus4 <= 12);
        } else if pic_order_cnt_type == 1 {
            delta_pic_order_always_zero_flag = self.next_bit()?;
            offset_for_non_ref_pic = self.se_v()?;
            offset_for_top_to_bottom_field = self.se_v()?;
            num_ref_frames_in_pic_order_cnt_cycle = self.ue_v::<u8>()?;

            for i in 0..num_ref_frames_in_pic_order_cnt_cycle {
                offset_for_ref_frame[usize::from(i)] = self.se_v()?;
            }
        }

        let max_num_ref_frames = self.ue_v()?;
        let gaps_in_frame_num_value_allowed_flag = self.next_bit()?;
        let pic_width_in_mbs_minus1 = self.ue_v()?;
        let pic_height_in_map_units_minus1 = self.ue_v()?;
        let frame_mbs_only_flag = self.next_bit()?;

        let mb_adaptive_frame_field_flag = if frame_mbs_only_flag == 0 {
            self.next_bit()?
        } else {
            0
        };

        let direct_8x8_inference_flag = self.next_bit()?;
        if frame_mbs_only_flag == 0 {
            debug_assert!(direct_8x8_inference_flag == 1);
        }
        let frame_cropping_flag = self.next_bit()?;

        let mut frame_crop_left_offset = 0;
        let mut frame_crop_right_offset = 0;
        let mut frame_crop_top_offset = 0;
        let mut frame_crop_bottom_offset = 0;
        if frame_cropping_flag != 0 {
            frame_crop_left_offset = self.ue_v()?;
            frame_crop_right_offset = self.ue_v()?;
            frame_crop_top_offset = self.ue_v()?;
            frame_crop_bottom_offset = self.ue_v()?;
        }

        let vui_parameters_present_flag = self.next_bit()?;

        let vui = if vui_parameters_present_flag != 0 {
            Some(self.vui_parameters()?)
        } else {
            None
        };

        Ok(SeqParameterSetData {
            profile_idc,
            constraint_set_flags,
            // constraint_set0_flag,
            // constraint_set1_flag,
            // constraint_set2_flag,
            // constraint_set3_flag,
            // constraint_set4_flag,
            // constraint_set5_flag,
            // reserved_zero_2bits,
            level_idc,
            seq_parameter_set_id,
            chroma_format_idc,
            separate_colour_plane_flag,
            bit_depth_luma_minus8,
            bit_depth_chroma_minus8,
            qpprime_y_zero_transform_bypass_flag,
            seq_scaling_matrix_present_flag,
            seq_scaling_list_present_flag,
            scaling_list_4x4: [ScalingList; 6],
            scaling_list_8x8: [ScalingList; 6],
            log2_max_frame_num_minus4,
            pic_order_cnt_type,
            log2_max_pic_order_cnt_lsb_minus4,
            delta_pic_order_always_zero_flag,
            offset_for_non_ref_pic,
            offset_for_top_to_bottom_field,
            num_ref_frames_in_pic_order_cnt_cycle,
            offset_for_ref_frame,
            max_num_ref_frames,
            gaps_in_frame_num_value_allowed_flag,
            pic_width_in_mbs_minus1,
            pic_height_in_map_units_minus1,
            frame_mbs_only_flag,
            mb_adaptive_frame_field_flag,
            direct_8x8_inference_flag,
            frame_cropping_flag,
            frame_crop_left_offset,
            frame_crop_right_offset,
            frame_crop_top_offset,
            frame_crop_bottom_offset,
            vui_parameters_present_flag,
            vui,
        })
    }

    fn scaling_list(
        &mut self,
        scalingList: (),
        sizeOfScalingList: (),
        useDefaultScalingMatrixFlag: (),
    ) {
        todo!()
    }

    fn vui_parameters(&mut self) -> NalUResult<Box<Vui>> {
        let aspect_ratio_info_present_flag = self.next_bit()?;
        let mut aspect_ratio_idc = 0;
        let mut sar_width = 0;
        let mut sar_height = 0;
        if aspect_ratio_info_present_flag != 0 {
            aspect_ratio_idc = self.read_u8()?;
            // Extended_SAR
            if aspect_ratio_idc == 255 {
                sar_width = self.read_u16()?;
                sar_height = self.read_u16()?;
            }
        }
        let overscan_info_present_flag = self.next_bit()?;
        let overscan_appropriate_flag = if overscan_info_present_flag != 0 {
            self.next_bit()?
        } else {
            0
        };

        let video_signal_type_present_flag = self.next_bit()?;
        let mut video_format = 5;
        let mut video_full_range_flag = 0;
        let mut colour_description_present_flag = 0;
        let mut colour_primaries = 2;
        let mut transfer_characteristics = 0;
        let mut matrix_coefficients = 0;
        if video_signal_type_present_flag != 0 {
            video_format = self.u_n::<u8>(3);
            debug_assert!(video_format < 6, "format: {video_format}");
            video_full_range_flag = self.next_bit()?;
            colour_description_present_flag = self.next_bit()?;
            if colour_description_present_flag != 0 {
                colour_primaries = self.read_u8()?;
                debug_assert!(colour_primaries < 23 && !(13..=21).contains(&colour_primaries));
                transfer_characteristics = self.read_u8()?;
                debug_assert!(transfer_characteristics < 19 && transfer_characteristics != 3);
                matrix_coefficients = self.read_u8()?;
                debug_assert!(matrix_coefficients < 15 && matrix_coefficients != 3);
            }
        }

        let chroma_loc_info_present_flag = self.next_bit()?;
        let mut chroma_sample_loc_type_top_field = 0;
        let mut chroma_sample_loc_type_bottom_field = 0;
        if chroma_loc_info_present_flag != 0 {
            chroma_sample_loc_type_top_field = self.ue_v()?;
            chroma_sample_loc_type_bottom_field = self.ue_v()?;
        }

        let timing_info_present_flag = self.next_bit()?;
        let mut num_units_in_tick = 0;
        let mut time_scale = 0;
        let mut fixed_frame_rate_flag = 0;
        if timing_info_present_flag != 0 {
            num_units_in_tick = self.read_u32()?;
            debug_assert!(num_units_in_tick > 0);
            time_scale = self.read_u32()?;
            debug_assert!(time_scale > 0);
            fixed_frame_rate_flag = self.next_bit()?;
        }

        let nal_hrd_parameters_present_flag = self.next_bit()?;
        let nal_hrd_parameters = if nal_hrd_parameters_present_flag != 0 {
            Some(self.hrd_parameters()?)
        } else {
            None
        };

        let vcl_hrd_parameters_present_flag = self.next_bit()?;
        let vcl_hrd_parameters = if vcl_hrd_parameters_present_flag != 0 {
            Some(self.hrd_parameters()?)
        } else {
            None
        };

        let low_delay_hrd_flag =
            if nal_hrd_parameters_present_flag != 0 || vcl_hrd_parameters_present_flag != 0 {
                self.next_bit()?
            } else {
                0
            };

        let pic_struct_present_flag = self.next_bit()?;
        let bitstream_restriction_flag = self.next_bit()?;

        let mut motion_vectors_over_pic_boundaries_flag = 0;
        let mut max_bytes_per_pic_denom = 0;
        let mut max_bits_per_mb_denom = 0;
        let mut log2_max_mv_length_horizontal = 0;
        let mut log2_max_mv_length_vertical = 0;
        let mut max_num_reorder_frames = 0;
        let mut max_dec_frame_buffering = 0;
        if bitstream_restriction_flag != 0 {
            motion_vectors_over_pic_boundaries_flag = self.next_bit()?;
            max_bytes_per_pic_denom = self.ue_v()?;
            max_bits_per_mb_denom = self.ue_v()?;
            log2_max_mv_length_horizontal = self.ue_v()?;
            log2_max_mv_length_vertical = self.ue_v()?;
            max_num_reorder_frames = self.ue_v()?;
            max_dec_frame_buffering = self.ue_v()?;
        }

        Ok(Box::new(Vui {
            aspect_ratio_info_present_flag,
            aspect_ratio_idc,
            sar_width,
            sar_height,
            overscan_info_present_flag,
            overscan_appropriate_flag,
            video_signal_type_present_flag,
            video_format,
            video_full_range_flag,
            colour_description_present_flag,
            colour_primaries,
            transfer_characteristics,
            matrix_coefficients,
            chroma_loc_info_present_flag,
            chroma_sample_loc_type_top_field,
            chroma_sample_loc_type_bottom_field,
            timing_info_present_flag,
            num_units_in_tick,
            time_scale,
            fixed_frame_rate_flag,
            nal_hrd_parameters_present_flag,
            nal_hrd_parameters,
            vcl_hrd_parameters_present_flag,
            vcl_hrd_parameters,
            low_delay_hrd_flag,
            pic_struct_present_flag,
            bitstream_restriction_flag,
            motion_vectors_over_pic_boundaries_flag,
            max_bytes_per_pic_denom,
            max_bits_per_mb_denom,
            log2_max_mv_length_horizontal,
            log2_max_mv_length_vertical,
            max_num_reorder_frames,
            max_dec_frame_buffering,
        }))
    }

    fn hrd_parameters(&mut self) -> NalUResult<Hrd> {
        let cpb_cnt_minus1 = self.ue_v::<u32>()?;
        debug_assert!(cpb_cnt_minus1 < 32, "{cpb_cnt_minus1}");
        let bit_rate_scale = self.u_n::<u8>(4);
        let cpb_size_scale = self.u_n::<u8>(4);

        let mut bit_rate_value_minus1 = Vec::new();
        let mut cpb_size_value_minus1 = Vec::new();
        let mut cbr_flag = Vec::new();
        for _ in 0..cpb_cnt_minus1 {
            bit_rate_value_minus1.push(self.ue_v()?);
            cpb_size_value_minus1.push(self.ue_v()?);
            cbr_flag.push(self.next_bit()?);
        }

        let initial_cpb_removal_delay_length_minus1 = self.u_n::<u8>(5);
        let cpb_removal_delay_length_minus1 = self.u_n::<u8>(5);
        let dpb_output_delay_length_minus1 = self.u_n::<u8>(5);
        let time_offset_length = self.u_n::<u8>(5);

        Ok(Hrd {
            cpb_cnt_minus1: cpb_cnt_minus1 as u8,
            bit_rate_scale,
            cpb_size_scale,
            bit_rate_value_minus1,
            cpb_size_value_minus1,
            cbr_flag,
            initial_cpb_removal_delay_length_minus1,
            cpb_removal_delay_length_minus1,
            dpb_output_delay_length_minus1,
            time_offset_length,
        })
    }
}

/// Sei
impl<'a, 'b> NalUParser<'a, 'b> {
    fn parse_sei(&mut self) -> NalUResult<NalU> {
        let mut messages = Vec::new();

        while self.more_rbsp_data() {
            messages.push(self.sei_message()?);
        }

        self.rbsp_trailing_bits()?;

        Ok(NalU::Sei(messages))
    }

    fn sei_message(&mut self) -> NalUResult<SeiMessage> {
        let mut payload_type = 0;
        while self.peek_u8() == Some(0xff) {
            self.read_u8()?;
            payload_type += 255;
        }

        let last_payload_type_byte = self.read_u8()?;

        payload_type += last_payload_type_byte as u32;

        let mut payload_size = 0;

        while self.peek_u8() == Some(0xff) {
            self.read_u8()?;
            payload_size += 255;
        }

        let last_payload_size_byte = self.read_u8()?;

        payload_size += last_payload_size_byte as u32;

        let payload = self.sei_payload(payload_type, payload_size)?;

        self.rbsp_trailing_bits()?;

        Ok(payload)
    }

    fn sei_payload(&mut self, payload_type: u32, payload_size: u32) -> NalUResult<SeiMessage> {
        Ok(match payload_type {
            5 => SeiMessage::UserDataUnregistered(self.user_data_unregistered(payload_size)?),
            _ => todo!("sei_payload type not implemented: {:?}", payload_type),
        })
    }

    fn user_data_unregistered(&mut self, payload_size: u32) -> NalUResult<UserDataUnregistered> {
        let uuid = self.read_uuid()?;

        let payload = self.buffer[self.cursor..self.cursor + (payload_size - 16) as usize * 8]
            .to_bitvec()
            .into_vec();

        self.cursor += (payload_size - 16) as usize * 8;

        Ok(UserDataUnregistered { uuid, payload })
    }
}

impl<'a, 'b> NalUParser<'a, 'b> {
    fn nal_ref_idc(&self) -> u8 {
        (self.buffer[0..8].load_be::<u8>() & 0b0110_0000) >> 5
    }

    fn nal_unit_type(&self) -> u8 {
        self.buffer[0..8].load_be::<u8>() & 0b0001_1111
    }

    /// IdrPicFlag
    fn idr_pic_flag(&self) -> bool {
        self.nal_unit_type() == 5
    }
}

impl NalU {
    /// Assumes 001 or length prefix has been stripped and that escape bytes
    /// have been removed
    pub fn parse(buffer: &[u8]) -> Self {
        todo!()
    }
}

struct AvccNalUIterator<'a> {
    buffer: &'a [u8],
    cursor: usize,
}

impl<'a> AvccNalUIterator<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }
}

impl<'a> Iterator for AvccNalUIterator<'a> {
    type Item = Cow<'a, [u8]>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.buffer.len() < self.cursor + 4 {
            return None;
        }

        // todo: can also be u8 or u16
        let len = u32::from_be_bytes([
            self.buffer[self.cursor],
            self.buffer[self.cursor + 1],
            self.buffer[self.cursor + 2],
            self.buffer[self.cursor + 3],
        ]);

        self.cursor += 4;

        // todo: remove escape bytes
        let range = self
            .buffer
            .get(self.cursor..self.cursor + len as usize)
            .map(strip_emulation_prevention_three_byte);

        self.cursor += len as usize;

        range.map(Cow::Owned)
    }
}

struct AnnexBNalUIterator<'a> {
    buffer: &'a [u8],
    cursor: usize,
}

impl<'a> Iterator for AnnexBNalUIterator<'a> {
    type Item = Cow<'a, [u8]>;

    fn next(&mut self) -> Option<Self::Item> {
        assert_eq!(self.buffer[self.cursor], 0);
        assert_eq!(self.buffer[self.cursor + 1], 0);
        assert_eq!(self.buffer[self.cursor + 2], 1);

        // skip 001
        self.cursor += 3;

        let aho = aho_corasick::AhoCorasick::new(&[[0_u8, 0, 1]]);
        let next_unit_idx = match aho.earliest_find(&self.buffer[self.cursor..]) {
            Some(m) => m.start(),
            None => {
                return self
                    .buffer
                    .get(self.cursor..)
                    .map(strip_emulation_prevention_three_byte)
                    .map(Cow::Owned)
            }
        };

        let range = self
            .buffer
            .get(self.cursor..next_unit_idx)
            .map(strip_emulation_prevention_three_byte);

        self.cursor = next_unit_idx;

        range.map(Cow::Owned)
    }
}

fn main() -> io::Result<()> {
    let buffer =
        fs::File::open("Y2Mate.is - TRVE DATA demo-26hinlQTrys-360p-1658850169678.mp4").unwrap();
    let mut mp4 = Mp4::new(BufReader::new(buffer));
    let buffer =
        fs::File::open("Y2Mate.is - TRVE DATA demo-26hinlQTrys-360p-1658850169678.mp4").unwrap();

    // ftyp 
    mp4.skip_chunk()?;
    let mut moov = Moov::parse(&mut mp4)?;
    let mut mdat = Mdat::parse(&mut mp4)?;
    // let mut header = moov.movie_header(&mut mp4).parse(&mut mp4)?;
    let mut trak = moov.trak(&mut mp4)[0].parse(&mut mp4)?;
    let track_id = trak.track_header(&mut mp4).parse(&mut mp4)?.track_id;
    let mut mdia = trak.mdia(&mut mp4).parse(&mut mp4)?;
    let mut media_header = mdia.mdhd(&mut mp4).parse(&mut mp4)?;
    let mut minf = mdia.minf(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut dinf = minf.dinf(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut dref = dinf.data_reference(&mut mp4).parse(&mut mp4)?;
    let mut stbl = minf.stbl(&mut mp4).unwrap().parse(&mut mp4)?;

    let mut hdlr = mdia.hdlr(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stsd = stbl.sample_description(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stco = stbl.chunk_offset(&mut mp4).unwrap().parse(&mut mp4)?;
    // let mut ctts = stbl.composition_offset(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stts = stbl.time_to_sample(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stsc = stbl.sample_to_chunk(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stsz = stbl.sample_size(&mut mp4).unwrap().parse(&mut mp4)?;

    // dbg!(trak.edts(&mut mp4).unwrap().parse(&mut mp4)?);

    let sample_desc = match mp4.parse_sample_description(stsd.entries[0], *b"vide")? {
        SampleDescriptionTable::Video(v) => v,
        _ => todo!(),
    };

    let avcc = sample_desc.avcc().unwrap();

    let time = 0; // media_header.convert_to_media_time(Duration::from_secs(5)) as u32;
                  // let time = media_header.time_scale * 3;
    let sample_id = stts.lookup_time(time);
    let chunk_id = stsc.lookup_chunk(sample_id);
    let chunk_offset = stco.chunk_offset(chunk_id);
    let sample_len = stsz.sample_size(sample_id);

    let mut sample_offset = chunk_offset;

    // dbg!(chunk_offset, sample_len);
    // dbg!(stsz.number_of_entries);

    // dbg!(stsd.entries[0].parse_sample_description(&mut mp4, hdlr.component_subtype)?);

    // let chunks = stco.chunk_offset_table;

    // let mut buffer = vec![0; sample_len as usize];

    // mp4.jump_to(chunk_offset as u64)?;

    let mut their_mp4 = mp4::read_mp4(buffer).unwrap();

    let xxx = &their_mp4.read_sample(1, 1).unwrap().unwrap().bytes;

    let nal_u_iterator = AvccNalUIterator::new(xxx);

    let stripped_sps = strip_emulation_prevention_three_byte(&avcc.sequence_parameter_sets[0].nal_unit);

    let sps = NalUParser::parse_sps(&stripped_sps).unwrap();

    for packet in nal_u_iterator {
        NalUParser::new(&packet, &sps).parse().unwrap();
    }

    // let mut h264_in = Vec::new();
    // to_bitstream_with_001_be::<u32>(&xxx, &mut h264_in);
    // // std::fs::write("./foo", xxx).unwrap();

    // let mut decoder = Decoder::new().unwrap();

    // let mut b = vec![0, 0, 1];
    // b.extend(&avcc.sequence_parameter_sets[0].nal_unit);

    // dbg!(decoder.decode(&b));

    // for packet in nal_units(&h264_in) {
    //     //     std::fs::write("./bar", packet).unwrap();
    //     //     break;
    //     // On the first few frames this may fail, so you should check the result
    //     // a few packets before giving up.
    //     let maybe_some_yuv = match decoder.decode(packet) {
    //         Ok(o) => dbg!(o),
    //         e => {
    //             dbg!(e);
    //             None
    //         }
    //     };
    // }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::NalUParser;

    #[test]
    fn test_ue_v() {
        let sps = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
        let mut parser = NalUParser::new(&[0b00100110, 0b11011010, 0b11000100, 0b10010100], &sps);

        assert_eq!(parser.ue_v::<u8>(), Ok(3));
        assert_eq!(parser.ue_v::<u8>(), Ok(0));
        assert_eq!(parser.ue_v::<u8>(), Ok(0));
        assert_eq!(parser.ue_v::<u8>(), Ok(2));
        assert_eq!(parser.ue_v::<u8>(), Ok(2));
        assert_eq!(parser.ue_v::<u8>(), Ok(1));
        assert_eq!(parser.ue_v::<u8>(), Ok(0));
        assert_eq!(parser.ue_v::<u8>(), Ok(0));
        assert_eq!(parser.ue_v::<u8>(), Ok(8));
        assert_eq!(parser.ue_v::<u8>(), Ok(4));
    }
}
