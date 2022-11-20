use std::fmt;

use super::*;

#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Header(pub(crate) [u8; 4]);

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match String::from_utf8(self.0.to_vec()) {
            Ok(s) => write!(f, "{}", s),
            Err(..) => write!(f, "{:?}", self.0),
        }
    }
}

/// File type compatibility—identifies the file type and differentiates it from
/// similar file types, such as MPEG-4 files and JPEG-2000 files
pub(crate) const FTYP: Header = Header(*b"ftyp");

/// Movie resource metadata about the movie (number and type of tracks, location
/// of sample data, and so on). Describes where the movie data can be found and
/// how to interpret it.
pub(crate) const MOOV: Header = Header(*b"moov");

/// Movie sample data—media samples such as video frames and groups of audio
/// samples. Usually this data can be interpreted only by using the movie resource.
pub(crate) const MDAT: Header = Header(*b"mdat");

/// The track header atom contains the track characteristics for the track,
/// including temporal, spatial, and volume information.
pub(crate) const TKHD: Header = Header(*b"tkhd");

/// The data contained in this atom defines characteristics of the entire movie,
/// such as time scale and duration
pub(crate) const MVHD: Header = Header(*b"mvhd");

/// Chunk offset atoms identify the location of each chunk of data in the media’s
/// data stream
pub(crate) const STCO: Header = Header(*b"stco");

/// 64-bit chunk offset atom. When this atom appears, it is used in place of the
/// original chunk offset atom, which can contain only 32-bit offsets
pub(crate) const CO64: Header = Header(*b"co64");

pub(crate) const CLIP: Header = Header(*b"clip");
pub(crate) const TRAK: Header = Header(*b"trak");
pub(crate) const UDTA: Header = Header(*b"udta");
pub(crate) const CTAB: Header = Header(*b"ctab");
pub(crate) const CMOV: Header = Header(*b"cmov");
pub(crate) const RMRA: Header = Header(*b"rmra");
pub(crate) const PRFL: Header = Header(*b"prfl");
pub(crate) const TAPT: Header = Header(*b"tapt");
pub(crate) const MATT: Header = Header(*b"matt");
pub(crate) const EDTS: Header = Header(*b"edts");
pub(crate) const TREF: Header = Header(*b"tref");
pub(crate) const TXAS: Header = Header(*b"txas");
pub(crate) const LOAD: Header = Header(*b"load");
pub(crate) const IMAP: Header = Header(*b"imap");
pub(crate) const MDIA: Header = Header(*b"mdia");
pub(crate) const MDHD: Header = Header(*b"mdhd");
pub(crate) const ELNG: Header = Header(*b"elng");
pub(crate) const HDLR: Header = Header(*b"hdlr");
pub(crate) const MINF: Header = Header(*b"minf");
pub(crate) const VMHD: Header = Header(*b"vmhd");
pub(crate) const SMHD: Header = Header(*b"smhd");
pub(crate) const GMHD: Header = Header(*b"gmhd");
pub(crate) const DINF: Header = Header(*b"dinf");
pub(crate) const STBL: Header = Header(*b"stbl");
pub(crate) const GMIN: Header = Header(*b"gmin");
pub(crate) const TEXT: Header = Header(*b"text");
pub(crate) const DREF: Header = Header(*b"dref");
pub(crate) const STSD: Header = Header(*b"stsd");
pub(crate) const STTS: Header = Header(*b"stts");
pub(crate) const CTTS: Header = Header(*b"ctts");
pub(crate) const CSLG: Header = Header(*b"cslg");
pub(crate) const STSS: Header = Header(*b"stss");
pub(crate) const STPS: Header = Header(*b"stps");
pub(crate) const STSC: Header = Header(*b"stsc");
pub(crate) const STSZ: Header = Header(*b"stsz");
pub(crate) const STSH: Header = Header(*b"stsh");
pub(crate) const SGPD: Header = Header(*b"sgpd");
pub(crate) const SBGP: Header = Header(*b"sbgp");
pub(crate) const SDTP: Header = Header(*b"sdtp");
pub(crate) const RMDA: Header = Header(*b"rmda");
pub(crate) const RDRF: Header = Header(*b"rdrf");
pub(crate) const RMDR: Header = Header(*b"rmdr");
pub(crate) const RMCS: Header = Header(*b"rmcs");
pub(crate) const RMVC: Header = Header(*b"rmvc");
pub(crate) const RMCD: Header = Header(*b"rmcd");
pub(crate) const RMQU: Header = Header(*b"rmqu");
pub(crate) const CRGN: Header = Header(*b"crgn");
pub(crate) const KMAT: Header = Header(*b"kmat");
pub(crate) const ELST: Header = Header(*b"elst");
pub(crate) const CLEF: Header = Header(*b"clef");
pub(crate) const PROF: Header = Header(*b"prof");
pub(crate) const ENOF: Header = Header(*b"enof");
pub(crate) const SSRC: Header = Header(*b"ssrc");
pub(crate) const OBID: Header = Header(*b"obid");
pub(crate) const ALIS: Header = Header(*b"alis");
pub(crate) const RSRC: Header = Header(*b"rsrc");
pub(crate) const URL: Header = Header(*b"url ");

macro_rules! set_header {
    ($s:ty, $header:ident) => {
        impl $s {
            pub const HEADER: Header = $header;
        }
    };
}

set_header!(Co64, CO64);
set_header!(Mdat, MDAT);
set_header!(Stco, STCO);
set_header!(Mvhd, MVHD);
set_header!(Tkhd, TKHD);
set_header!(Ftyp, FTYP);
set_header!(Moov, MOOV);
set_header!(Clip, CLIP);
set_header!(Trak, TRAK);
set_header!(Udta, UDTA);
set_header!(Ctab, CTAB);
set_header!(Cmov, CMOV);
set_header!(Rmra, RMRA);
set_header!(Gmin, GMIN);
set_header!(Text, TEXT);
set_header!(Dref, DREF);
set_header!(Stsd, STSD);
set_header!(Stts, STTS);
set_header!(Ctts, CTTS);
set_header!(Cslg, CSLG);
set_header!(Stss, STSS);
set_header!(Stps, STPS);
set_header!(Stsc, STSC);
set_header!(Stsz, STSZ);
set_header!(Stsh, STSH);
set_header!(Sgpd, SGPD);
set_header!(Sbgp, SBGP);
set_header!(Sdtp, SDTP);
set_header!(Rmda, RMDA);
set_header!(Rdrf, RDRF);
set_header!(Rmdr, RMDR);
set_header!(Rmcs, RMCS);
set_header!(Rmvc, RMVC);
set_header!(Rmcd, RMCD);
set_header!(Rmqu, RMQU);
set_header!(Crgn, CRGN);
set_header!(Kmat, KMAT);
set_header!(Elst, ELST);
set_header!(Clef, CLEF);
set_header!(Prof, PROF);
set_header!(Enof, ENOF);
set_header!(Ssrc, SSRC);
set_header!(Obid, OBID);
set_header!(Prfl, PRFL);
set_header!(Tapt, TAPT);
set_header!(Matt, MATT);
set_header!(Edts, EDTS);
set_header!(Tref, TREF);
set_header!(Txas, TXAS);
set_header!(Load, LOAD);
set_header!(Imap, IMAP);
set_header!(Mdia, MDIA);
set_header!(Mdhd, MDHD);
set_header!(Elng, ELNG);
set_header!(Hdlr, HDLR);
set_header!(Minf, MINF);
set_header!(Vmhd, VMHD);
set_header!(Smhd, SMHD);
set_header!(Gmhd, GMHD);
set_header!(Dinf, DINF);
set_header!(Stbl, STBL);
set_header!(Alis, ALIS);
set_header!(Rsrc, RSRC);
set_header!(Url, URL);
