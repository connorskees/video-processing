use atom_macro::{mp4_atom, mp4_container_atom};

use crate::{data_structures::Matrix, Fixed16, Fixed32, Header, Mp4, Reference};

use super::{InternalElement, Mdia, UnparsedAtom};

#[mp4_container_atom]
pub struct Trak {
    pub track_header: Reference<Tkhd>,
    pub tapt: Option<Reference<Tapt>>,
    pub matt: Option<Reference<Matt>>,
    pub edts: Option<Reference<Edts>>,
    pub tref: Option<Reference<Tref>>,
    pub txas: Option<Reference<Txas>>,
    pub load: Option<Reference<Load>>,
    pub imap: Option<Reference<Imap>>,
    pub mdia: Reference<Mdia>,
}

#[mp4_atom]
pub struct Tkhd {
    /// A 1-byte specification of the version of this track header.
    pub version: u8,

    /// Three bytes that are reserved for the track header flags. These flags
    /// indicate how the track is used in the movie. The following flags are
    /// valid (all flags are enabled when set to 1).
    ///
    /// Track enabled
    ///  - Indicates that the track is enabled. Flag value is 0x0001.
    ///
    /// Track in movie
    ///  - Indicates that the track is used in the movie. Flag value is 0x0002.
    ///
    /// Track in preview
    ///  - Indicates that the track is used in the movie’s preview. Flag value
    ///    is 0x0004.
    ///
    /// Track in poster
    ///  - Indicates that the track is used in the movie’s poster. Flag value is
    ///    0x0008.
    pub flags: [u8; 3],

    /// A 32-bit integer that indicates the calendar date and time (expressed in
    /// seconds since midnight, January 1, 1904) when the track header was created.
    /// It is strongly recommended that this value should be specified using
    /// coordinated universal time (UTC).
    pub creation_time: u32,

    /// A 32-bit integer that indicates the calendar date and time (expressed
    /// in seconds since midnight, January 1, 1904) when the track header was
    /// changed. It is strongly recommended that this value should be specified
    /// using coordinated universal time (UTC).
    pub modification_time: u32,

    /// A 32-bit integer that uniquely identifies the track. The value 0 cannot
    /// be used.
    pub track_id: u32,

    /// A 32-bit integer that is reserved for use by Apple. Set this field to 0.
    pub reserved: u32,

    /// A time value that indicates the duration of this track (in the movie’s
    /// time coordinate system). Note that this property is derived from the
    /// track’s edits. The value of this field is equal to the sum of the durations
    /// of all of the track’s edits. If there is no edit list, then the duration
    /// is the sum of the sample durations, converted into the movie timescale.
    pub duration: u32,

    /// An 8-byte value that is reserved for use by Apple. Set this field to 0.
    pub reserved_2: u64,

    /// A 16-bit integer that indicates this track’s spatial priority in its
    /// movie. The QuickTime Movie Toolbox uses this value to determine how
    /// tracks overlay one another. Tracks with lower layer values are displayed
    /// in front of tracks with higher layer values.
    pub layer: u16,

    /// A 16-bit integer that identifies a collection of movie tracks that contain
    /// alternate data for one another. This same identifier appears in each
    /// 'tkhd' atom of the other tracks in the group. QuickTime chooses one track
    /// from the group to be used when the movie is played. The choice may be
    /// based on such considerations as playback quality, language, or the
    /// capabilities of the computer.
    ///
    /// A value of zero indicates that the track is not in an alternate track
    /// group.
    ///
    /// The most common reason for having alternate tracks is to provide versions
    /// of the same track in different languages. Figure 2-8 shows an example
    /// of several tracks. The video track’s Alternate Group ID is 0, which
    /// means that it is not in an alternate group (and its language codes are
    /// empty; normally, video tracks should have the appropriate language
    /// tags). The three sound tracks have the same Group ID, so they form one
    /// alternate group, and the subtitle tracks have a different Group ID, so
    /// they form another alternate group. The tracks would not be adjacent in
    /// an actual QuickTime file; this is just a list of example track field
    /// values.
    pub alternate_group: u16,

    /// A 16-bit fixed-point value that indicates how loudly this track’s sound
    /// is to be played. A value of 1.0 indicates normal volume.
    pub volume: Fixed16,

    /// A 16-bit integer that is reserved for use by Apple. Set this field to 0.
    pub reserved_3: u16,

    /// The matrix structure associated with this track.
    pub matrix: Matrix,

    /// A 32-bit fixed-point number that specifies the width of this track in pixels
    pub track_width: Fixed32,

    /// A 32-bit fixed-point number that specifies the height of this track in pixels
    pub track_height: Fixed32,
}

#[mp4_atom]
pub struct Tapt {}
#[mp4_atom]
pub struct Matt {}
#[mp4_atom]
pub struct Edts {}
#[mp4_atom]
pub struct Tref {}
#[mp4_atom]
pub struct Txas {}
#[mp4_atom]
pub struct Load {}
#[mp4_atom]
pub struct Imap {}
