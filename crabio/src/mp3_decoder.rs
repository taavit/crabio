use crate::utils::{bit_stream_cache::BitStreamInfo, clip_to_short::clip_to_short};

pub const CHANNEL_MONO: usize = 0;
pub const CHANNEL_LEFT: usize = 0;
pub const CHANNEL_RIGHT: usize = 1;

pub const SYNCWORDH: u8 = 0xff;
pub const SYNCWORDL: u8 = 0xf0;
pub const DQ_FRACBITS_OUT: u8 = 25; // number of fraction bits in output of dequant
pub const CSHIFT: u8 = 12; // coefficients have 12 leading sign bits for early-terminating mulitplies
pub const SIBYTES_MPEG1_MONO: usize = 17;
pub const SIBYTES_MPEG1_STEREO: usize = 32;
pub const SIBYTES_MPEG2_MONO: usize = 9;
pub const SIBYTES_MPEG2_STEREO: usize = 17;
pub const IMDCT_SCALE: u8 = 2; // additional scaling (by sqrt(2)) for fast IMDCT36
pub const NGRANS_MPEG1: u8 = 2;
pub const NGRANS_MPEG2: u8 = 1;
pub const SQRTHALF: u32 = 0x5a82799a; // sqrt(0.5) in Q31 format

const C3_0: i32 = 0x6ed9eba1; /* format = Q31, cos(pi/6) */
const C6: [i32; 3] = [0x7ba3751d, 0x5a82799a, 0x2120fb83]; /* format = Q31, cos(((0:2) + 0.5) * (pi/6)) */

const C9_0: i32 = 0x6ed9eba1;
const C9_1: i32 = 0x620dbe8b;
const C9_2: i32 = 0x163a1a7e;
const C9_3: i32 = 0x5246dd49;
const C9_4: i32 = 0x7e0e2e32;

pub const POLY_COEF: [u32; 264] = [
    /* shuffled vs. original from 0, 1, ... 15 to 0, 15, 2, 13, ... 14, 1 */
    0x00000000, 0x00000074, 0x00000354, 0x0000072c, 0x00001fd4, 0x00005084, 0x000066b8, 0x000249c4,
    0x00049478, 0xfffdb63c, 0x000066b8, 0xffffaf7c, 0x00001fd4, 0xfffff8d4, 0x00000354, 0xffffff8c,
    0xfffffffc, 0x00000068, 0x00000368, 0x00000644, 0x00001f40, 0x00004ad0, 0x00005d1c, 0x00022ce0,
    0x000493c0, 0xfffd9960, 0x00006f78, 0xffffa9cc, 0x0000203c, 0xfffff7e4, 0x00000340, 0xffffff84,
    0xfffffffc, 0x00000060, 0x00000378, 0x0000056c, 0x00001e80, 0x00004524, 0x000052a0, 0x00020ffc,
    0x000491a0, 0xfffd7ca0, 0x00007760, 0xffffa424, 0x00002080, 0xfffff6ec, 0x00000328, 0xffffff74,
    0xfffffffc, 0x00000054, 0x00000384, 0x00000498, 0x00001d94, 0x00003f7c, 0x00004744, 0x0001f32c,
    0x00048e18, 0xfffd6008, 0x00007e70, 0xffff9e8c, 0x0000209c, 0xfffff5ec, 0x00000310, 0xffffff68,
    0xfffffffc, 0x0000004c, 0x0000038c, 0x000003d0, 0x00001c78, 0x000039e4, 0x00003b00, 0x0001d680,
    0x00048924, 0xfffd43ac, 0x000084b0, 0xffff990c, 0x00002094, 0xfffff4e4, 0x000002f8, 0xffffff5c,
    0xfffffffc, 0x00000044, 0x00000390, 0x00000314, 0x00001b2c, 0x0000345c, 0x00002ddc, 0x0001ba04,
    0x000482d0, 0xfffd279c, 0x00008a20, 0xffff93a4, 0x0000206c, 0xfffff3d4, 0x000002dc, 0xffffff4c,
    0xfffffffc, 0x00000040, 0x00000390, 0x00000264, 0x000019b0, 0x00002ef0, 0x00001fd4, 0x00019dc8,
    0x00047b1c, 0xfffd0be8, 0x00008ecc, 0xffff8e64, 0x00002024, 0xfffff2c0, 0x000002c0, 0xffffff3c,
    0xfffffff8, 0x00000038, 0x0000038c, 0x000001bc, 0x000017fc, 0x0000299c, 0x000010e8, 0x000181d8,
    0x0004720c, 0xfffcf09c, 0x000092b4, 0xffff894c, 0x00001fc0, 0xfffff1a4, 0x000002a4, 0xffffff2c,
    0xfffffff8, 0x00000034, 0x00000380, 0x00000120, 0x00001618, 0x00002468, 0x00000118, 0x00016644,
    0x000467a4, 0xfffcd5cc, 0x000095e0, 0xffff8468, 0x00001f44, 0xfffff084, 0x00000284, 0xffffff18,
    0xfffffff8, 0x0000002c, 0x00000374, 0x00000090, 0x00001400, 0x00001f58, 0xfffff068, 0x00014b14,
    0x00045bf0, 0xfffcbb88, 0x00009858, 0xffff7fbc, 0x00001ea8, 0xffffef60, 0x00000268, 0xffffff04,
    0xfffffff8, 0x00000028, 0x0000035c, 0x00000008, 0x000011ac, 0x00001a70, 0xffffded8, 0x00013058,
    0x00044ef8, 0xfffca1d8, 0x00009a1c, 0xffff7b54, 0x00001dfc, 0xffffee3c, 0x0000024c, 0xfffffef0,
    0xfffffff4, 0x00000024, 0x00000340, 0xffffff8c, 0x00000f28, 0x000015b0, 0xffffcc70, 0x0001161c,
    0x000440bc, 0xfffc88d8, 0x00009b3c, 0xffff7734, 0x00001d38, 0xffffed18, 0x0000022c, 0xfffffedc,
    0xfffffff4, 0x00000020, 0x00000320, 0xffffff1c, 0x00000c68, 0x0000111c, 0xffffb92c, 0x0000fc6c,
    0x00043150, 0xfffc708c, 0x00009bb8, 0xffff7368, 0x00001c64, 0xffffebf4, 0x00000210, 0xfffffec4,
    0xfffffff0, 0x0000001c, 0x000002f4, 0xfffffeb4, 0x00000974, 0x00000cb8, 0xffffa518, 0x0000e350,
    0x000420b4, 0xfffc5908, 0x00009b9c, 0xffff6ff4, 0x00001b7c, 0xffffead0, 0x000001f4, 0xfffffeac,
    0xfffffff0, 0x0000001c, 0x000002c4, 0xfffffe58, 0x00000648, 0x00000884, 0xffff9038, 0x0000cad0,
    0x00040ef8, 0xfffc425c, 0x00009af0, 0xffff6ce0, 0x00001a88, 0xffffe9b0, 0x000001d4, 0xfffffe94,
    0xffffffec, 0x00000018, 0x0000028c, 0xfffffe04, 0x000002e4, 0x00000480, 0xffff7a90, 0x0000b2fc,
    0x0003fc28, 0xfffc2c90, 0x000099b8, 0xffff6a3c, 0x00001988, 0xffffe898, 0x000001bc, 0xfffffe7c,
    0x000001a0, 0x0000187c, 0x000097fc, 0x0003e84c, 0xffff6424, 0xffffff4c, 0x00000248, 0xffffffec,
];

#[inline(always)]
pub fn clip_2n(y: i32, n: u32) -> i32 {
    if n >= 31 {
        // No clipping possible or needed in 32-bit signed range
        // This case is very rare (only when no guard-bit scaling was applied)
        y
    } else {
        // n is 0..=30 → 1 << n is safe (positive, no sign-bit issue)
        let bound = 1i32 << n; // normal shift is fine and preferred here
        y.min(bound - 1).max(-bound)
    }
}

#[inline]
pub const fn sar_64(x: u64, n: i32) -> u64 {
    x >> n
}

#[inline(always)]
pub const fn mulshift_32(x: i32, y: i32) -> i32 {
    ((x as i64) * (y as i64) >> 32) as i32
}

#[inline(always)]
pub const fn madd_64(sum64: u64, x: i32, y: i32) -> u64 {
    let prod = (x as i64).wrapping_mul(y as i64);
    sum64.wrapping_add(prod as u64)
} /* returns 64-bit value in [edx:eax] */

#[repr(usize)]
#[derive(Debug, PartialEq, Eq, Default, Clone, Copy)]
pub enum BlockType {
    #[default]
    Normal = 0, // Długie okno (Long)
    Start = 1, // Okno przejściowe (Start)
    Short = 2, // Trzy krótkie okna (Short) - TO JEST TWÓJ "SPECIAL CASE"
    Stop = 3,  // Okno kończące (Stop)
}

/// 12-point IMDCT for MP3 short blocks (fixed-point)
///
/// Input:  exactly 18 coefficients (only indices 0,3,6,9,12,15 used)
/// Output: exactly 6 time-domain samples
#[inline(always)]
pub fn imdct_12(x: &[i32; 16], out: &mut [i32; 6]) {
    let x0 = x[0];
    let x1 = x[3];
    let x2 = x[6];
    let x3 = x[9];
    let x4 = x[12];
    let x5 = x[15];

    // Pre-butterfly
    let t4 = x4 - x5;
    let mut t3 = x3 - t4;
    let t2 = x2 - t3;
    t3 -= x5;
    let mut t1 = x1 - t2;
    let t0 = x0 - t1;
    t1 -= t3;

    let t0 = t0 >> 1;
    let t1 = t1 >> 1;

    // Even part
    let a0 = mulshift_32(C3_0, t2) << 1;
    let a1 = t0 + (t4 >> 1);
    let a2 = t0 - t4;

    let even0 = a1 + a0;
    let even2 = a2;
    let even4 = a1 - a0;

    // Odd part
    let a0 = mulshift_32(C3_0, t3) << 1;
    let a1 = t1 + (x5 >> 1); // note: x5, not t5 — original uses original x5
    let a2 = t1 - x5;

    let odd1 = mulshift_32(C6[0], a1 + a0) << 2;
    let odd3 = mulshift_32(C6[1], a2) << 2;
    let odd5 = mulshift_32(C6[2], a1 - a0) << 2;

    // Output
    out[0] = even0 + odd1;
    out[1] = even2 + odd3;
    out[2] = even4 + odd5;
    out[3] = even4 - odd5;
    out[4] = even2 - odd3;
    out[5] = even0 - odd1;
}

#[inline(always)]
pub fn idct_9(x: &mut [i32; 9]) {
    let x0 = x[0];
    let x1 = x[1];
    let x2 = x[2];
    let x3 = x[3];
    let x4 = x[4];
    let x5 = x[5];
    let x6 = x[6];
    let x7 = x[7];
    let x8 = x[8];

    // Stage 1: differences and sums
    let a1 = x0 - x6;
    let a2 = x1 - x5;
    let a3 = x1 + x5;
    let a4 = x2 - x4;
    let a5 = x2 + x4;
    let a6 = x2 + x8;
    let a7 = x1 + x7;

    let a8 = a6 - a5; // x8 - x4
    let a9 = a3 - a7; // x5 - x7
    let a10 = a2 - x7; // x1 - x5 - x7
    let a11 = a4 - x8; // x2 - x4 - x8

    // Multiplies with precomputed constants
    let m1 = mulshift_32(C9_0, x3);
    let m3 = mulshift_32(C9_0, a10);
    let m5 = mulshift_32(C9_1, a5);
    let m6 = mulshift_32(C9_2, a6);
    let m7 = mulshift_32(C9_1, a8);
    let m8 = mulshift_32(C9_2, a5);
    let m9 = mulshift_32(C9_3, a9);
    let m10 = mulshift_32(C9_4, a7);
    let m11 = mulshift_32(C9_3, a3);
    let m12 = mulshift_32(C9_4, a9);

    // Stage 2: intermediate sums
    let a12 = x0 + (x6 >> 1);
    let a13 = a12 + (m1 << 1);
    let a14 = a12 - (m1 << 1);
    let a15 = a1 + (a11 >> 1);
    let a16 = (m5 << 1) + (m6 << 1);
    let a17 = (m7 << 1) - (m8 << 1);
    let a18 = a16 + a17;
    let a19 = (m9 << 1) + (m10 << 1);
    let a20 = (m11 << 1) - (m12 << 1);

    let a21 = a20 - a19;
    let a22 = a13 + a16;
    let a23 = a14 + a16;
    let a24 = a14 + a17;
    let a25 = a13 + a17;
    let a26 = a14 - a18;
    let a27 = a13 - a18;

    // Final output (in-place)
    x[0] = a22 + a19;
    x[1] = a15 + (m3 << 1);
    x[2] = a24 + a20;
    x[3] = a26 - a21;
    x[4] = a1 - a11;
    x[5] = a27 + a21;
    x[6] = a25 - a20;
    x[7] = a15 - (m3 << 1);
    x[8] = a23 - a19;
}

///
///  P O L Y P H A S E
///

pub const HUFF_PAIRTABS: usize = 32;
pub const BLOCK_SIZE: usize = 18;
pub const NBANDS: usize = 32;
pub const MAX_REORDER_SAMPS: usize = (192 - 126) * 3; // largest critical band for short blocks (see sfBandTable)
pub const VBUF_LENGTH: usize = 17 * 2 * NBANDS; // for double-sized vbuf FIFO
pub const MAX_SCFBD: usize = 4; // max scalefactor bands per channel
pub const MAINBUF_SIZE: usize = 1940;
pub const MAX_NGRAN: usize = 2; // max granules
pub const MAX_NCHAN: usize = 2; // max channels
pub const MAX_NSAMP: usize = 576; // max samples per channel, per granule

pub const ERR_MP3_NONE: i8 = 0;
pub const ERR_MP3_INDATA_UNDERFLOW: i8 = -1;
pub const ERR_MP3_MAINDATA_UNDERFLOW: i8 = -2;
pub const ERR_MP3_FREE_BITRATE_SYNC: i8 = -3;
pub const ERR_MP3_OUT_OF_MEMORY: i8 = -4;
pub const ERR_MP3_NULL_POINTER: i8 = -5;
pub const ERR_MP3_INVALID_FRAMEHEADER: i8 = -6;
pub const ERR_MP3_INVALID_SIDEINFO: i8 = -7;
pub const ERR_MP3_INVALID_SCALEFACT: i8 = -8;
pub const ERR_MP3_INVALID_HUFFCODES: i8 = -9;
pub const ERR_MP3_INVALID_DEQUANTIZE: i8 = -10;
pub const ERR_MP3_INVALID_IMDCT: i8 = -11;
pub const ERR_MP3_INVALID_SUBBAND: i8 = -12;
pub const ERR_UNKNOWN: i8 = -127;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(usize)]
pub enum SampleRateIndex {
    #[default]
    SampleRate0 = 0,
    SampleRate1 = 1,
    SampleRate2 = 2,
}

impl SampleRateIndex {
    pub const fn from_u8(v: u8) -> Result<Self, ()> {
        match v {
            0 => Ok(SampleRateIndex::SampleRate0),
            1 => Ok(SampleRateIndex::SampleRate1),
            2 => Ok(SampleRateIndex::SampleRate2),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(usize)]
pub enum LayerIndex {
    #[default]
    Layer1 = 1,
    Layer2 = 2,
    Layer3 = 3,
}

impl LayerIndex {
    pub const fn from_u8(v: u8) -> Result<Self, ()> {
        match v {
            1 => Ok(LayerIndex::Layer1),
            2 => Ok(LayerIndex::Layer2),
            3 => Ok(LayerIndex::Layer3),
            _ => Err(())
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(usize)]
pub enum BitrateIndex {
    #[default]
    Bitrate0 = 0,
    Bitrate1 = 1,
    Bitrate2 = 2,
    Bitrate3 = 3,
    Bitrate4 = 4,
    Bitrate5 = 5,
    Bitrate6 = 6,
    Bitrate7 = 7,
    Bitrate8 = 8,
    Bitrate9 = 9,
    Bitrate10 = 10,
    Bitrate11 = 11,
    Bitrate12 = 12,
    Bitrate13 = 13,
    Bitrate14 = 14,
}

impl BitrateIndex {
    pub const fn from_u8(v: u8) -> Result<Self, ()> {
        match v {
            0 => Ok(BitrateIndex::Bitrate0),
            1 => Ok(BitrateIndex::Bitrate1),
            2 => Ok(BitrateIndex::Bitrate2),
            3 => Ok(BitrateIndex::Bitrate3),
            4 => Ok(BitrateIndex::Bitrate4),
            5 => Ok(BitrateIndex::Bitrate5),
            6 => Ok(BitrateIndex::Bitrate6),
            7 => Ok(BitrateIndex::Bitrate7),
            8 => Ok(BitrateIndex::Bitrate8),
            9 => Ok(BitrateIndex::Bitrate9),
            10 => Ok(BitrateIndex::Bitrate10),
            11 => Ok(BitrateIndex::Bitrate11),
            12 => Ok(BitrateIndex::Bitrate12),
            13 => Ok(BitrateIndex::Bitrate13),
            14 => Ok(BitrateIndex::Bitrate14),
            _ => Err(())
        }
    }
}


#[repr(C)]
#[allow(non_snake_case)]
#[derive(Default)]
pub struct FrameHeader {
    pub layer: LayerIndex,      /* layer index (1, 2, or 3) */
    pub crc: i32,        /* CRC flag: 0 = disabled, 1 = enabled */
    pub br_idx: BitrateIndex,      /* bitrate index (0 - 15) */
    pub sr_idx: SampleRateIndex,      /* sample rate index (0 - 2) */
    pub paddingBit: i32, /* padding flag: 0 = no padding, 1 = single pad byte */
    pub privateBit: i32, /* unused */
    pub modeExt: usize,  /* used to decipher joint stereo mode */
    pub copyFlag: i32,   /* copyright flag: 0 = no, 1 = yes */
    pub origFlag: i32,   /* original flag: 0 = copy, 1 = original */
    pub emphasis: i32,   /* deemphasis mode */
    pub CRCWord: i32,    /* CRC word (16 bits, 0 if crc not enabled) */
}

impl FrameHeader {
    pub fn check_pad_bit(&self) -> i32 {
        if self.paddingBit != 0 { 1 } else { 0 }
    }
}

#[repr(C)]
#[derive(Debug, Default, Clone, Copy)]
pub struct SideInfoSub {
    pub part23_length: i32,     /* number of bits in main data */
    pub n_bigvals: i32, /* 2x this = first set of Huffman cw's (maximum amplitude can be > 1) */
    pub global_gain: i32, /* overall gain for dequantizer */
    pub sf_compress: i32, /* unpacked to figure out number of bits in scale factors */
    pub win_switch_flag: i32, /* window switching flag */
    pub blockType: BlockType, /* block type */
    pub mixedBlock: i32, /* 0 = regular block (all short or long), 1 = mixed block */
    pub tableSelect: [i32; 3], /* index of Huffman tables for the big values regions */
    pub subBlockGain: [i32; 3], /* subblock gain offset, relative to global gain */
    pub region0Count: i32, /* 1+region0Count = num scale factor bands in first region of bigvals */
    pub region1Count: i32, /* 1+region1Count = num scale factor bands in second region of bigvals */
    pub preFlag: i32,   /* for optional high frequency boost */
    pub sfactScale: i32, /* scaling of the scalefactors */
    pub count1TableSelect: i32, /* index of Huffman table for quad codewords */
}

#[repr(C)]
pub struct SideInfo {
    pub main_data_begin: i32,
    pub private_bits: i32,
    pub scfsi: [[i32; MAX_SCFBD]; MAX_NCHAN], /* 4 scalefactor bands per channel */
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CriticalBandInfo {
    pub cbType: i32,      /* pure long = 0, pure short = 1, mixed = 2 */
    pub cbEndS: [i32; 3], /* number nonzero short cb's, per subbblock */
    pub cbEndSMax: i32,   /* max of cbEndS[] */
    pub cbEndL: i32,      /* number nonzero long cb's  */
}

#[repr(C)]
pub struct DequantInfo {
    pub work_buf: [i32; MAX_REORDER_SAMPS], /* workbuf for reordering short blocks */
}

#[repr(C)]
pub struct HuffmanInfo {
    pub huff_dec_buf: [[i32; MAX_NSAMP]; MAX_NCHAN], /* used both for decoded Huffman values and dequantized coefficients */
    pub non_zero_bound: [i32; MAX_NCHAN], /* number of coeffs in huffDecBuf[ch] which can be > 0 */
    pub gb: [i32; MAX_NCHAN],             /* minimum number of guard bits in huffDecBuf[ch] */
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum HuffTabType {
    NoBits,
    OneShot,
    LoopNoLinbits,
    LoopLinbits,
    QuadA,
    QuadB,
    InvalidTab,
}

#[repr(C)]
pub struct HuffTabLookup {
    pub lin_bits: i32,
    pub tab_type: HuffTabType, /*HuffTabType*/
}

#[repr(C)]
pub struct IMDCTInfo {
    pub outBuf: [[[i32; NBANDS]; BLOCK_SIZE]; MAX_NCHAN], /* output of IMDCT */
    pub overBuf: [[i32; MAX_NSAMP / 2]; MAX_NCHAN], /* overlap-add buffer (by symmetry, only need 1/2 size) */
    pub numPrevIMDCT: [i32; MAX_NCHAN], /* how many IMDCT's calculated in this channel on prev. granule */
    pub prevType: [BlockType; MAX_NCHAN],
    pub prevWinSwitch: [i32; MAX_NCHAN],
    pub gb: [i32; MAX_NCHAN],
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ScaleFactorInfoSub {
    /* max bits in scalefactors = 5, so use char's to save space */
    pub l: [u8; 23],      /* [band] */
    pub s: [[u8; 3]; 13], /* [band][window] */
}

#[repr(C)]
pub struct ScaleFactorJS {
    /* used in MPEG 2, 2.5 intensity (joint) stereo only */
    pub intensity_scale: i32,
    pub slen: [i32; 4],
    pub nr: [i32; 4],
}

/* NOTE - could get by with smaller vbuf if memory is more important than speed
 *  (in Subband, instead of replicating each block in FDCT32 you would do a memmove on the
 *   last 15 blocks to shift them down one, a hardware style FIFO)
 */
#[repr(C)]
pub struct SubbandInfo {
    pub vbuf: [i32; MAX_NCHAN * VBUF_LENGTH], /* vbuf for fast DCT-based synthesis PQMF - double size for speed (no modulo indexing) */
    pub vindex: i32,                          /* internal index for tracking position in vbuf */
}

#[repr(C)]
pub struct MP3Decoder {
    pub m_MP3DecInfo: MP3DecInfo,
    pub m_FrameHeader: FrameHeader,
    pub m_MP3FrameInfo: MP3FrameInfo,
    pub m_SideInfo: SideInfo,
    pub m_SideInfoSub: [[SideInfoSub; MAX_NCHAN]; MAX_NGRAN],
    pub m_SFBandTable: SFBandTable,
    pub m_ScaleFactorJS: ScaleFactorJS,
    pub m_SubbandInfo: SubbandInfo,
    pub m_ScaleFactorInfoSub: [[ScaleFactorInfoSub; MAX_NCHAN]; MAX_NGRAN],
    pub m_CriticalBandInfo: [CriticalBandInfo; MAX_NCHAN], /* filled in dequantizer, used in joint stereo reconstruction */
    pub m_HuffmanInfo: HuffmanInfo,
    pub m_DequantInfo: DequantInfo,
    pub m_IMDCTInfo: IMDCTInfo,
    pub m_sMode: StereoMode,        /* mono/stereo mode */
    pub m_MPEGVersion: MPEGVersion, /* version ID */
}

/***********************************************************************************************************************
 * Function:    MP3FindSyncWord
 *
 * Description: locate the next byte-alinged sync word in the raw mp3 stream
 *
 * Inputs:      buffer to search for sync word
 *              max number of bytes to search in buffer
 *
 * Outputs:     none
 *
 * Return:      offset to first sync word (bytes from start of buf)
 *              -1 if sync not found after searching nBytes
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
pub fn mp3_find_sync_word(data: &[u8]) -> Option<&[u8]> {
    let mut tail = data;

    while tail.len() >= 2 {
        if tail[0] == SYNCWORDH && (tail[1] & SYNCWORDL) == SYNCWORDL {
            return Some(tail);
        }
        tail = &tail[1..];
    }
    None
}

/***********************************************************************************************************************
 * Function:    MP3FindFreeSync
 *
 * Description: figure out number of bytes between adjacent sync words in "free" mode
 *
 * Inputs:      buffer to search for next sync word
 *              the 4-byte frame header starting at the current sync word
 *              max number of bytes to search in buffer
 *
 * Outputs:     none
 *
 * Return:      offset to next sync word, minus any pad byte (i.e. nSlots)
 *              -1 if sync not found after searching nBytes
 *
 * Notes:       this checks that the first 22 bits of the next frame header are the
 *                same as the current frame header, but it's still not foolproof
 *                (could accidentally find a sequence in the bitstream which
 *                 appears to match but is not actually the next frame header)
 *              this could be made more error-resilient by checking several frames
 *                in a row and verifying that nSlots is the same in each case
 *              since free mode requires CBR (see spec) we generally only call
 *                this function once (first frame) then store the result (nSlots)
 *                and just use it from then on
 **********************************************************************************************************************/
/// Find the start of the next frame with a matching header (free format detection)
/// Returns the byte offset to the frame start (excluding padding byte if set)
pub fn mp3_find_free_sync(buf: &[u8], first_header: &[u8; 4]) -> Option<usize> {
    if buf.len() < 4 {
        return None;
    }

    let fh0 = first_header[0];
    let fh1 = first_header[1];
    let fh2_masked = first_header[2] & 0xFC;
    let padding = (first_header[2] >> 1) & 0x01 != 0;

    buf.windows(4)
        .position(|window| {
            window[0] == SYNCWORDH &&                  // sync high byte
            (window[1] & SYNCWORDL) == SYNCWORDL &&         // sync low bits
            window[0] == fh0 &&
            window[1] == fh1 &&
            (window[2] & 0xFC) == fh2_masked
        })
        .map(|pos| if padding { pos.saturating_sub(1) } else { pos })
}

/***********************************************************************************************************************
 * Function:    PolyphaseStereo
 *
 * Description: filter one subband and produce 32 output PCM samples for each channel
 *
 * Inputs:      pointer to PCM output buffer
 *              number of "extra shifts" (vbuf format = Q(DQ_FRACBITS_OUT-2))
 *              pointer to start of vbuf (preserved from last call)
 *              start of filter coefficient table (in proper, shuffled order)
 *              no minimum number of guard bits is required for input vbuf
 *                (see additional scaling comments below)
 *
 * Outputs:     32 samples of two channels of decoded PCM data, (i.e. Q16.0)
 *
 * Return:      none
 *
 * Notes:       interleaves PCM samples LRLRLR...
 **********************************************************************************************************************/

#[unsafe(no_mangle)]
pub fn polyphase_stereo(pcm: &mut [i16; 64], vbuf: &[i32], coef: &[u32; 264]) {
    if vbuf.len() < 1064 {
        return;
    }
    let rnd_val = 1 << ((DQ_FRACBITS_OUT - 2 - 2 - 15) - 1 + (32 - CSHIFT));
    let vbuf = &vbuf[..1064];
    /* special case, output sample 0 */
    let mut sum1_r: u64 = rnd_val;
    let mut sum1_l: u64 = rnd_val;
    let mut sum2_r: u64;
    let mut sum2_l: u64;
    let mut c1: u32;
    let mut c2: u32;
    let mut v_lo: i32;
    let mut v_hi: i32;

    for (j, c) in coef[..16].chunks_exact(2).enumerate() {
        c1 = c[0];
        c2 = c[1];
        v_lo = vbuf[j];
        v_hi = vbuf[23 - j];
        sum1_l = madd_64(sum1_l as u64, v_lo, c1 as i32);
        sum1_l = madd_64(sum1_l as u64, v_hi, -(c2 as i32));
        v_lo = vbuf[32 + j];
        v_hi = vbuf[32 + (23 - j)];
        sum1_r = madd_64(sum1_r as u64, v_lo, c1 as i32);
        sum1_r = madd_64(sum1_r as u64, v_hi, -(c2 as i32));
    }

    pcm[CHANNEL_LEFT] = clip_to_short(
        sar_64(sum1_l as u64, (32 - CSHIFT) as i32) as i32,
        (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
    );
    pcm[CHANNEL_RIGHT] = clip_to_short(
        sar_64(sum1_r as u64, (32 - CSHIFT) as i32) as i32,
        (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
    );

    /* special case, output sample 16 */
    let mut coef_idx = 256;
    let vbuf_idx = 64 * 16;
    sum1_l = rnd_val;
    sum1_r = rnd_val;

    for j in 0..8 {
        c1 = coef[coef_idx];
        coef_idx += 1;
        v_lo = vbuf[vbuf_idx + j];
        sum1_l = madd_64(sum1_l, v_lo, c1 as i32);
        v_lo = vbuf[vbuf_idx + 32 + j];
        sum1_r = madd_64(sum1_r, v_lo, c1 as i32);
    }
    pcm[2 * 16 + CHANNEL_LEFT] = clip_to_short(
        sar_64(sum1_l, (32 - CSHIFT) as i32) as i32,
        (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
    );
    pcm[2 * 16 + CHANNEL_RIGHT] = clip_to_short(
        sar_64(sum1_r, (32 - CSHIFT) as i32) as i32,
        (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
    );

    /* main convolution loop: sum1L = samples 1, 2, 3, ... 15   sum2L = samples 31, 30, ... 17 */
    let (coef_chunks, _) = coef[16..].as_chunks::<16>();
    let (vbuf_chunk, _) = vbuf[64..].as_chunks::<64>();

    let (pcm_head, pcm_tail) = pcm.split_at_mut(32);
    let (pcm_head, _) = &mut pcm_head[2..].as_chunks_mut::<2>();
    let (pcm_tail, _) = &mut pcm_tail[2..].as_chunks_mut::<2>();

    let pcm_head_iter = pcm_head.iter_mut();
    let pcm_tail_iter = pcm_tail.iter_mut().rev();

    for ((coef, vbuf), (pcm_head, pcm_tail)) in coef_chunks
        .iter()
        .zip(vbuf_chunk)
        .zip(pcm_head_iter.zip(pcm_tail_iter))
    {
        sum1_l = rnd_val;
        sum2_l = rnd_val;
        calculate_sums_l(coef, vbuf, &mut sum1_l, &mut sum2_l);

        sum1_r = rnd_val;
        sum2_r = rnd_val;
        calculate_sums_r(coef, vbuf, &mut sum1_r, &mut sum2_r);

        pcm_head[CHANNEL_LEFT] = clip_to_short(
            sar_64(sum1_l as u64, (32 - CSHIFT) as i32) as i32,
            (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
        );
        pcm_head[CHANNEL_RIGHT] = clip_to_short(
            sar_64(sum1_r as u64, (32 - CSHIFT) as i32) as i32,
            (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
        );
        pcm_tail[CHANNEL_LEFT] = clip_to_short(
            sar_64(sum2_l as u64, (32 - CSHIFT) as i32) as i32,
            (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
        );
        pcm_tail[CHANNEL_RIGHT] = clip_to_short(
            sar_64(sum2_r as u64, (32 - CSHIFT) as i32) as i32,
            (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
        );
    }
}

fn calculate_sums_l(coef: &[u32; 16], vbuf: &[i32; 64], sum1_l: &mut u64, sum2_l: &mut u64) {
    let (coef_chunks, _) = coef.as_chunks::<2>();
    for (j, coef) in coef_chunks.iter().enumerate() {
        let c1 = coef[0];
        let c2 = coef[1];
        let v_lo = vbuf[j];
        let v_hi = vbuf[23 - j];
        *sum1_l = madd_64(*sum1_l, v_lo, c1 as i32);
        *sum2_l = madd_64(*sum2_l, v_lo, c2 as i32);

        *sum1_l = madd_64(*sum1_l, v_hi, -(c2 as i32));
        *sum2_l = madd_64(*sum2_l, v_hi, c1 as i32);
    }
}

fn calculate_sums_r(coef: &[u32; 16], vbuf: &[i32; 64], sum1_r: &mut u64, sum2_r: &mut u64) {
    let (coef_chunks, _) = coef.as_chunks::<2>();
    for (j, coef) in coef_chunks.iter().enumerate() {
        let c1 = coef[0];
        let c2 = coef[1];
        let v_lo = vbuf[32 + j];
        let v_hi = vbuf[32 + 23 - j];
        *sum1_r = madd_64(*sum1_r, v_lo, c1 as i32);
        *sum2_r = madd_64(*sum2_r, v_lo, c2 as i32);
        *sum1_r = madd_64(*sum1_r, v_hi, -(c2 as i32));
        *sum2_r = madd_64(*sum2_r, v_hi, c1 as i32);
    }
}

pub fn polyphase_mono(mut pcm: &mut [i16], vbuf: &[i32], coef_base: &[u32]) {
    let mut v_lo: i32;
    let mut v_hi: i32;
    let mut c1: u32;
    let mut c2: u32;
    let mut sum1_l: u64;
    let mut sum2_l: u64;
    let rnd_val: u64 = 1 << ((DQ_FRACBITS_OUT - 2 - 2 - 15) - 1 + (32 - CSHIFT));

    /* special case, output sample 0 */
    let mut coef = coef_base;
    let mut vb1 = vbuf;
    sum1_l = rnd_val;
    for j in 0..8 {
        c1 = coef[0];
        coef = &coef[1..];
        c2 = coef[0];
        coef = &coef[1..];
        v_lo = vb1[j];
        v_hi = vb1[23 - (j)]; // 0...7
        sum1_l = madd_64(sum1_l, v_lo, c1 as i32);
        sum1_l = madd_64(sum1_l, v_hi, -(c2 as i32));
    }
    pcm[CHANNEL_MONO] = clip_to_short(
        sar_64(sum1_l, (32 - CSHIFT) as i32) as i32,
        (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
    );

    /* special case, output sample 16 */
    coef = &coef_base[256..];
    vb1 = &vbuf[64 * 16..];
    sum1_l = rnd_val;
    for j in 0..8 {
        c1 = coef[0];
        coef = &coef[1..];
        v_lo = vb1[j];
        sum1_l = madd_64(sum1_l, v_lo, c1 as i32); // 0...7
    }
    pcm[16] = clip_to_short(
        sar_64(sum1_l, (32 - CSHIFT) as i32) as i32,
        (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
    );

    /* main convolution loop: sum1L = samples 1, 2, 3, ... 15   sum2L = samples 31, 30, ... 17 */
    coef = &coef_base[16..];
    vb1 = &vbuf[64..];
    pcm = &mut pcm[1..];

    /* right now, the compiler creates bad asm from this... */
    for i in (1..=15).rev() {
        sum1_l = rnd_val;
        sum2_l = rnd_val;
        for j in 0..8 {
            c1 = coef[0];
            coef = &coef[1..];
            c2 = coef[0];
            coef = &coef[1..];
            v_lo = vb1[j];
            v_hi = vb1[23 - j];
            sum1_l = madd_64(sum1_l, v_lo, c1 as i32);
            sum2_l = madd_64(sum2_l, v_lo, c2 as i32);
            sum1_l = madd_64(sum1_l, v_hi, -(c2 as i32));
            sum2_l = madd_64(sum2_l, v_hi, c1 as i32);
        }
        vb1 = &vb1[64..];
        pcm[0] = clip_to_short(
            sar_64(sum1_l, (32 - CSHIFT) as i32) as i32,
            (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
        );
        pcm[2 * i] = clip_to_short(
            sar_64(sum2_l, (32 - CSHIFT) as i32) as i32,
            (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32,
        );
        pcm = &mut pcm[1..];
    }
}

/***********************************************************************************************************************
 * D C T 3 2
 **********************************************************************************************************************/

/***********************************************************************************************************************
 * Function:    FDCT32
 *
 * Description: Ken's highly-optimized 32-point DCT (radix-4 + radix-8)
 *
 * Inputs:      input buffer, length = 32 samples
 *              require at least 6 guard bits in input vector x to avoid possibility
 *                of overflow in internal calculations (see bbtest_imdct test app)
 *              buffer offset and oddblock flag for polyphase filter input buffer
 *              number of guard bits in input
 *
 * Outputs:     output buffer, data copied and interleaved for polyphase filter
 *              no guarantees about number of guard bits in output
 *
 * Return:      none
 *
 * Notes:       number of muls = 4*8 + 12*4 = 80
 *              final stage of DCT is hardcoded to shuffle data into the proper order
 *                for the polyphase filterbank
 *              fully unrolled stage 1, for max precision (scale the 1/cos() factors
 *                differently, depending on magnitude)
 *              guard bit analysis verified by exhaustive testing of all 2^32
 *                combinations of max pos/max neg values in x[]
 **********************************************************************************************************************/
const FDCT32S1S2: [i32; 16] = [5, 3, 3, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 4];

const M_COS0_0: i32 = 0x4013c251; /* Q31 */
const M_COS0_1: i32 = 0x40b345bd; /* Q31 */
const M_COS0_2: i32 = 0x41fa2d6d; /* Q31 */
const M_COS0_3: i32 = 0x43f93421; /* Q31 */
const M_COS0_4: i32 = 0x46cc1bc4; /* Q31 */
const M_COS0_5: i32 = 0x4a9d9cf0; /* Q31 */
const M_COS0_6: i32 = 0x4fae3711; /* Q31 */
const M_COS0_7: i32 = 0x56601ea7; /* Q31 */
const M_COS0_8: i32 = 0x5f4cf6eb; /* Q31 */
const M_COS0_9: i32 = 0x6b6fcf26; /* Q31 */
const M_COS0_10: i32 = 0x7c7d1db3; /* Q31 */
const M_COS0_11: i32 = 0x4ad81a97; /* Q30 */
const M_COS0_12: i32 = 0x5efc8d96; /* Q30 */
const M_COS0_13: i32 = 0x41d95790; /* Q29 */
const M_COS0_14: i32 = 0x6d0b20cf; /* Q29 */
const M_COS0_15: i32 = 0x518522fb; /* Q27 */
const M_COS1_0: i32 = 0x404f4672; /* Q31 */
const M_COS1_1: i32 = 0x42e13c10; /* Q31 */
const M_COS1_2: i32 = 0x48919f44; /* Q31 */
const M_COS1_3: i32 = 0x52cb0e63; /* Q31 */
const M_COS1_4: i32 = 0x64e2402e; /* Q31 */
const M_COS1_5: i32 = 0x43e224a9; /* Q30 */
const M_COS1_6: i32 = 0x6e3c92c1; /* Q30 */
const M_COS1_7: i32 = 0x519e4e04; /* Q28 */
const M_COS2_0: i32 = 0x4140fb46; /* Q31 */
const M_COS2_1: i32 = 0x4cf8de88; /* Q31 */
const M_COS2_2: i32 = 0x73326bbf; /* Q31 */
const M_COS2_3: i32 = 0x52036742; /* Q29 */
const M_COS3_0: i32 = 0x4545e9ef; /* Q31 */
const M_COS3_1: i32 = 0x539eba45; /* Q30 */
const M_COS4_0: i32 = 0x5a82799a; /* Q31 */

const M_DCTTAB: [i32; 48] = [
    /* first pass */
    M_COS0_0, M_COS0_15, M_COS1_0, /* 31, 27, 31 */
    M_COS0_1, M_COS0_14, M_COS1_1, /* 31, 29, 31 */
    M_COS0_2, M_COS0_13, M_COS1_2, /* 31, 29, 31 */
    M_COS0_3, M_COS0_12, M_COS1_3, /* 31, 30, 31 */
    M_COS0_4, M_COS0_11, M_COS1_4, /* 31, 30, 31 */
    M_COS0_5, M_COS0_10, M_COS1_5, /* 31, 31, 30 */
    M_COS0_6, M_COS0_9, M_COS1_6, /* 31, 31, 30 */
    M_COS0_7, M_COS0_8, M_COS1_7, /* 31, 31, 28 */
    /* second pass */
    M_COS2_0, M_COS2_3, M_COS3_0, /* 31, 29, 31 */
    M_COS2_1, M_COS2_2, M_COS3_1, /* 31, 31, 30 */
    -M_COS2_0, -M_COS2_3, M_COS3_0, /* 31, 29, 31 */
    -M_COS2_1, -M_COS2_2, M_COS3_1, /* 31, 31, 30 */
    M_COS2_0, M_COS2_3, M_COS3_0, /* 31, 29, 31 */
    M_COS2_1, M_COS2_2, M_COS3_1, /* 31, 31, 30 */
    -M_COS2_0, -M_COS2_3, M_COS3_0, /* 31, 29, 31 */
    -M_COS2_1, -M_COS2_2, M_COS3_1, /* 31, 31, 30 */
];

fn fdct_32_first_pass(buf_slice: &mut [i32; NBANDS], cptr0: &[i32; 24]) {
    let (fdct_head, fdct_tail) = FDCT32S1S2.split_at(8);
    for (i, (fh, ft)) in fdct_head.iter().zip(fdct_tail.iter()).enumerate() {
        let base = i * 3;
        let a0 = buf_slice[i];
        let a1 = buf_slice[15 - i];
        let a2 = buf_slice[16 + i];
        let a3 = buf_slice[31 - i];

        let b0 = a0 + a3;
        let b3 = mulshift_32(cptr0[base], a0 - a3) << 1;
        let b1 = a1 + a2;
        let b2 = mulshift_32(cptr0[base + 1], a1 - a2) << (*fh as i32);

        let coeff = cptr0[base + 2];
        let shift_idx = *ft as i32;

        buf_slice[i] = b0 + b1;
        buf_slice[15 - i] = mulshift_32(coeff, b0 - b1) << shift_idx;
        buf_slice[16 + i] = b2 + b3;
        buf_slice[31 - i] = mulshift_32(coeff, b3 - b2) << shift_idx;
    }
}

pub fn fdct_32(
    buf_slice: &mut [i32; NBANDS],
    dest_slice: &mut [i32],
    offset: i32,
    odd_block: i32,
    gb: i32,
) {
    const EXPECTED_LENGTH: usize = MAX_NCHAN * VBUF_LENGTH - 32;
    let dest_slice = &mut dest_slice[0..MAX_NCHAN * VBUF_LENGTH - 32];
    if dest_slice.len() < EXPECTED_LENGTH {
        return;
    }
    let mut es = 0;
    if gb < 6 {
        es = 6 - gb;
        for d in buf_slice.iter_mut() {
            *d >>= es;
        }
    }

    let (cptr_all, _) = M_DCTTAB.as_chunks::<24>();
    let cptr0: &[i32; 24] = &cptr_all[0];

    /* --- FIRST PASS: Butterfly and Phase 1 --- */
    fdct_32_first_pass(buf_slice, cptr0);

    /* --- SECOND PASS: 4x8-point DCT --- */
    let cptr1: &[i32; 24] = &cptr_all[1];
    let (chunks, _) = buf_slice.as_chunks_mut::<8>();
    let (cptr_chunks, _) = cptr1.as_chunks::<6>();

    for (buf_chunk, cptr) in chunks.iter_mut().zip(cptr_chunks) {
        let b0 = buf_chunk[0] + buf_chunk[7];
        let b7 = mulshift_32(cptr[0], buf_chunk[0] - buf_chunk[7]) << 1;
        let b3 = buf_chunk[3] + buf_chunk[4];
        let b4 = mulshift_32(cptr[1], buf_chunk[3] - buf_chunk[4]) << 3;

        let t0 = b0 + b3;
        let t3 = mulshift_32(cptr[2], b0 - b3) << 1;
        let t4 = b4 + b7;
        let t7 = mulshift_32(cptr[2], b7 - b4) << 1;

        let b1 = buf_chunk[1] + buf_chunk[6];
        let b6 = mulshift_32(cptr[3], buf_chunk[1] - buf_chunk[6]) << 1;
        let b2 = buf_chunk[2] + buf_chunk[5];
        let b5 = mulshift_32(cptr[4], buf_chunk[2] - buf_chunk[5]) << 1;

        let t1 = b1 + b2;
        let t2 = mulshift_32(cptr[5], b1 - b2) << 2;
        let t5 = b5 + b6;
        let t6 = mulshift_32(cptr[5], b6 - b5) << 2;

        let bb0 = t0 + t1;
        let bb1 = mulshift_32(M_COS4_0, t0 - t1) << 1;
        let bb2 = t2 + t3;
        let bb3 = mulshift_32(M_COS4_0, t3 - t2) << 1;

        buf_chunk[0] = bb0;
        buf_chunk[1] = bb1;
        buf_chunk[2] = bb2 + bb3;
        buf_chunk[3] = bb3;

        let bb4 = t4 + t5;
        let bb5 = mulshift_32(M_COS4_0, t4 - t5) << 1;
        let bb6 = t6 + t7;
        let bb7 = mulshift_32(M_COS4_0, t7 - t6) << 1;
        let bb6_sum = bb6 + bb7;

        buf_chunk[4] = bb4 + bb6_sum;
        buf_chunk[5] = bb5 + bb7;
        buf_chunk[6] = bb5 + bb6_sum;
        buf_chunk[7] = bb7;
    }

    // Obliczanie bazowych offsetów dla d[]
    let v_toggle = if odd_block != 0 { 0 } else { VBUF_LENGTH };
    let v_toggle_inv = v_toggle ^ VBUF_LENGTH;
    let off_8 = ((offset - odd_block) & 7) as usize;

    /* sample 0 - delayed block */
    let d0_idx = 64 * 16 + off_8 + v_toggle;
    if let Some(d0_slot) = dest_slice.get_mut(d0_idx..d0_idx + 9) {
        let s = buf_slice[0];
        d0_slot[0] = s;
        d0_slot[8] = s;
    }

    /* samples 16 to 31 */
    let d16_base = offset as usize + v_toggle_inv;
    if let Some(d16_slice) = dest_slice.get_mut(d16_base..d16_base + 1024) {
        let (d, _) = d16_slice.as_chunks_mut::<64>();

        let mut s = buf_slice[1];
        d[0][0] = s;
        d[0][8] = s;

        let tmp1 = buf_slice[25] + buf_slice[29];
        s = buf_slice[17] + tmp1;
        d[1][0] = s;
        d[1][8] = s;
        s = buf_slice[9] + buf_slice[13];
        d[2][0] = s;
        d[2][8] = s;
        s = buf_slice[21] + tmp1;
        d[3][0] = s;
        d[3][8] = s;

        let tmp2 = buf_slice[29] + buf_slice[27];
        s = buf_slice[5];
        d[4][0] = s;
        d[4][8] = s;
        s = buf_slice[21] + tmp2;
        d[5][0] = s;
        d[5][8] = s;
        s = buf_slice[13] + buf_slice[11];
        d[6][0] = s;
        d[6][8] = s;
        s = buf_slice[19] + tmp2;
        d[7][0] = s;
        d[7][8] = s;

        let tmp3 = buf_slice[27] + buf_slice[31];
        s = buf_slice[3];
        d[8][0] = s;
        d[8][8] = s;
        s = buf_slice[19] + tmp3;
        d[9][0] = s;
        d[9][8] = s;
        s = buf_slice[11] + buf_slice[15];
        d[10][0] = s;
        d[10][8] = s;
        s = buf_slice[23] + tmp3;
        d[11][0] = s;
        d[11][8] = s;

        let tmp4 = buf_slice[31];
        s = buf_slice[7];
        d[12][0] = s;
        d[12][8] = s;
        s = buf_slice[23] + tmp4;
        d[13][0] = s;
        d[13][8] = s;
        s = buf_slice[15];
        d[14][0] = s;
        d[14][8] = s;
        s = tmp4;
        d[15][0] = s;
        d[15][8] = s;
    }

    /* samples 1 to 16 */
    let d1_base = 16 + off_8 + v_toggle;
    if let Some(d1_slice) = dest_slice.get_mut(d1_base..d1_base + 1024) {
        let (d, _) = d1_slice.as_chunks_mut::<64>();

        let mut s = buf_slice[1];
        d[0][0] = s;
        d[0][8] = s;

        let tmp1 = buf_slice[30] + buf_slice[25];
        s = buf_slice[17] + tmp1;
        d[1][0] = s;
        d[1][8] = s;
        s = buf_slice[14] + buf_slice[9];
        d[2][0] = s;
        d[2][8] = s;
        s = buf_slice[22] + tmp1;
        d[3][0] = s;
        d[3][8] = s;
        s = buf_slice[6];
        d[4][0] = s;
        d[4][8] = s;

        let tmp2 = buf_slice[26] + buf_slice[30];
        s = buf_slice[22] + tmp2;
        d[5][0] = s;
        d[5][8] = s;
        s = buf_slice[10] + buf_slice[14];
        d[6][0] = s;
        d[6][8] = s;
        s = buf_slice[18] + tmp2;
        d[7][0] = s;
        d[7][8] = s;
        s = buf_slice[2];
        d[8][0] = s;
        d[8][8] = s;

        let tmp3 = buf_slice[28] + buf_slice[26];
        s = buf_slice[18] + tmp3;
        d[9][0] = s;
        d[9][8] = s;
        s = buf_slice[12] + buf_slice[10];
        d[10][0] = s;
        d[10][8] = s;
        s = buf_slice[20] + tmp3;
        d[11][0] = s;
        d[11][8] = s;
        s = buf_slice[4];
        d[12][0] = s;
        d[12][8] = s;

        let tmp4 = buf_slice[24] + buf_slice[28];
        s = buf_slice[20] + tmp4;
        d[13][0] = s;
        d[13][8] = s;
        s = buf_slice[8] + buf_slice[12];
        d[14][0] = s;
        d[14][8] = s;
        s = buf_slice[16] + tmp4;
        d[15][0] = s;
        d[15][8] = s;
    }

    /* final rescale + clip if es > 0 */
    if es != 0 {
        let n_clip = (31 - es) as u32;

        // Re-use logic for sample 0
        if let Some(val) = dest_slice.get_mut(d0_idx) {
            let s = (clip_2n(*val, n_clip)) << es;
            *val = s;
            dest_slice[d0_idx + 8] = s;
        }

        // Re-use logic for 16-31
        if let Some(d16_slice) = dest_slice.get_mut(d16_base..d16_base + 1024) {
            for i in d16_slice.chunks_exact_mut(64).take(16) {
                let s = (clip_2n(i[0], n_clip)) << es;
                i[0] = s;
                i[8] = s;
            }
        }

        // Re-use logic for 1-16
        if let Some(d1_slice) = dest_slice.get_mut(d1_base..d1_base + 1024) {
            for i in d1_slice.chunks_exact_mut(64).take(16) {
                let s = (clip_2n(i[0], n_clip)) << es;
                i[0] = s;
                i[8] = s;
            }
        }
    }
}

pub fn freq_invert_rescale(y_full: &mut [i32], x_prev: &mut [i32], block_idx: i32, es: i32) -> i32 {
    let is_odd_block = (block_idx & 0x01) == 0x01;

    if es == 0 {
        if is_odd_block {
            for row in 0..9 {
                let idx = (2 * row + 1) * NBANDS;
                if let Some(val) = y_full.get_mut(idx) {
                    *val = val.wrapping_neg();
                }
            }
        }
        return 0;
    }

    let mut m_out = 0;
    let n_clip = (31 - es) as u32;

    for (i, xp) in x_prev.iter_mut().enumerate() {
        let base = i * 2 * NBANDS;

        // --- Pierwszy wiersz (Parzysty) ---
        if let Some(val) = y_full.get_mut(base) {
            let d = clip_2n(*val, n_clip);
            let res = d << es;
            *val = res;
            m_out |= res.abs();
        }

        if let Some(val) = y_full.get_mut(base + NBANDS) {
            let mut d = *val;
            if is_odd_block {
                d = -d;
            }
            d = clip_2n(d, n_clip);
            let res = d << es;
            *val = res;
            m_out |= res.abs();
        }

        let d_xp = clip_2n(*xp, n_clip);
        let res_xp = d_xp << es;
        *xp = res_xp;
    }

    m_out
}

/***********************************************************************************************************************
 * Function:    WinPrevious
 *
 * Description: apply specified window to second half of previous IMDCT (overlap part)
 *
 * Inputs:      vector of 9 coefficients (xPrev)
 *
 * Outputs:     18 windowed output coefficients (gain 1 integer bit)
 *              window type (0, 1, 2, 3)
 *
 * Return:      none
 *
 * Notes:       produces 9 output samples from 18 input samples via symmetry
 *              all blocks gain at least 1 guard bit via window (long blocks get extra
 *                sign bit, short blocks can have one addition but max gain < 1.0)
 **********************************************************************************************************************/

const IMDCT_WIN: [[u32; 36]; 4] = [
    [
        0x02aace8b, 0x07311c28, 0x0a868fec, 0x0c913b52, 0x0d413ccd, 0x0c913b52, 0x0a868fec,
        0x07311c28, 0x02aace8b, 0xfd16d8dd, 0xf6a09e66, 0xef7a6275, 0xe7dbc161, 0xe0000000,
        0xd8243e9f, 0xd0859d8b, 0xc95f619a, 0xc2e92723, 0xbd553175, 0xb8cee3d8, 0xb5797014,
        0xb36ec4ae, 0xb2bec333, 0xb36ec4ae, 0xb5797014, 0xb8cee3d8, 0xbd553175, 0xc2e92723,
        0xc95f619a, 0xd0859d8b, 0xd8243e9f, 0xe0000000, 0xe7dbc161, 0xef7a6275, 0xf6a09e66,
        0xfd16d8dd,
    ],
    [
        0x02aace8b, 0x07311c28, 0x0a868fec, 0x0c913b52, 0x0d413ccd, 0x0c913b52, 0x0a868fec,
        0x07311c28, 0x02aace8b, 0xfd16d8dd, 0xf6a09e66, 0xef7a6275, 0xe7dbc161, 0xe0000000,
        0xd8243e9f, 0xd0859d8b, 0xc95f619a, 0xc2e92723, 0xbd44ef14, 0xb831a052, 0xb3aa3837,
        0xafb789a4, 0xac6145bb, 0xa9adecdc, 0xa864491f, 0xad1868f0, 0xb8431f49, 0xc8f42236,
        0xdda8e6b1, 0xf47755dc, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000,
    ],
    [
        0x07311c28, 0x0d413ccd, 0x07311c28, 0xf6a09e66, 0xe0000000, 0xc95f619a, 0xb8cee3d8,
        0xb2bec333, 0xb8cee3d8, 0xc95f619a, 0xe0000000, 0xf6a09e66, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
        0x00000000,
    ],
    [
        0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x028e9709,
        0x04855ec0, 0x026743a1, 0xfcde2c10, 0xf515dc82, 0xec93e53b, 0xe4c880f8, 0xdd5d0b08,
        0xd63510b7, 0xcf5e834a, 0xc8e6b562, 0xc2da4105, 0xbd553175, 0xb8cee3d8, 0xb5797014,
        0xb36ec4ae, 0xb2bec333, 0xb36ec4ae, 0xb5797014, 0xb8cee3d8, 0xbd553175, 0xc2e92723,
        0xc95f619a, 0xd0859d8b, 0xd8243e9f, 0xe0000000, 0xe7dbc161, 0xef7a6275, 0xf6a09e66,
        0xfd16d8dd,
    ],
];

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub fn win_previous(
    x_prev: &mut [i32; BLOCK_SIZE / 2],
    x_prev_win: &mut [i32; BLOCK_SIZE],
    bt_prev: BlockType,
) {
    match bt_prev {
        BlockType::Short => {
            // Special case for short blocks – explicit unrolled version matching the original
            let w = IMDCT_WIN[2];

            x_prev_win[0] =
                mulshift_32(w[6] as i32, x_prev[2]) + mulshift_32(w[0] as i32, x_prev[6]);
            x_prev_win[1] =
                mulshift_32(w[7] as i32, x_prev[1]) + mulshift_32(w[1] as i32, x_prev[7]);
            x_prev_win[2] =
                mulshift_32(w[8] as i32, x_prev[0]) + mulshift_32(w[2] as i32, x_prev[8]);
            x_prev_win[3] =
                mulshift_32(w[9] as i32, x_prev[0]) + mulshift_32(w[3] as i32, x_prev[8]);
            x_prev_win[4] =
                mulshift_32(w[10] as i32, x_prev[1]) + mulshift_32(w[4] as i32, x_prev[7]);
            x_prev_win[5] =
                mulshift_32(w[11] as i32, x_prev[2]) + mulshift_32(w[5] as i32, x_prev[6]);
            x_prev_win[6] = mulshift_32(w[6] as i32, x_prev[5]);
            x_prev_win[7] = mulshift_32(w[7] as i32, x_prev[4]);
            x_prev_win[8] = mulshift_32(w[8] as i32, x_prev[3]);
            x_prev_win[9] = mulshift_32(w[9] as i32, x_prev[3]);
            x_prev_win[10] = mulshift_32(w[10] as i32, x_prev[4]);
            x_prev_win[11] = mulshift_32(w[11] as i32, x_prev[5]);

            // Zero the unused upper part (original sets 12..17 to 0)
            x_prev_win[12..].fill(0);
        }
        t => {
            // Long blocks (0, 1, 3) – symmetric windowing
            // wpLo points to imdctWin[btPrev] + 18
            // wpHi points to imdctWin[btPrev] + 35 (i.e. wpLo + 17 backwards)
            let win = IMDCT_WIN[t as usize];
            let wp_lo = &win[18..36]; // 18 elements forward
            let wp_hi = &win[18..36][..18]; // same range, but we will iterate backwards

            let mut lo_idx = 0;
            let mut hi_idx = 17;

            for &x in x_prev.iter() {
                let w_lo = wp_lo[lo_idx];
                let w_hi = wp_hi[hi_idx];

                x_prev_win[lo_idx] = mulshift_32(w_lo as i32, x);
                x_prev_win[17 - lo_idx] = mulshift_32(w_hi as i32, x);

                lo_idx += 1;
                hi_idx -= 1;
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(usize)]
pub enum ChannelCount {
    SingleChannel = 1,
    DualChannel = 2,
}

impl ChannelCount {
    pub const fn channels(&self) -> &'static [ChannelIndex] {
        match self {
            ChannelCount::SingleChannel => &[ChannelIndex::Channel0],
            ChannelCount::DualChannel => &[ChannelIndex::Channel0, ChannelIndex::Channel1],
        }
    }
}

impl GranuleCount {
    pub const fn granules(&self) -> &'static [GranuleIndex] {
        match self {
            GranuleCount::Mpeg1Granule => &[GranuleIndex::Granule0, GranuleIndex::Granule1],
            GranuleCount::Mpeg2Granule => &[GranuleIndex::Granule0],
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(usize)]
pub enum GranuleCount {
    Mpeg1Granule = 2,
    Mpeg2Granule = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum ChannelIndex {
    Channel0 = 0,
    Channel1 = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum GranuleIndex {
    Granule0 = 0,
    Granule1 = 1,
}

#[repr(C)]
#[allow(non_snake_case)]
#[derive(Debug)]
pub struct MP3DecInfo {
    /* buffer which must be large enough to hold largest possible main_data section */
    pub mainBuf: [u8; MAINBUF_SIZE],
    /* special info for "free" bitrate files */
    pub freeBitrateFlag: i32,
    pub freeBitrateSlots: i32,
    /* user-accessible info */
    pub bitrate: i32,
    pub nChans: ChannelCount,
    pub samprate: i32,
    pub nGrans: GranuleCount, /* granules per frame */
    pub nGranSamps: i32,      /* samples per granule */
    pub nSlots: i32,
    pub layer: LayerIndex,

    pub mainDataBegin: i32,
    pub mainDataBytes: i32,
    pub part23Length: [[i32; MAX_NCHAN]; MAX_NGRAN],
}

pub const SAMPLERATE_TAB: [[i32; 3]; 3] = [
    [44100, 48000, 32000], /* MPEG-1 */
    [22050, 24000, 16000], /* MPEG-2 */
    [11025, 12000, 8000],  /* MPEG-2.5 */
];

/* indexing = [version][mono/stereo]
 * number of bytes in side info section of bitstream
 */
const SIDE_BYTES_TAB: [[i32; 2]; 3] = [
    [17, 32], /* MPEG-1:   mono, stereo */
    [9, 17],  /* MPEG-2:   mono, stereo */
    [9, 17],  /* MPEG-2.5: mono, stereo */
];

/* indexing = [version][sampleRate][long (.l) or short (.s) block]
 *   sfBandTable[v][s].l[cb] = index of first bin in critical band cb (long blocks)
 *   sfBandTable[v][s].s[cb] = index of first bin in critical band cb (short blocks)
 */
const SF_BAND_TABLE: [[SFBandTable; 3]; 3] = [
    [
        /* MPEG-1 (44, 48, 32 kHz) */
        SFBandTable {
            l: [
                0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 52, 62, 74, 90, 110, 134, 162, 196, 238, 288,
                342, 418, 576,
            ],
            s: [0, 4, 8, 12, 16, 22, 30, 40, 52, 66, 84, 106, 136, 192],
        },
        SFBandTable {
            l: [
                0, 4, 8, 12, 16, 20, 24, 30, 36, 42, 50, 60, 72, 88, 106, 128, 156, 190, 230, 276,
                330, 384, 576,
            ],
            s: [0, 4, 8, 12, 16, 22, 28, 38, 50, 64, 80, 100, 126, 192],
        },
        SFBandTable {
            l: [
                0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 54, 66, 82, 102, 126, 156, 194, 240, 296, 364,
                448, 550, 576,
            ],
            s: [0, 4, 8, 12, 16, 22, 30, 42, 58, 78, 104, 138, 180, 192],
        },
    ],
    [
        /* MPEG-2 (22, 24, 16 kHz) */
        SFBandTable {
            l: [
                0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 116, 140, 168, 200, 238, 284, 336,
                396, 464, 522, 576,
            ],
            s: [0, 4, 8, 12, 18, 24, 32, 42, 56, 74, 100, 132, 174, 192],
        },
        SFBandTable {
            l: [
                0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 114, 136, 162, 194, 232, 278, 332,
                394, 464, 540, 576,
            ],
            s: [0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 136, 180, 192],
        },
        SFBandTable {
            l: [
                0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 116, 140, 168, 200, 238, 284, 336,
                396, 464, 522, 576,
            ],
            s: [0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192],
        },
    ],
    [
        /* MPEG-2.5 (11, 12, 8 kHz) */
        SFBandTable {
            l: [
                0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 116, 140, 168, 200, 238, 284, 336,
                396, 464, 522, 576,
            ],
            s: [0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192],
        },
        SFBandTable {
            l: [
                0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 116, 140, 168, 200, 238, 284, 336,
                396, 464, 522, 576,
            ],
            s: [0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192],
        },
        SFBandTable {
            l: [
                0, 12, 24, 36, 48, 60, 72, 88, 108, 132, 160, 192, 232, 280, 336, 400, 476, 566,
                568, 570, 572, 574, 576,
            ],
            s: [0, 8, 16, 24, 36, 52, 72, 96, 124, 160, 162, 164, 166, 192],
        },
    ],
];

/* indexing = [version][layer]
 * number of samples in one frame (per channel)
 */
pub const SAMPLES_PER_FRAME_TAB: [[i32; 3]; 3] = [
    [384, 1152, 1152], /* MPEG1 */
    [384, 1152, 576],  /* MPEG2 */
    [384, 1152, 576],  /* MPEG2.5 */
];

#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(C)]
pub enum MPEGVersion {
    /* map to 0,1,2 to make table indexing easier */
    MPEG1 = 0,
    MPEG2 = 1,
    MPEG25 = 2,
}

#[repr(C)]
pub struct MP3FrameInfo {
    pub bitrate: i32,
    pub n_chans: ChannelCount,
    pub samprate: i32,
    pub bitsPerSample: i32,
    pub outputSamps: i32,
    pub layer: LayerIndex,
    pub version: MPEGVersion,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SFBandTable {
    pub l: [i32; 23],
    pub s: [i32; 14],
}

#[repr(C)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum StereoMode {
    /* map these to the corresponding 2-bit values in the frame header */
    Stereo = 0x00, /* two independent channels, but L and R frames might have different # of bits */
    Joint = 0x01, /* coupled channels - layer III: mix of M-S and intensity, Layers I/II: intensity and direct coding only */
    Dual = 0x02,  /* two independent channels, L and R always have exactly 1/2 the total bitrate */
    Mono = 0x03,  /* one channel */
}

impl StereoMode {
    pub const fn get_channel_count(&self) -> ChannelCount {
        match &self {
            StereoMode::Mono => ChannelCount::SingleChannel,
            _ => ChannelCount::DualChannel,
        }
    }
}

/* indexing = [version][layer][bitrate index]
 * bitrate (kbps) of frame
 *   - bitrate index == 0 is "free" mode (bitrate determined on the fly by
 *       counting bits between successive sync words)
 */
const BITRATE_TAB: [[[i16; 15]; 3]; 3] = [
    [
        /* MPEG-1 */
        [
            0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448,
        ], /* Layer 1 */
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384,
        ], /* Layer 2 */
        [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
        ], /* Layer 3 */
    ],
    [
        /* MPEG-2 */
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
        ], /* Layer 1 */
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160], /* Layer 2 */
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160], /* Layer 3 */
    ],
    [
        /* MPEG-2.5 */
        [
            0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256,
        ], /* Layer 1 */
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160], /* Layer 2 */
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160], /* Layer 3 */
    ],
];

/* indexing = [version][sampleRate][bitRate]
 * for layer3, nSlots = floor(samps/frame * bitRate / sampleRate / 8)
 *   - add one pad slot if necessary
 */
const SLOT_TAB: [[[i16; 15]; 3]; 3] = [
    [
        /* MPEG-1 */
        [
            0, 104, 130, 156, 182, 208, 261, 313, 365, 417, 522, 626, 731, 835, 1044,
        ], /* 44 kHz */
        [
            0, 96, 120, 144, 168, 192, 240, 288, 336, 384, 480, 576, 672, 768, 960,
        ], /* 48 kHz */
        [
            0, 144, 180, 216, 252, 288, 360, 432, 504, 576, 720, 864, 1008, 1152, 1440,
        ], /* 32 kHz */
    ],
    [
        /* MPEG-2 */
        [
            0, 26, 52, 78, 104, 130, 156, 182, 208, 261, 313, 365, 417, 470, 522,
        ], /* 22 kHz */
        [
            0, 24, 48, 72, 96, 120, 144, 168, 192, 240, 288, 336, 384, 432, 480,
        ], /* 24 kHz */
        [
            0, 36, 72, 108, 144, 180, 216, 252, 288, 360, 432, 504, 576, 648, 720,
        ], /* 16 kHz */
    ],
    [
        /* MPEG-2.5 */
        [
            0, 52, 104, 156, 208, 261, 313, 365, 417, 522, 626, 731, 835, 940, 1044,
        ], /* 11 kHz */
        [
            0, 48, 96, 144, 192, 240, 288, 336, 384, 480, 576, 672, 768, 864, 960,
        ], /* 12 kHz */
        [
            0, 72, 144, 216, 288, 360, 432, 504, 576, 720, 864, 1008, 1152, 1296, 1440,
        ], /*  8 kHz */
    ],
];

impl MP3Decoder {
    pub fn subband(&mut self, mut pcm_buf: &mut [i16]) -> i32 {
        if self.m_MP3DecInfo.nChans == ChannelCount::DualChannel {
            /* stereo */
            let (chunks, _) = pcm_buf.as_chunks_mut::<64>();
            if chunks.len() < BLOCK_SIZE {
                return -1;
            }
            for (b, chunk) in chunks.iter_mut().take(BLOCK_SIZE).enumerate() {
                fdct_32(
                    &mut self.m_IMDCTInfo.outBuf[0][b],
                    &mut self.m_SubbandInfo.vbuf,
                    self.m_SubbandInfo.vindex,
                    b as i32 & 0x01,
                    self.m_IMDCTInfo.gb[0],
                );
                fdct_32(
                    &mut self.m_IMDCTInfo.outBuf[1][b],
                    &mut self.m_SubbandInfo.vbuf[1 * 32..],
                    self.m_SubbandInfo.vindex,
                    b as i32 & 0x01,
                    self.m_IMDCTInfo.gb[1],
                );
                polyphase_stereo(
                    chunk,
                    &self.m_SubbandInfo.vbuf[self.m_SubbandInfo.vindex as usize
                        + VBUF_LENGTH * (b as i32 & 0x01) as usize..],
                    &POLY_COEF,
                );
                self.m_SubbandInfo.vindex = (self.m_SubbandInfo.vindex - (b as i32 & 0x01)) & 7;
            }
        } else {
            /* mono */
            for b in 0..BLOCK_SIZE {
                fdct_32(
                    &mut self.m_IMDCTInfo.outBuf[0][b],
                    &mut self.m_SubbandInfo.vbuf,
                    self.m_SubbandInfo.vindex,
                    b as i32 & 0x01,
                    self.m_IMDCTInfo.gb[0],
                );
                polyphase_mono(
                    pcm_buf,
                    &self.m_SubbandInfo.vbuf
                        [self.m_SubbandInfo.vindex as usize + VBUF_LENGTH * (b & 0x01)..],
                    &POLY_COEF,
                );
                self.m_SubbandInfo.vindex = (self.m_SubbandInfo.vindex - (b as i32 & 0x01)) & 7;
                pcm_buf = &mut pcm_buf[NBANDS as usize..];
            }
        }

        return 0;
    }

    pub fn unpack_frame_header(&mut self, buf: &[u8]) -> Result<usize, i8> {
        /* validate pointers and sync word */
        if buf.len() < 4 {
            return Err(ERR_MP3_INVALID_FRAMEHEADER);
        }
        if (buf[0] & SYNCWORDH) != SYNCWORDH || (buf[1] & SYNCWORDL) != SYNCWORDL {
            return Err(ERR_MP3_INVALID_FRAMEHEADER);
        }
        let m_frame_header = &mut self.m_FrameHeader;
        let m_mp3_dec_info = &mut self.m_MP3DecInfo;
        /* read header fields - use bitmasks instead of GetBits() for speed, since format never varies */
        let ver_idx = (buf[1] >> 3) & 0x03;
        self.m_MPEGVersion = if ver_idx == 0 {
            MPEGVersion::MPEG25
        } else if ver_idx & 0x01 == 0x01 {
            MPEGVersion::MPEG1
        } else {
            MPEGVersion::MPEG2
        };
        m_frame_header.layer = match LayerIndex::from_u8(4 - ((buf[1] >> 1) & 0x03)) {
            Ok(v) => v,
            Err(_) => { return Err(ERR_MP3_INVALID_FRAMEHEADER); }
        }; /* easy mapping of index to layer number, 4 = error */
        m_frame_header.crc = 1 - ((buf[1] as i32 >> 0) & 0x01);
        m_frame_header.br_idx = match BitrateIndex::from_u8((buf[2] >> 4) & 0x0f) {
            Ok(v) => v,
            Err(_) => return Err(ERR_MP3_INVALID_FRAMEHEADER),
        };
        m_frame_header.sr_idx = match SampleRateIndex::from_u8((buf[2] >> 2) & 0x03) {
            Ok(v) => v,
            Err(_) => {return Err(ERR_MP3_INVALID_FRAMEHEADER); }
        };
        m_frame_header.paddingBit = (buf[2] as i32 >> 1) & 0x01;
        m_frame_header.privateBit = (buf[2] as i32 >> 0) & 0x01;
        self.m_sMode = match (buf[3] >> 6) & 0x03 {
            0x00 => StereoMode::Stereo,
            0x01 => StereoMode::Joint,
            0x02 => StereoMode::Dual,
            0x03 => StereoMode::Mono,
            _ => return Err(ERR_MP3_INVALID_FRAMEHEADER),
        }; /* maps to correct enum (see definition) */
        m_frame_header.modeExt = (buf[3] as usize >> 4) & 0x03;
        m_frame_header.copyFlag = (buf[3] as i32 >> 3) & 0x01;
        m_frame_header.origFlag = (buf[3] as i32 >> 2) & 0x01;
        m_frame_header.emphasis = (buf[3] as i32 >> 0) & 0x03;
        /* check parameters to avoid indexing tables with bad values */
        /* for readability (we reference sfBandTable many times in decoder) */
        self.m_SFBandTable =
            SF_BAND_TABLE[self.m_MPEGVersion as usize][m_frame_header.sr_idx as usize];
        if self.m_sMode != StereoMode::Joint {
            /* just to be safe (dequant, stproc check fh->modeExt) */
            m_frame_header.modeExt = 0;
        }
        /* init user-accessible data */
        m_mp3_dec_info.nChans = self.m_sMode.get_channel_count();
        m_mp3_dec_info.samprate =
            SAMPLERATE_TAB[self.m_MPEGVersion as usize][m_frame_header.sr_idx as usize];
        m_mp3_dec_info.nGrans = if self.m_MPEGVersion == MPEGVersion::MPEG1 {
            GranuleCount::Mpeg1Granule
        } else {
            GranuleCount::Mpeg2Granule
        };
        m_mp3_dec_info.nGranSamps = (SAMPLES_PER_FRAME_TAB[self.m_MPEGVersion as usize]
            [(m_frame_header.layer as usize - 1) as usize])
            / m_mp3_dec_info.nGrans as i32;
        m_mp3_dec_info.layer = m_frame_header.layer;

        /* get bitrate and nSlots from table, unless brIdx == 0 (free mode) in which case caller must figure it out himself
         * question - do we want to overwrite mp3DecInfo->bitrate with 0 each time if it's free mode, and
         *  copy the pre-calculated actual free bitrate into it in mp3dec.c (according to the spec,
         *  this shouldn't be necessary, since it should be either all frames free or none free)
         */
        if m_frame_header.br_idx != BitrateIndex::Bitrate0 {
            m_mp3_dec_info.bitrate = (BITRATE_TAB[self.m_MPEGVersion as usize]
                [m_frame_header.layer as usize - 1][m_frame_header.br_idx as usize])
                as i32
                * 1000;
            /* nSlots = total frame bytes (from table) - sideInfo bytes - header - CRC (if present) + pad (if present) */
            m_mp3_dec_info.nSlots = SLOT_TAB[self.m_MPEGVersion as usize]
                [m_frame_header.sr_idx as usize][m_frame_header.br_idx as usize]
                as i32
                - SIDE_BYTES_TAB[self.m_MPEGVersion as usize][if self.m_sMode == StereoMode::Mono {
                    0
                } else {
                    1
                }]
                - 4
                - (if m_frame_header.crc != 0 { 2 } else { 0 })
                + (if m_frame_header.paddingBit != 0 { 1 } else { 0 });
        }
        /* load crc word, if enabled, and return length of frame header (in bytes) */
        if m_frame_header.crc != 0 {
            if buf.len() < 6 {
                return Err(ERR_MP3_INVALID_FRAMEHEADER);
            }
            m_frame_header.CRCWord = (buf[4] as i32) << 8 | (buf[5] as i32) << 0;
            return Ok(6);
        } else {
            m_frame_header.CRCWord = 0;
            return Ok(4);
        }
    }

    /***********************************************************************************************************************
     * Function:    MP3GetLastFrameInfo
     *
     * Description: get info about last MP3 frame decoded (number of sampled decoded,
     *                sample rate, bitrate, etc.)
     *
     * Inputs:
     *
     * Outputs:     filled-in MP3FrameInfo struct
     *
     * Return:      none
     *
     * Notes:       call this right after calling MP3Decode
     **********************************************************************************************************************/

    pub fn mp3_get_last_frame_info(&mut self) {
        if self.m_MP3DecInfo.layer != LayerIndex::Layer3 {
            self.m_MP3FrameInfo.bitrate = 0;
            self.m_MP3FrameInfo.n_chans = ChannelCount::SingleChannel;
            self.m_MP3FrameInfo.samprate = 0;
            self.m_MP3FrameInfo.bitsPerSample = 0;
            self.m_MP3FrameInfo.outputSamps = 0;
            self.m_MP3FrameInfo.layer = LayerIndex::Layer1;
            self.m_MP3FrameInfo.version = MPEGVersion::MPEG1;
        } else {
            self.m_MP3FrameInfo.bitrate = self.m_MP3DecInfo.bitrate;
            self.m_MP3FrameInfo.n_chans = self.m_MP3DecInfo.nChans;
            self.m_MP3FrameInfo.samprate = self.m_MP3DecInfo.samprate;
            self.m_MP3FrameInfo.bitsPerSample = 16;
            self.m_MP3FrameInfo.outputSamps = self.m_MP3DecInfo.nChans as i32
                * SAMPLES_PER_FRAME_TAB[self.m_MPEGVersion as usize]
                    [self.m_MP3DecInfo.layer as usize - 1] as i32;
            self.m_MP3FrameInfo.layer = self.m_MP3DecInfo.layer;
            self.m_MP3FrameInfo.version = self.m_MPEGVersion;
        }
    }

    pub fn unpack_side_info(&mut self, buf: &[u8]) -> usize {
        let n_bytes: usize;

        let m_side_info_sub = &mut self.m_SideInfoSub;
        let m_mp3_dec_info = &mut self.m_MP3DecInfo;
        let m_mpegversion = self.m_MPEGVersion;
        let m_s_mode = self.m_sMode;
        let m_side_info = &mut self.m_SideInfo;

        let mut bsi: BitStreamInfo;

        /* validate pointers and sync word */
        if m_mpegversion == MPEGVersion::MPEG1 {
            /* MPEG 1 */
            n_bytes = if m_s_mode == StereoMode::Mono {
                SIBYTES_MPEG1_MONO
            } else {
                SIBYTES_MPEG1_STEREO
            };
            bsi = BitStreamInfo::from_slice(buf);
            m_side_info.main_data_begin = bsi.get_bits(9) as i32;
            m_side_info.private_bits =
                bsi.get_bits(if m_s_mode == StereoMode::Mono { 5 } else { 3 }) as i32;
            for ch in 0..m_mp3_dec_info.nChans as usize {
                for bd in 0..MAX_SCFBD {
                    m_side_info.scfsi[ch][bd] = bsi.get_bits(1) as i32;
                }
            }
        } else {
            /* MPEG 2, MPEG 2.5 */
            n_bytes = if m_s_mode == StereoMode::Mono {
                SIBYTES_MPEG2_MONO
            } else {
                SIBYTES_MPEG2_STEREO
            };
            bsi = BitStreamInfo::from_slice(buf);
            m_side_info.main_data_begin = bsi.get_bits(8) as i32;
            m_side_info.private_bits =
                bsi.get_bits(if m_s_mode == StereoMode::Mono { 1 } else { 2 }) as i32;
        }
        for gr in 0..m_mp3_dec_info.nGrans as usize {
            for ch in 0..m_mp3_dec_info.nChans as usize {
                let sis = &mut m_side_info_sub[gr as usize][ch as usize]; /* side info subblock for this granule, channel */
                sis.part23_length = bsi.get_bits(12) as i32;
                sis.n_bigvals = bsi.get_bits(9) as i32;
                sis.global_gain = bsi.get_bits(8) as i32;
                sis.sf_compress = bsi.get_bits(if m_mpegversion == MPEGVersion::MPEG1 {
                    4
                } else {
                    9
                }) as i32;
                sis.win_switch_flag = bsi.get_bits(1) as i32;
                if sis.win_switch_flag != 0 {
                    /* this is a start, stop, short, or mixed block */
                    sis.blockType = match bsi.get_bits(2) {
                        0 => BlockType::Normal,
                        1 => BlockType::Start,
                        2 => BlockType::Short,
                        3 => BlockType::Stop,
                        _ => unreachable!(""),
                    }; /* 0 = normal, 1 = start, 2 = short, 3 = stop */
                    sis.mixedBlock = bsi.get_bits(1) as i32; /* 0 = not mixed, 1 = mixed */
                    sis.tableSelect[0] = bsi.get_bits(5) as i32;
                    sis.tableSelect[1] = bsi.get_bits(5) as i32;
                    sis.tableSelect[2] = 0; /* unused */
                    sis.subBlockGain[0] = bsi.get_bits(3) as i32;
                    sis.subBlockGain[1] = bsi.get_bits(3) as i32;
                    sis.subBlockGain[2] = bsi.get_bits(3) as i32;
                    if sis.blockType == BlockType::Normal {
                        /* this should not be allowed, according to spec */
                        sis.n_bigvals = 0;
                        sis.part23_length = 0;
                        sis.sf_compress = 0;
                    } else if sis.blockType == BlockType::Short && sis.mixedBlock == 0 {
                        /* short block, not mixed */
                        sis.region0Count = 8;
                    } else {
                        /* start, stop, or short-mixed */
                        sis.region0Count = 7;
                    }
                    sis.region1Count = 20 - sis.region0Count;
                } else {
                    /* this is a normal block */
                    sis.blockType = BlockType::Normal;
                    sis.mixedBlock = 0;
                    sis.tableSelect[0] = bsi.get_bits(5) as i32;
                    sis.tableSelect[1] = bsi.get_bits(5) as i32;
                    sis.tableSelect[2] = bsi.get_bits(5) as i32;
                    sis.region0Count = bsi.get_bits(4) as i32;
                    sis.region1Count = bsi.get_bits(3) as i32;
                }
                sis.preFlag = if m_mpegversion == MPEGVersion::MPEG1 {
                    bsi.get_bits(1) as i32
                } else {
                    0
                };
                sis.sfactScale = bsi.get_bits(1) as i32;
                sis.count1TableSelect = bsi.get_bits(1) as i32;
            }
        }
        m_mp3_dec_info.mainDataBegin = m_side_info.main_data_begin; /* needed by main decode loop */
        // assert(nBytes == CalcBitsUsed(bsi, buf, 0) >> 3);
        n_bytes
    }
}

#[cfg(test)]
mod unpack_frame_header_test {
    use crate::mp3_decoder::{
        BLOCK_SIZE, CriticalBandInfo, DequantInfo, FrameHeader, HuffmanInfo, IMDCTInfo,
        MAINBUF_SIZE, MAX_NCHAN, MAX_NGRAN, MAX_NSAMP, MAX_REORDER_SAMPS, MAX_SCFBD, MP3Decoder,
        MP3FrameInfo, MPEGVersion, NBANDS, SFBandTable, ScaleFactorInfoSub, ScaleFactorJS,
        SideInfo, SideInfoSub, StereoMode, SubbandInfo, VBUF_LENGTH,
    };

    fn make_decoder() -> MP3Decoder {
        let m_FrameHeader = FrameHeader::default();
        let m_MP3DecInfo = super::MP3DecInfo {
            bitrate: 0,
            freeBitrateFlag: 0,
            freeBitrateSlots: 0,
            layer: 0,
            mainBuf: [0; MAINBUF_SIZE],
            mainDataBegin: 0,
            mainDataBytes: 0,
            nChans: ChannelCount::SingleChannel,
            nGranSamps: 0,
            nGrans: 0,
            nSlots: 0,
            part23Length: [[0; MAX_NCHAN]; MAX_NGRAN],
            samprate: 0,
        };
        let m_MP3FrameInfo = MP3FrameInfo {
            bitrate: 0,
            bitsPerSample: 0,
            layer: 0,
            n_chans: 0,
            outputSamps: 0,
            samprate: 0,
            version: MPEGVersion::MPEG1,
        };
        let m_SideInfo = SideInfo {
            main_data_begin: 0,
            private_bits: 0,
            scfsi: [[0; MAX_SCFBD]; MAX_NCHAN],
        };
        let m_SFBandTable = SFBandTable {
            l: [0; 23],
            s: [0; 14],
        };
        let m_SideInfoSub = [[SideInfoSub::default(); MAX_NCHAN]; MAX_NGRAN];
        MP3Decoder {
            m_FrameHeader,
            m_MP3DecInfo,
            m_MP3FrameInfo,
            m_SideInfo,
            m_SFBandTable,
            m_SideInfoSub,
            m_ScaleFactorJS: ScaleFactorJS {
                intensity_scale: 0,
                nr: [0; 4],
                slen: [0; 4],
            },
            m_SubbandInfo: SubbandInfo {
                vbuf: [0; MAX_NCHAN * VBUF_LENGTH],
                vindex: 0,
            },
            m_ScaleFactorInfoSub: [[ScaleFactorInfoSub {
                l: [0; 23],
                s: [[0; 3]; 13],
            }; MAX_NCHAN]; MAX_NGRAN],
            m_CriticalBandInfo: [CriticalBandInfo {
                cbEndL: 0,
                cbEndS: [0; 3],
                cbEndSMax: 0,
                cbType: 0,
            }; MAX_NCHAN],
            m_HuffmanInfo: HuffmanInfo {
                gb: [0; MAX_NCHAN],
                huff_dec_buf: [[0; MAX_NSAMP]; MAX_NCHAN],
                non_zero_bound: [0; MAX_NCHAN],
            },
            m_DequantInfo: DequantInfo {
                work_buf: [0; MAX_REORDER_SAMPS],
            },
            m_IMDCTInfo: IMDCTInfo {
                gb: [0; MAX_NCHAN],
                outBuf: [[[0; NBANDS]; BLOCK_SIZE]; MAX_NCHAN],
                overBuf: [[0; MAX_NSAMP / 2]; MAX_NCHAN],
                numPrevIMDCT: [0; MAX_NCHAN],
                prevType: [0; MAX_NCHAN],
                prevWinSwitch: [0; MAX_NCHAN],
            },
            m_sMode: StereoMode::Stereo,
            m_MPEGVersion: MPEGVersion::MPEG1,
        }
    }
    #[test]
    fn test_unpack_frame() {
        let buf: [u8; 4] = [0xFF, 0xFB, 0x92, 0x64];
        let mut m_MP3Decoder = make_decoder();
        let res = m_MP3Decoder.unpack_frame_header(&buf);

        assert_eq!(m_MP3Decoder.m_MP3DecInfo.bitrate, 128000);
        assert_eq!(res, Ok(4));
    }

    #[test]
    fn test_unpack_frame_mpeg2() {
        let buf: [u8; 6] = [0xFF, 0xF2, 0x20, 0xC4, 0x8E, 0xF6];
        let mut m_MP3Decoder = make_decoder();
        let res = m_MP3Decoder.unpack_frame_header(&buf);
        assert_eq!(m_MP3Decoder.m_MP3DecInfo.bitrate, 16000);
        assert_eq!(m_MP3Decoder.m_MP3DecInfo.samprate, 22050);
        assert_eq!(m_MP3Decoder.m_MPEGVersion, MPEGVersion::MPEG2);
        assert_eq!(res, Ok(6));
    }
}

#[cfg(test)]
mod clip_2n_tests {
    use super::clip_2n;

    #[test]
    fn test_normal_range() {
        // Standard MP3 decoder usage: clip to 16-bit signed range
        assert_eq!(clip_2n(0, 15), 0);
        assert_eq!(clip_2n(32767, 15), 32767);
        assert_eq!(clip_2n(-32768, 15), -32768);

        // Positive overflow
        assert_eq!(clip_2n(32768, 15), 32767);
        assert_eq!(clip_2n(100000, 15), 32767);

        // Negative overflow
        assert_eq!(clip_2n(-32769, 15), -32768);
        assert_eq!(clip_2n(-100000, 15), -32768);
    }

    #[test]
    fn test_different_n_values() {
        assert_eq!(clip_2n(0, 0), 0);
        assert_eq!(clip_2n(1, 0), 0); // max = 0, min = -1 → clips to 0 on positive
        assert_eq!(clip_2n(-1, 0), -1);

        assert_eq!(clip_2n(1023, 10), 1023);
        assert_eq!(clip_2n(1024, 10), 1023);
        assert_eq!(clip_2n(-1024, 10), -1024);
        assert_eq!(clip_2n(-1025, 10), -1024);

        assert_eq!(clip_2n(7, 3), 7);
        assert_eq!(clip_2n(8, 3), 7);
        assert_eq!(clip_2n(-8, 3), -8);
        assert_eq!(clip_2n(-9, 3), -8);
    }

    #[test]
    fn test_bit_exact_match_with_original_helix_macro() {
        // The original CLIP_2N uses the sign-bit XOR trick
        // It clips to [-(1<<n), (1<<n)-1]
        // Verify we match that exactly

        // n=15 → [-32768, 32767]
        assert_eq!(clip_2n(32768, 15), 32767); // 2^15 → clamped to 2^15 - 1
        assert_eq!(clip_2n(i32::MAX, 15), 32767);
        assert_eq!(clip_2n(i32::MIN, 15), -32768);

        // n=1 → [-2, 1]
        assert_eq!(clip_2n(2, 1), 1);
        assert_eq!(clip_2n(-3, 1), -2);
    }

    #[test]
    fn test_extreme_n_values() {
        // n too large → safely clamped to 31
        assert_eq!(clip_2n(i32::MAX, 40), i32::MAX); // n=40 → clamped to 31
        assert_eq!(clip_2n(i32::MIN, 100), i32::MIN);
    }

    #[test]
    fn test_n_equals_31() {
        // Full i32 range except overflow not possible
        assert_eq!(clip_2n(i32::MAX, 31), i32::MAX);
        assert_eq!(clip_2n(i32::MIN, 31), i32::MIN);
        assert_eq!(clip_2n(0, 31), 0);
    }

    #[test]
    fn test_n_equals_0() {
        // Range: [-1, 0]
        assert_eq!(clip_2n(0, 0), 0);
        assert_eq!(clip_2n(1, 0), 0);
        assert_eq!(clip_2n(100, 0), 0);
    }

    #[test]
    fn no_panic_on_invalid_shift() {
        // This test ensures we DON'T panic on large n
        // If you used raw `1 << n` without clamping/wrapping, this would panic in debug
        let _ = clip_2n(123, 40); // Should NOT panic
    }
}
