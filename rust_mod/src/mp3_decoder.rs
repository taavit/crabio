pub const polyCoef: [u32; 264] = [
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


const HUFF_PAIRTABS: u8          = 32;
const BLOCK_SIZE: usize          = 18;
const NBANDS: usize              = 32;
const MAX_REORDER_SAMPS: usize   = (192-126)*3;      // largest critical band for short blocks (see sfBandTable)
const VBUF_LENGTH: usize         = 17*2* NBANDS;    // for double-sized vbuf FIFO
const MAX_SCFBD: usize           = 4;     // max scalefactor bands per channel
const MAINBUF_SIZE: usize        = 1940;
const MAX_NGRAN: usize           = 2;     // max granules
const MAX_NCHAN: usize           = 2;     // max channels
const MAX_NSAMP: usize           = 576;   // max samples per channel, per granule


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
    workBuf: [i32; MAX_REORDER_SAMPS],             /* workbuf for reordering short blocks */
}

struct HuffmanInfo {
    huffDecBuf: [[i32; MAX_NCHAN]; MAX_NSAMP],       /* used both for decoded Huffman values and dequantized coefficients */
    nonZeroBound: [i32; MAX_NCHAN],                /* number of coeffs in huffDecBuf[ch] which can be > 0 */
    gb: [i32; MAX_NCHAN],                          /* minimum number of guard bits in huffDecBuf[ch] */
}

enum HuffTabType {
    noBits,
    oneShot,
    loopNoLinbits,
    loopLinbits,
    quadA,
    quadB,
    invalidTab
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
    intensityScale: i32,
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
    mainBuf: [u8; MAINBUF_SIZE],
    /* special info for "free" bitrate files */
    freeBitrateFlag: i32,
    freeBitrateSlots: i32,
    /* user-accessible info */
    bitrate: i32,
    nChans: i32,
    samprate: i32,
    nGrans: i32,             /* granules per frame */
    nGranSamps: i32,         /* samples per granule */
    nSlots: i32,
    layer: i32,

    mainDataBegin: i32,
    mainDataBytes: i32,
    part23Length: [[i32; MAX_NGRAN]; MAX_NCHAN],
}

pub fn get_bits(bsi: &mut BitStreamInfo<'_>, n_bits: u32) -> u32 {
    let n_bits = n_bits.min(32) as i32; // safe cap

    if bsi.cached_bits >= n_bits {
        let data = bsi.cache >> (32 - n_bits);
        bsi.cache <<= n_bits;
        bsi.cached_bits -= n_bits;
        data
    } else {
        let mut data = bsi.cache >> (32 - bsi.cached_bits);
        let needed = n_bits - bsi.cached_bits;

        refill_bitstream_cache(bsi);

        let low = bsi.cache >> (32 - needed);
        data = (data << needed) | low;

        bsi.cache <<= needed;
        bsi.cached_bits -= needed;

        data
    }
}

pub fn refill_bitstream_cache(bsi: &mut BitStreamInfo<'_>) {
    let len = bsi.bytes.len();
    if len >= 4 {
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
