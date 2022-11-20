use std::{
    fs,
    io::{self, BufReader},
};

use openh264::decoder::Decoder;
use openh264::nal_units;

use mp4_parser::{DataRef, Mdat, Moov, Mp4, Parse};

fn main() -> io::Result<()> {
    let buffer =
        fs::File::open("Y2Mate.is - TRVE DATA demo-26hinlQTrys-360p-1658850169678.mp4").unwrap();
    let mut mp4 = Mp4::new(BufReader::new(buffer));

    // let ftyp = Ftyp::parse(&mut mp4);
    // let header = mp4.peek_header()?;
    // dbg!(header);
    // dbg!(moov);
    mp4.skip_chunk()?;
    let mut moov = Moov::parse(&mut mp4)?;
    let mut mdat = Mdat::parse(&mut mp4)?;
    // let mut header = moov.movie_header(&mut mp4).parse(&mut mp4)?;
    let mut trak = moov.trak(&mut mp4)[0].parse(&mut mp4)?;
    let mut mdia = trak.mdia(&mut mp4).parse(&mut mp4)?;
    let mut minf = mdia.minf(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut dinf = minf.dinf(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut dref = dinf.data_reference(&mut mp4).parse(&mut mp4)?;
    let mut stbl = minf.stbl(&mut mp4).unwrap().parse(&mut mp4)?;

    let mut hdlr = mdia.hdlr(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stsd = stbl.sample_description(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut stco = stbl.chunk_offset(&mut mp4).unwrap().parse(&mut mp4)?;
    let mut ctts = stbl.composition_offset(&mut mp4).unwrap().parse(&mut mp4)?;

    dbg!(match dref.data[0] {
        DataRef::Url(u) => u.parse(&mut mp4)?,
        _ => panic!(),
    });
    // dbg!(stsd.entries[0].parse_sample_description(&mut mp4, hdlr.component_subtype)?);

    let h264_in = &mdat.bytes[84878..106928];

    let mut decoder = Decoder::new().unwrap();

    for packet in nal_units(h264_in) {
        // On the first few frames this may fail, so you should check the result
        // a few packets before giving up.
        let maybe_some_yuv = match decoder.decode(packet) {
            Ok(o) => dbg!(o),
            _ => None,
        };
        // dbg!(maybe_some_yuv.is_);
    }

    Ok(())
}
