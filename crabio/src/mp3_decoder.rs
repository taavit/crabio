pub const SYNCWORDH: u8              =0xff;
pub const SYNCWORDL: u8              =0xf0;
pub const DQ_FRACBITS_OUT: u8        =25;  // number of fraction bits in output of dequant
pub const CSHIFT: u8                 =12;  // coefficients have 12 leading sign bits for early-terminating mulitplies
pub const SIBYTES_MPEG1_MONO: u8     =17;
pub const SIBYTES_MPEG1_STEREO: u8   =32;
pub const SIBYTES_MPEG2_MONO: u8     =9;
pub const SIBYTES_MPEG2_STEREO: u8   =17;
pub const IMDCT_SCALE: u8            =2;   // additional scaling (by sqrt(2)) for fast IMDCT36
pub const NGRANS_MPEG1: u8           =2;
pub const NGRANS_MPEG2: u8           =1;
pub const SQRTHALF: u32               =0x5a82799a;  // sqrt(0.5) in Q31 format

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


#[inline]
pub fn sar_64(x: u64, n: i32) -> u64 {
    x >> n
}

#[inline]
pub fn mulshift_32(x: i32, y: i32) -> i32 {
    ((x as u64) * (y as u64) >> 32) as i32
}
#[inline]
pub fn madd_64(sum64: u64, x: i32, y: i32) -> u64 {
    sum64 + (x as u64) * (y as u64)
}/* returns 64-bit value in [edx:eax] */

/// 12-point IMDCT for MP3 short blocks (fixed-point)
///
/// Input:  exactly 18 coefficients (only indices 0,3,6,9,12,15 used)
/// Output: exactly 6 time-domain samples
#[inline(always)]
pub fn imdct_12(x: &[i32; 18], out: &mut [i32; 6]) {
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
    let a1 = t1 + (x5 >> 1);  // note: x5, not t5 — original uses original x5
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

    let a8  = a6 - a5; // x8 - x4
    let a9  = a3 - a7; // x5 - x7
    let a10 = a2 - x7; // x1 - x5 - x7
    let a11 = a4 - x8; // x2 - x4 - x8

    // Multiplies with precomputed constants
    let m1  = mulshift_32(C9_0, x3);
    let m3  = mulshift_32(C9_0, a10);
    let m5  = mulshift_32(C9_1, a5);
    let m6  = mulshift_32(C9_2, a6);
    let m7  = mulshift_32(C9_1, a8);
    let m8  = mulshift_32(C9_2, a5);
    let m9  = mulshift_32(C9_3, a9);
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
#[inline(always)]
pub fn clip_to_short(mut x: i32, frac_bits: i32) -> i16 {
    x >>= frac_bits;

    #[cfg(target_arch = "xtensa")]
    unsafe {
        core::arch::asm!(
            "clamps {0}, {0}, 15",
            inout(reg) x,
            options(nomem, nostack, preserves_flags),
        );
    }

    #[cfg(not(target_arch = "xtensa"))]
    {
        x = x.clamp(i16::MIN as i32, i16::MAX as i32);
    }

    x as i16
}


pub const HUFF_PAIRTABS: u8          = 32;
pub const BLOCK_SIZE: usize          = 18;
pub const NBANDS: usize              = 32;
pub const MAX_REORDER_SAMPS: usize   = (192-126)*3;      // largest critical band for short blocks (see sfBandTable)
pub const VBUF_LENGTH: usize         = 17*2* NBANDS;    // for double-sized vbuf FIFO
pub const MAX_SCFBD: usize           = 4;     // max scalefactor bands per channel
pub const MAINBUF_SIZE: usize        = 1940;
pub const MAX_NGRAN: usize           = 2;     // max granules
pub const MAX_NCHAN: usize           = 2;     // max channels
pub const MAX_NSAMP: usize           = 576;   // max samples per channel, per granule


const ERR_MP3_NONE: i8 =                  0;
const ERR_MP3_INDATA_UNDERFLOW: i8 =     -1;
const ERR_MP3_MAINDATA_UNDERFLOW: i8 =   -2;
const ERR_MP3_FREE_BITRATE_SYNC: i8 =    -3;
const ERR_MP3_OUT_OF_MEMORY: i8 =        -4;
const ERR_MP3_NULL_POINTER: i8 =         -5;
const ERR_MP3_INVALID_FRAMEHEADER: i8 =  -6;
const ERR_MP3_INVALID_SIDEINFO: i8 =     -7;
const ERR_MP3_INVALID_SCALEFACT: i8 =    -8;
const ERR_MP3_INVALID_HUFFCODES: i8 =    -9;
const ERR_MP3_INVALID_DEQUANTIZE: i8 =   -10;
const ERR_MP3_INVALID_IMDCT: i8 =        -11;
const ERR_MP3_INVALID_SUBBAND: i8 =      -12;
const ERR_UNKNOWN: i8 =                  -127;


pub struct MP3FrameInfo {
    bitrate: i32,
    nChans: i32,
    samprate: i32,
    bitsPerSample: i32,
    outputSamps: i32,
    layer: i32,
    version: i32,
}

struct SFBandTable {
    l: [i32; 23],
    s: [i32; 14],
}

pub struct BitStreamInfo<'a> {
    pub bytes: &'a [u8],
    pub cache: u32,
    pub cached_bits: i32,
}

#[repr(C)]
enum StereoMode {          /* map these to the corresponding 2-bit values in the frame header */
    Stereo = 0x00,      /* two independent channels, but L and R frames might have different # of bits */
    Joint = 0x01,       /* coupled channels - layer III: mix of M-S and intensity, Layers I/II: intensity and direct coding only */
    Dual = 0x02,        /* two independent channels, L and R always have exactly 1/2 the total bitrate */
    Mono = 0x03         /* one channel */
}

#[repr(C)]
enum MPEGVersion {          /* map to 0,1,2 to make table indexing easier */
    MPEG1 =  0,
    MPEG2 =  1,
    MPEG25 = 2
}

struct FrameHeader {
    layer: i32,              /* layer index (1, 2, or 3) */
    crc: i32,                /* CRC flag: 0 = disabled, 1 = enabled */
    brIdx: i32,              /* bitrate index (0 - 15) */
    srIdx: i32,              /* sample rate index (0 - 2) */
    paddingBit: i32,         /* padding flag: 0 = no padding, 1 = single pad byte */
    privateBit: i32,         /* unused */
    modeExt: i32,            /* used to decipher joint stereo mode */
    copyFlag: i32,           /* copyright flag: 0 = no, 1 = yes */
    origFlag: i32,           /* original flag: 0 = copy, 1 = original */
    emphasis: i32,           /* deemphasis mode */
    CRCWord: i32,            /* CRC word (16 bits, 0 if crc not enabled) */
}

struct SideInfoSub {
    part23Length: i32,       /* number of bits in main data */
    nBigvals: i32,           /* 2x this = first set of Huffman cw's (maximum amplitude can be > 1) */
    globalGain: i32,         /* overall gain for dequantizer */
    sfCompress: i32,         /* unpacked to figure out number of bits in scale factors */
    winSwitchFlag: i32,      /* window switching flag */
    blockType: i32,          /* block type */
    mixedBlock: i32,         /* 0 = regular block (all short or long), 1 = mixed block */
    tableSelect: [i32; 3],     /* index of Huffman tables for the big values regions */
    subBlockGain: [i32; 3],    /* subblock gain offset, relative to global gain */
    region0Count: i32,       /* 1+region0Count = num scale factor bands in first region of bigvals */
    region1Count: i32,       /* 1+region1Count = num scale factor bands in second region of bigvals */
    preFlag: i32,            /* for optional high frequency boost */
    sfactScale: i32,         /* scaling of the scalefactors */
    count1TableSelect: i32,  /* index of Huffman table for quad codewords */
}

struct SideInfo {
    mainDataBegin: i32,
    privateBits: i32,
    scfsi: [[i32; MAX_NCHAN]; MAX_SCFBD],                /* 4 scalefactor bands per channel */
}

struct CriticalBandInfo {
    cbType: i32,             /* pure long = 0, pure short = 1, mixed = 2 */
    cbEndS: [i32; 3],          /* number nonzero short cb's, per subbblock */
    cbEndSMax: i32,          /* max of cbEndS[] */
    cbEndL: i32,             /* number nonzero long cb's  */
}

struct DequantInfo {
    work_buf: [i32; MAX_REORDER_SAMPS],             /* workbuf for reordering short blocks */
}

struct HuffmanInfo {
    huff_dec_buf: [[i32; MAX_NCHAN]; MAX_NSAMP],       /* used both for decoded Huffman values and dequantized coefficients */
    non_zero_bound: [i32; MAX_NCHAN],                /* number of coeffs in huffDecBuf[ch] which can be > 0 */
    gb: [i32; MAX_NCHAN],                          /* minimum number of guard bits in huffDecBuf[ch] */
}

enum HuffTabType {
    NoBits,
    OneShot,
    LoopNoLinbits,
    LoopLinbits,
    QuadA,
    QuadB,
    InvalidTab
}

struct HuffTabLookup {
    linBits: i32,
    tabType: i32, /*HuffTabType*/
}

struct IMDCTInfo {
    outBuf: [[[i32; MAX_NCHAN]; BLOCK_SIZE]; NBANDS],  /* output of IMDCT */
    overBuf: [[i32; MAX_NCHAN]; MAX_NSAMP / 2],      /* overlap-add buffer (by symmetry, only need 1/2 size) */
    numPrevIMDCT: [i32; MAX_NCHAN],                /* how many IMDCT's calculated in this channel on prev. granule */
    prevType: [i32; MAX_NCHAN],
    prevWinSwitch: [i32; MAX_NCHAN],
    gb: [i32; MAX_NCHAN],
}

struct BlockCount {
    nBlocksLong: i32,
    nBlocksTotal: i32,
    nBlocksPrev: i32,
    prevType: i32,
    prevWinSwitch: i32,
    currWinSwitch: i32,
    gbIn: i32,
    gbOut: i32,
}

struct ScaleFactorInfoSub {    /* max bits in scalefactors = 5, so use char's to save space */
    l: [u8; 23],            /* [band] */
    s: [[u8; 13]; 3],         /* [band][window] */
}

struct ScaleFactorJS { /* used in MPEG 2, 2.5 intensity (joint) stereo only */
    intensity_scale: i32,
    slen: [i32; 4],
    nr: [i32; 4],
}

/* NOTE - could get by with smaller vbuf if memory is more important than speed
 *  (in Subband, instead of replicating each block in FDCT32 you would do a memmove on the
 *   last 15 blocks to shift them down one, a hardware style FIFO)
 */
struct SubbandInfo {
    vbuf: [i32; MAX_NCHAN * VBUF_LENGTH],      /* vbuf for fast DCT-based synthesis PQMF - double size for speed (no modulo indexing) */
    vindex: i32,                             /* internal index for tracking position in vbuf */
}

struct MP3DecInfo {
    /* buffer which must be large enough to hold largest possible main_data section */
    main_buf: [u8; MAINBUF_SIZE],
    /* special info for "free" bitrate files */
    free_bitrate_flag: i32,
    free_bitrate_slots: i32,
    /* user-accessible info */
    bitrate: i32,
    n_chans: i32,
    samprate: i32,
    n_grans: i32,             /* granules per frame */
    n_gran_samps: i32,         /* samples per granule */
    n_slots: i32,
    layer: i32,

    main_data_begin: i32,
    main_data_bytes: i32,
    part23_length: [[i32; MAX_NGRAN]; MAX_NCHAN],
}

pub fn get_bits(bsi: &mut BitStreamInfo<'_>, mut n_bits: u32) -> u32 {
    n_bits = n_bits.min(32);

    // Special case: requesting 0 bits
    if n_bits == 0 {
        return 0;
    }

    // Extract top n_bits from current cache
    let mut data = bsi.cache.wrapping_shr(32 - n_bits);

    // Consume the bits we just read
    bsi.cache = bsi.cache.wrapping_shl(n_bits);
    bsi.cached_bits -= n_bits as i32;

    // If we went negative → we crossed a 32-bit boundary → need to refill
    if bsi.cached_bits < 0 {
        let needed = (-bsi.cached_bits) as u32;  // positive amount needed from new cache

        refill_bitstream_cache(bsi);

        // OR in up to 'needed' bits from the freshly loaded cache
        let available = bsi.cached_bits.max(0) as u32;
        let take = needed.min(available);

        if take > 0 {
            data |= bsi.cache.wrapping_shr(32 - take);

            bsi.cache = bsi.cache.wrapping_shl(take);
            bsi.cached_bits -= take as i32;
        }
        // If no more data (EOF), low bits stay 0 — correct behavior
    }

    data
}

pub fn refill_bitstream_cache(bsi: &mut BitStreamInfo<'_>) {
    let len = bsi.bytes.len();
    if len == 0 {
        bsi.cache = 0;
        bsi.cached_bits = 0;
    } else if len >= 4 {
        bsi.cache = u32::from_be_bytes([bsi.bytes[0], bsi.bytes[1], bsi.bytes[2], bsi.bytes[3]]);
        bsi.cached_bits = 32;
        bsi.bytes = &bsi.bytes[4..];
    } else {
        bsi.cache = 0u32;
        for &byte in bsi.bytes {
            bsi.cache = (bsi.cache << 8) | (byte as u32);
        }
        let shift = 8 * (4-len);
        bsi.cache = bsi.cache << shift;
        bsi.cached_bits = (8 * len) as i32;
        bsi.bytes = &[];
    }
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
pub fn mp3_find_sync_word(data: &[u8]) -> Option<&[u8]> {
    data.windows(2)
        .position(|w| w[0] == SYNCWORDH && (w[1] & SYNCWORDL) == SYNCWORDL)
        .map(|pos| &data[pos..])
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
pub fn mp3_find_free_sync(buf: &[u8], first_header: [u8; 4]) -> Option<usize> {
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
pub fn polyphase_stereo(mut pcm: &mut [i16], vbuf: &[i32], coef_base: &[u32]) {
    let rnd_val = 1 << ((DQ_FRACBITS_OUT - 2 - 2 - 15) - 1 + (32 - CSHIFT));

    /* special case, output sample 0 */
    let mut coef = coef_base;
    let vb1 = vbuf;
    let mut sum1_r: u64 = rnd_val;
    let mut sum1_l: u64 = rnd_val;
    let mut sum2_r: u64 ;
    let mut sum2_l: u64 ;
    let mut c1: u32;
    let mut c2: u32;
    let mut v_lo: i32;
    let mut v_hi: i32;

    for j in 0..8 {
        c1 = coef[0];
        coef = &coef[1..];
        c2=coef[0];
        coef = &coef[1..];
        v_lo=vb1[j];
        v_hi=vb1[23-j];
        sum1_l = madd_64(
            sum1_l as u64,
            v_lo,
            c1 as i32
        );
        sum1_l = madd_64(sum1_l as u64, v_hi, -(c2 as i32));
        v_lo=vb1[32+j];
        v_hi=vb1[32+(23-j)];
        sum1_r=madd_64(sum1_r as u64, v_lo,  c1 as i32);
        sum1_r=madd_64(sum1_r as u64, v_hi, -(c2 as i32));
    }

    pcm[0] = clip_to_short(sar_64(sum1_l as u64, (32 - CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
    pcm[1] = clip_to_short(sar_64(sum1_r as u64, (32 - CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);

    /* special case, output sample 16 */
    coef = &coef_base[256..];
    let mut vb1 = &vbuf[64*16..];
    sum1_l = rnd_val;
    sum1_r = rnd_val;

    for j in 0..8 {
        c1 = coef[0];
        coef = &coef[1..];
        v_lo = vb1[j];
        sum1_l = madd_64(sum1_l as u64, v_lo,  c1 as i32);
        v_lo = vb1[32+j];
        sum1_r = madd_64(sum1_r as u64, v_lo,  c1 as i32);
    }
    pcm[2*16 + 0] = clip_to_short(sar_64(sum1_l as u64, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
    pcm[2*16 + 1] = clip_to_short(sar_64(sum1_r as u64, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);

    /* main convolution loop: sum1L = samples 1, 2, 3, ... 15   sum2L = samples 31, 30, ... 17 */
    coef = &coef_base[16..];
    vb1 = &vbuf[64..];
    pcm = &mut pcm[2..];

    /* right now, the compiler creates bad asm from this... */
    for i in (1..=15).rev() {
        sum1_l = rnd_val;
        sum2_l = rnd_val;
        sum1_r = rnd_val;
        sum2_r = rnd_val;

        for j in 0..8 {
            c1=coef[0];
            coef = &coef[1..];
            c2=coef[0];
            coef = &coef[1..];
            v_lo=vb1[j];
            v_hi = vb1[23-j];
            sum1_l=madd_64(sum1_l as u64, v_lo,  c1 as i32);
            sum2_l=madd_64(sum2_l as u64, v_lo,  c2 as i32);

            sum1_l=madd_64(sum1_l as u64, v_hi, -(c2 as i32));
            sum2_l=madd_64(sum2_l as u64, v_hi,  c1 as i32);

            v_lo=vb1[32+j];
            v_hi=vb1[32+23-(j)];
            sum1_r= madd_64(sum1_r as u64, v_lo,  c1 as i32);
            sum2_r= madd_64(sum2_r as u64, v_lo,  c2 as i32);
            sum1_r= madd_64(sum1_r as u64, v_hi, -(c2 as i32));
            sum2_r= madd_64(sum2_r as u64, v_hi,  c1 as i32);
        }
        vb1 = &vb1[64..];
        pcm[0]         = clip_to_short(sar_64(sum1_l as u64, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
        pcm[1]         = clip_to_short(sar_64(sum1_r as u64, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
        pcm[2*2*i + 0] = clip_to_short(sar_64(sum2_l as u64, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
        pcm[2*2*i + 1] = clip_to_short(sar_64(sum2_r as u64, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
        pcm = &mut pcm[2..];
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
        c1=coef[0];
        coef = &coef[1..];
        c2=coef[0];
        coef = &coef[1..];
        v_lo=vb1[j];
        v_hi=vb1[23-(j)]; // 0...7
        sum1_l=madd_64(sum1_l, v_lo, c1 as i32); sum1_l=madd_64(sum1_l, v_hi, -(c2 as i32));
    }
    pcm[0] = clip_to_short(sar_64(sum1_l, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);

    /* special case, output sample 16 */
    coef = &coef_base[256..];
    vb1 = &vbuf[64*16..];
    sum1_l = rnd_val;
    for j in 0..8 {
        c1=coef[0];
        coef = &coef[1..];
        v_lo=vb1[j];
        sum1_l = madd_64(sum1_l, v_lo,  c1 as i32); // 0...7
    }
    pcm[16] = clip_to_short(sar_64(sum1_l, (32-CSHIFT) as i32)as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);

    /* main convolution loop: sum1L = samples 1, 2, 3, ... 15   sum2L = samples 31, 30, ... 17 */
    coef = &coef_base[16..];
    vb1 = &vbuf[64..];
    pcm = &mut pcm[1..];

    /* right now, the compiler creates bad asm from this... */
    for i in (1..=15).rev() {
        sum1_l = rnd_val;
        sum2_l = rnd_val;
        for j in 0..8 {
            c1= coef[0];
            coef = &coef[1..];
            c2=coef[0];
            coef = &coef[1..];
            v_lo= vb1[j];
            v_hi = vb1[23-j];
            sum1_l=madd_64(sum1_l, v_lo,  c1 as i32); sum2_l = madd_64(sum2_l, v_lo,  c2 as i32);
            sum1_l=madd_64(sum1_l, v_hi, -(c2 as i32)); sum2_l = madd_64(sum2_l, v_hi,  c1 as i32);
        }
        vb1 = &vb1[64..];
        pcm[0]       = clip_to_short(sar_64(sum1_l, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
        pcm[2*i] = clip_to_short(sar_64(sum2_l, (32-CSHIFT) as i32) as i32, (DQ_FRACBITS_OUT - 2 - 2 - 15) as i32);
        pcm = &mut pcm[1..];
    }
}

#[cfg(test)]
mod tests {
    use crate::mp3_decoder::clip_to_short;

#[test]
    fn test_no_shift_no_clipping() {
        // frac_bits = 0 → no shift, just clipping
        assert_eq!(clip_to_short(0, 0), 0);
        assert_eq!(clip_to_short(100, 0), 100);
        assert_eq!(clip_to_short(-5000, 0), -5000);
        assert_eq!(clip_to_short(32767, 0), 32767);
        assert_eq!(clip_to_short(-32768, 0), -32768);
    }

    #[test]
    fn test_positive_clipping() {
        // Values that exceed i16::MAX after shift should be clamped to 32767
        assert_eq!(clip_to_short(32767 << 1, 1), 32767); // 65534 >> 1 = 32767 → no clip
        assert_eq!(clip_to_short(32768 << 1, 1), 32767); // 65536 >> 1 = 32768 → clipped
        assert_eq!(clip_to_short(1_000_000, 0), 32767);
        assert_eq!(clip_to_short(2_147_483_647, 0), 32767); // i32::MAX
        assert_eq!(clip_to_short(40000, 1), 20000); // 40000 >> 1 = 20000 → no clip?
        assert_eq!(clip_to_short(100_000, 1), 32767); // 100000 >> 1 = 50000 → still > 32767 → clipped
    }

    #[test]
    fn test_negative_clipping() {
        // Values that go below i16::MIN after shift should be clamped to -32768
        assert_eq!(clip_to_short(-32768 << 1, 1), -32768); // -65536 >> 1 = -32768 → no clip
        assert_eq!(clip_to_short((-32768 << 1) - 2, 1), -32768); // -65538 >> 1 = -32769 → clipped
        assert_eq!(clip_to_short(-100_000, 0), -32768);
        assert_eq!(clip_to_short(-2_147_483_648, 0), -32768); // i32::MIN
    }

    #[test]
    fn test_exact_edges() {
        assert_eq!(clip_to_short(32767, 0), 32767);
        assert_eq!(clip_to_short(32768, 0), 32767);
        assert_eq!(clip_to_short(-32768, 0), -32768);
        assert_eq!(clip_to_short(-32769, 0), -32768);

        // After shifting
        assert_eq!(clip_to_short(32767 << 5, 5), 32767);
        assert_eq!(clip_to_short((32767 << 5) + 1, 5), 32767);
        assert_eq!(clip_to_short(-32768 << 5, 5), -32768);
        assert_eq!(clip_to_short((-32768 << 5) - 1, 5), -32768);
    }

    #[test]
    fn test_large_frac_bits() {
        // Shifting by many bits → most values become 0 or -1 (for negative)
        assert_eq!(clip_to_short(12345, 20), 0);
        assert_eq!(clip_to_short(-12345, 20), -1); // arithmetic right shift in Rust for signed
        assert_eq!(clip_to_short(1 << 30, 30), 1);
        assert_eq!(clip_to_short(-1 << 30, 30), -1);
        assert_eq!(clip_to_short(1_000_000, 31), 0);
        assert_eq!(clip_to_short(-1_000_000, 31), -1);
    }

    #[test]
    fn test_frac_bits_zero() {
        // No shift, pure clipping
        for i in -40000..=40000 {
            let expected = i.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            assert_eq!(clip_to_short(i, 0), expected);
        }
    }

    #[test]
    fn test_small_shifts() {
        assert_eq!(clip_to_short(1, 1), 0);
        assert_eq!(clip_to_short(3, 1), 1);
        assert_eq!(clip_to_short(4, 1), 2);
        assert_eq!(clip_to_short(-3, 1), -2);
        // But let's verify actual behavior:
        assert_eq!(clip_to_short(-5, 1), -3);
        // Better to test known values
        assert_eq!(clip_to_short(-1, 0), -1);
        assert_eq!(clip_to_short(-1 << 10, 10), -1);
    }

    mod bitstream {
        use crate::mp3_decoder::{BitStreamInfo, get_bits, refill_bitstream_cache};

        fn make_bsi(data: &[u8]) -> BitStreamInfo<'_> {
        BitStreamInfo {
            bytes: data,
            cache: 0,
            cached_bits: 0,
        }
    }

    #[test]
    fn test_get_single_bits() {
        let data = [0b1011_0011];
        let mut bsi = make_bsi(&data);

        assert_eq!(get_bits(&mut bsi, 1), 1);
        assert_eq!(get_bits(&mut bsi, 1), 0);
        assert_eq!(get_bits(&mut bsi, 1), 1);
        assert_eq!(get_bits(&mut bsi, 1), 1);
        assert_eq!(get_bits(&mut bsi, 1), 0);
        assert_eq!(get_bits(&mut bsi, 1), 0);
        assert_eq!(get_bits(&mut bsi, 1), 1);
        assert_eq!(get_bits(&mut bsi, 1), 1);

        assert_eq!(bsi.cached_bits, 0);
        assert_eq!(bsi.bytes.len(), 0); // consumed 1 byte
    }

    #[test]
    fn test_get_various_sizes_from_cache() {
        let data = [0xAB, 0xCD, 0xEF]; // 0xABCDEF
        let mut bsi = make_bsi(&data);

        // First read 12 bits: should get 0xABC (0b1010_1011_1100)
        assert_eq!(get_bits(&mut bsi, 12), 0xABC);
        // Now cache has 12 bits left: 0xDEF (shifted up)
        // cache = 0xCDEF0000 >> (32-12) wait, let's verify next

        assert_eq!(get_bits(&mut bsi, 8), 0xDE); // next 8 bits
        assert_eq!(get_bits(&mut bsi, 4), 0xF);  // next 4 bits (0xE from 0xEF)
        assert_eq!(get_bits(&mut bsi, 4), 0x0);  // last 4 bits

        assert_eq!(bsi.cached_bits, 0);
    }

    #[test]
    fn test_split_across_refill() {
        let data = [0xF0, 0x0F, 0xAA];
        let mut bsi = make_bsi(&data);

        // Read 4 bits: 0b1111
        assert_eq!(get_bits(&mut bsi, 4), 0xF);
        // Now 4 bits left in cache: 0b0000 (from 0xF0)

        // Read 12 bits: should take remaining 4 (0x0) + next 8 (0x0F) = 0x00F
        assert_eq!(get_bits(&mut bsi, 12), 0x00F);

        // Now cache has 0xAA and possibly more, but we read 8 bits from refill
        assert_eq!(get_bits(&mut bsi, 8), 0xAA);

        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn test_full_32_bits() {
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut bsi = make_bsi(&data);

        assert_eq!(get_bits(&mut bsi, 32), 0x12345678);
        assert_eq!(bsi.cached_bits, 0);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn test_more_than_32_bits_capped() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut bsi = make_bsi(&data);

        // Should cap at 32 and return top 32 bits
        assert_eq!(get_bits(&mut bsi, 40), 0xFFFFFFFF);
        // After reading 32 bits, one byte remains in buffer, cache should be refilled
        refill_bitstream_cache(&mut bsi); // simulate next refill
        assert_eq!(bsi.cache, 0xFF000000);
        assert_eq!(bsi.cached_bits, 8);
    }

    #[test]
    fn test_small_reads_after_large() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut bsi = make_bsi(&data);

        assert_eq!(get_bits(&mut bsi, 16), 0xDEAD);
        assert_eq!(get_bits(&mut bsi, 3), 0b101); // first 3 bits of 0xBE = 0b10111110
        assert_eq!(get_bits(&mut bsi, 5), 0b11110); // next 5 bits
        assert_eq!(get_bits(&mut bsi, 8), 0xEF);
    }

    #[test]
    fn test_empty_stream_returns_zero() {
        let mut bsi = make_bsi(&[]);

        assert_eq!(get_bits(&mut bsi, 1), 0);
        assert_eq!(get_bits(&mut bsi, 16), 0);
        assert_eq!(get_bits(&mut bsi, 32), 0);
        assert_eq!(bsi.cached_bits, 0);
    }

    #[test]
    fn test_partial_final_byte() {
        let data = [0b1010_1010]; // 0b1010_1010
        let mut bsi = make_bsi(&data);

        assert_eq!(get_bits(&mut bsi, 3), 0b101);
        assert_eq!(get_bits(&mut bsi, 3), 0b010);
        assert_eq!(get_bits(&mut bsi, 2), 0b10);
        // Only 8 bits total, no more
        assert_eq!(get_bits(&mut bsi, 1), 0);
    }
    }
}

#[cfg(test)]
mod refill_tests {
    use super::{refill_bitstream_cache, BitStreamInfo};

    fn make_bsi(bytes: &[u8]) -> BitStreamInfo<'_> {
        BitStreamInfo {
            bytes,
            cache: 0xDEADBEEF, // garbage initial value to detect overwrite
            cached_bits: -99,   // invalid initial value
        }
    }

    #[test]
    fn refill_empty() {
        let data = [];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);

        assert_eq!(bsi.cache, 0);
        assert_eq!(bsi.cached_bits, 0);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_full_4_bytes() {
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);

        assert_eq!(bsi.cache, 0x12345678);
        assert_eq!(bsi.cached_bits, 32);
        assert_eq!(bsi.bytes, &[][..]); // consumed 4 bytes
    }

    #[test]
    fn refill_full_more_than_4_bytes() {
        let data = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);

        assert_eq!(bsi.cache, 0xABCDEF12);
        assert_eq!(bsi.cached_bits, 32);
        assert_eq!(bsi.bytes, &[0x34, 0x56][..]); // left 2 bytes
    }

    #[test]
    fn refill_3_bytes() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);

        assert_eq!(bsi.cache, 0xABCDEF00); // left-justified!
        assert_eq!(bsi.cached_bits, 24);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_2_bytes() {
        let data = [0x12, 0x34];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);

        assert_eq!(bsi.cache, 0x12340000);
        assert_eq!(bsi.cached_bits, 16);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_1_byte() {
        let data = [0xFF];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);

        assert_eq!(bsi.cache, 0xFF000000);
        assert_eq!(bsi.cached_bits, 8);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_multiple_calls() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let mut bsi = make_bsi(&data);

        refill_bitstream_cache(&mut bsi);
        assert_eq!(bsi.cache, 0xAABBCCDD);
        assert_eq!(bsi.cached_bits, 32);
        assert_eq!(bsi.bytes, &[0xEE][..]);

        refill_bitstream_cache(&mut bsi);
        assert_eq!(bsi.cache, 0xEE000000);
        assert_eq!(bsi.cached_bits, 8);
        assert_eq!(bsi.bytes.len(), 0);

        refill_bitstream_cache(&mut bsi);
        assert_eq!(bsi.cache, 0);
        assert_eq!(bsi.cached_bits, 0);
    }
}