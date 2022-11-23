#[derive(Debug, Clone)]
pub struct SliceHeader {
    pub first_mb_in_slice: u32,
    pub slice_type: u8,
    pub pic_parameter_set_id: u8,
    pub colour_plane_id: u8,
    pub frame_num: u16,
    pub field_pic_flag: u8,
    pub bottom_field_flag: u8,
    pub idr_pic_id: u16,
    pub pic_order_cnt_lsb: u16,
    pub delta_pic_order_cnt_bottom: i32,
    pub delta_pic_order_cnt: [i32; 2],
    pub redundant_pic_cnt: u8,
    pub direct_spatial_mv_pred_flag: u8,
    pub num_ref_idx_active_override_flag: u8,
    pub num_ref_idx_l0_active_minus1: u8,
    pub num_ref_idx_l1_active_minus1: u8,
    pub ref_pic_list_modification: Option<RefPicListModification>,
    pub dec_ref_pic_marking: Option<DecRefPicMarking>,
    pub cabac_init_idc: u8,
    pub slice_qp_delta: i8,
    pub sp_for_switch_flag: u8,
    pub slice_qs_delta: i8,
    pub disable_deblocking_filter_idc: u8,
    pub slice_alpha_c0_offset_div2: i8,
    pub slice_beta_offset_div2: i8,
    pub slice_group_change_cycle: u16,
}

#[derive(Debug, Clone)]
pub struct RefPicListModification {
    pub ref_pic_list_modification_flag_l0: u8,
    pub ref_pic_list_modification_flag_l1: u8,
    pub rplm_l0: Vec<Rplm>,
    pub rplm_l1: Vec<Rplm>,
}

#[derive(Debug, Clone)]
pub struct Rplm {
    pub modification_of_pic_nums_idc: u8,
    pub abs_diff_pic_num_minus1: i32,
    pub long_term_pic_num: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SliceType {
    P = 0,
    B = 1,
    I = 2,
    SP = 3,
    SI = 4,
}

impl SliceType {
    pub fn is(n: u8, slice_type: SliceType) -> bool {
        n % 5 == slice_type as u8
    }

    pub fn is_any(n: u8, slice_types: &[SliceType]) -> bool {
        slice_types.iter().any(|s| SliceType::is(n, *s))
    }
}

#[derive(Debug, Clone)]
pub struct DecRefPicMarking {
    pub no_output_of_prior_pics_flag: u8,
    pub long_term_reference_flag: u8,
    pub adaptive_ref_pic_marking_mode_flag: u8,
    pub mmco: Vec<Mmco>,
}

#[derive(Debug, Clone)]
pub struct Mmco {
    pub memory_management_control_operation: u8,
    pub difference_of_pic_nums_minus1: i32,
    pub long_term_pic_num: u8,
    pub long_term_frame_idx: u8,
    pub max_long_term_frame_idx_plus1: u8,
}
