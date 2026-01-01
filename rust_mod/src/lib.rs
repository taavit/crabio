#![no_std]
#![feature(asm_experimental_arch)]
use core::panic::PanicInfo;

use crabio::mp3_decoder::{
    BitStreamInfo, MAINBUF_SIZE, MAX_NCHAN, MAX_NGRAN, NBANDS, POLY_COEF, VBUF_LENGTH, clip_2n, clip_to_short, fdct_32, freq_invert_rescale, get_bits, idct_9, imdct_12, madd_64, mp3_find_free_sync, mp3_find_sync_word, mulshift_32, polyphase_mono, polyphase_stereo, refill_bitstream_cache, sar_64, win_previous
};

#[repr(C)]
pub struct BitStreamInfoC {
    pub byte_ptr: *const u8, // unsigned char *bytePtr;
    pub i_cache: u32,        // unsigned int iCache;
    pub cached_bits: i32,    // int cachedBits;
    pub n_bytes: i32,        // int nBytes;
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn CLIP_2N(y: i32, n: u32) -> i32 {
    clip_2n(y, n)
}

#[unsafe(no_mangle)]
pub extern "C" fn rust_add(left: u64, right: u64) -> u64 {
    left + right
}

#[unsafe(no_mangle)]
pub extern "C" fn SAR64(x: u64, n: i32) -> u64 {
    sar_64(x, n)
}

#[unsafe(no_mangle)]
pub extern "C" fn xSAR64(x: u64, n: i32) -> u64 {
    sar_64(x, n)
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn MULSHIFT32(x: i32, y: i32) -> i32 {
    mulshift_32(x, y)
}

#[unsafe(no_mangle)]
pub extern "C" fn imdct12(x: *const i32, out: *mut i32) {
    let x_arr: &[i32; 18] = unsafe {
        core::slice::from_raw_parts(x, 18)
            .try_into()
            .unwrap_unchecked()
    };
    let out_arr: &mut [i32; 6] = unsafe {
        core::slice::from_raw_parts_mut(out, 6)
            .try_into()
            .unwrap_unchecked()
    };

    imdct_12(x_arr, out_arr);
}

#[unsafe(no_mangle)]
pub extern "C" fn idct9(x: *mut i32) {
    let x_arr: &mut [i32; 9] = unsafe {
        core::slice::from_raw_parts_mut(x, 9)
            .try_into()
            .unwrap_unchecked()
    };
    idct_9(x_arr);
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn MADD64(sum64: u64, x: i32, y: i32) -> u64 {
    madd_64(sum64, x, y)
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn ClipToShort(x: i32, frac_bits: i32) -> i16 {
    clip_to_short(x, frac_bits)
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn RefillBitstreamCache(bsi_c: *mut BitStreamInfoC) {
    let bsi_c: &mut BitStreamInfoC = unsafe { &mut *bsi_c };
    let data_slice = unsafe { core::slice::from_raw_parts(bsi_c.byte_ptr, bsi_c.n_bytes as usize) };
    let mut bsi_rs = BitStreamInfo {
        bytes: data_slice,
        cache: bsi_c.i_cache,
        cached_bits: bsi_c.cached_bits,
    };

    refill_bitstream_cache(&mut bsi_rs);
    bsi_c.i_cache = bsi_rs.cache;
    let consumed = data_slice.len() - bsi_rs.bytes.len();
    bsi_c.byte_ptr = unsafe { bsi_c.byte_ptr.add(consumed) };
    bsi_c.n_bytes -= consumed as i32;
    bsi_c.cached_bits = bsi_rs.cached_bits;
}

#[unsafe(no_mangle)]
pub extern "C" fn GetBits(bsi_c: *mut BitStreamInfoC, n_bits: u32) -> u32 {
    let bsi_c: &mut BitStreamInfoC = unsafe { &mut *bsi_c };
    let data_slice = unsafe { core::slice::from_raw_parts(bsi_c.byte_ptr, bsi_c.n_bytes as usize) };
    let mut bsi_rs = BitStreamInfo {
        bytes: data_slice,
        cache: bsi_c.i_cache,
        cached_bits: bsi_c.cached_bits,
    };

    let res = get_bits(&mut bsi_rs, n_bits);
    bsi_c.i_cache = bsi_rs.cache;
    let consumed = data_slice.len() - bsi_rs.bytes.len();
    bsi_c.byte_ptr = unsafe { bsi_c.byte_ptr.add(consumed) };
    bsi_c.n_bytes -= consumed as i32;
    bsi_c.cached_bits = bsi_rs.cached_bits;

    res
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn MP3FindSyncWord(buf: *const u8, n_bytes: i32) -> i32 {
    if n_bytes < 2 {
        return -1;
    }

    let data = unsafe { core::slice::from_raw_parts(buf, n_bytes as usize) };

    mp3_find_sync_word(data)
        .map(|tail| unsafe { tail.as_ptr().offset_from(buf) } as i32)
        .unwrap_or(-1)
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe extern "C" fn MP3FindFreeSync(buf: *const u8, first_fh: *const u8, n_bytes: i32) -> i32 {
    let data = unsafe { core::slice::from_raw_parts(buf, n_bytes as usize) };
    let first_header: [u8; 4] = unsafe {
        [
            *first_fh.offset(0),
            *first_fh.offset(1),
            *first_fh.offset(2),
            *first_fh.offset(3),
        ]
    };
    mp3_find_free_sync(data, first_header)
        .map(|off| off as i32)
        .unwrap_or(-1)
}

/***********************************************************************************************************************
 * P O L Y P H A S E
 **********************************************************************************************************************/

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
#[allow(non_snake_case)]
pub fn PolyphaseStereo(pcm: *mut i16, vbuf: *const i32, coef_base: *const u32) {
    let pcm = unsafe { core::slice::from_raw_parts_mut(pcm, NBANDS * MAX_NCHAN) }; // 32*2
    let vbuf = unsafe { core::slice::from_raw_parts(vbuf, VBUF_LENGTH) };
    let coef_base = unsafe { core::slice::from_raw_parts(coef_base, POLY_COEF.len()) };

    polyphase_stereo(pcm, vbuf, coef_base);
}

/***********************************************************************************************************************
 * Function:    PolyphaseMono
 *
 * Description: filter one subband and produce 32 output PCM samples for one channel
 *
 * Inputs:      pointer to PCM output buffer
 *              number of "extra shifts" (vbuf format = Q(DQ_FRACBITS_OUT-2))
 *              pointer to start of vbuf (preserved from last call)
 *              start of filter coefficient table (in proper, shuffled order)
 *              no minimum number of guard bits is required for input vbuf
 *                (see additional scaling comments below)
 *
 * Outputs:     32 samples of one channel of decoded PCM data, (i.e. Q16.0)
 *
 * Return:      none
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe fn PolyphaseMono(pcm: *mut i16, vbuf: *const i32, coef_base: *const u32) {
    let pcm = unsafe { core::slice::from_raw_parts_mut(pcm, NBANDS * MAX_NCHAN) }; // 32*2
    let vbuf = unsafe { core::slice::from_raw_parts(vbuf, VBUF_LENGTH) };
    let coef_base = unsafe { core::slice::from_raw_parts(coef_base, POLY_COEF.len()) };

    polyphase_mono(pcm, vbuf, coef_base);
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn FDCT32(buf: *mut i32, dest: *mut i32, offset: i32, odd_block: i32, gb: i32) {
    let buf_slice = unsafe { core::slice::from_raw_parts_mut(buf, 32) };
    let dest_slice = unsafe { core::slice::from_raw_parts_mut(dest, VBUF_LENGTH * 2) };

    fdct_32(buf_slice, dest_slice, offset, odd_block, gb);
}

/***********************************************************************************************************************
 * Function:    FreqInvertRescale
 *
 * Description: do frequency inversion (odd samples of odd blocks) and rescale
 *                if necessary (extra guard bits added before IMDCT)
 *
 * Inputs:      output vector y (18 new samples, spaced NBANDS apart)
 *              previous sample vector xPrev (9 samples)
 *              index of current block
 *              number of extra shifts added before IMDCT (usually 0)
 *
 * Outputs:     inverted and rescaled (as necessary) outputs
 *              rescaled (as necessary) previous samples
 *
 * Return:      updated mOut (from new outputs y)
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe fn FreqInvertRescale(
    y: *mut i32,
    x_prev: *mut i32,
    block_idx: i32,
    es: i32,
) -> i32 {
    let y_slice = unsafe { core::slice::from_raw_parts_mut(y, 9 * NBANDS * 2 + NBANDS) };
    let x_prev: &mut [i32] = unsafe { core::slice::from_raw_parts_mut(x_prev, 9) };
    
    freq_invert_rescale(y_slice, x_prev, block_idx, es)
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn WinPrevious(xPrev: *mut i32, xPrevWin: *mut i32, bt_prev: i32) {
    let x_prev: &mut [i32; 9] = unsafe { &mut *xPrev.cast::<[i32; 9]>() };
    let x_prev_win: &mut [i32; 18] = unsafe { &mut *xPrevWin.cast::<[i32; 18]>() };
    
    win_previous(x_prev, x_prev_win, bt_prev);
}


#[repr(C)]
#[allow(non_snake_case)]
pub struct FrameHeader {
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

#[repr(C)]
#[allow(non_snake_case)]
pub struct MP3DecInfo {
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
    part23Length: [[i32; MAX_NCHAN]; MAX_NGRAN],
}

pub const SAMPLERATE_TAB: [[i32; 3]; 3] = [
        [ 44100, 48000, 32000 ], /* MPEG-1 */
        [ 22050, 24000, 16000 ], /* MPEG-2 */
        [ 11025, 12000, 8000  ], /* MPEG-2.5 */
];

/* indexing = [version][mono/stereo]
 * number of bytes in side info section of bitstream
 */
const sideBytesTab: [[i32; 2]; 3] = [
    [ 17, 32 ], /* MPEG-1:   mono, stereo */
    [ 9, 17 ], /* MPEG-2:   mono, stereo */
    [ 9, 17 ], /* MPEG-2.5: mono, stereo */
];

/* indexing = [version][sampleRate][long (.l) or short (.s) block]
 *   sfBandTable[v][s].l[cb] = index of first bin in critical band cb (long blocks)
 *   sfBandTable[v][s].s[cb] = index of first bin in critical band cb (short blocks)
 */
const sfBandTable: [[SFBandTable; 3]; 3] = [
    [ /* MPEG-1 (44, 48, 32 kHz) */
        SFBandTable {
            l: [0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 52,  62,  74,  90, 110, 134, 162, 196, 238, 288, 342, 418, 576 ],
            s: [0, 4, 8, 12, 16, 22, 30, 40, 52, 66, 84, 106, 136, 192]
        },
        SFBandTable {
            l: [0, 4, 8, 12, 16, 20, 24, 30, 36, 42, 50,  60,  72,  88, 106, 128, 156, 190, 230, 276, 330, 384, 576 ],
            s: [0, 4, 8, 12, 16, 22, 28, 38, 50, 64, 80, 100, 126, 192]
        },
        SFBandTable {
            l: [0, 4, 8, 12, 16, 20, 24, 30, 36, 44,  54,  66,  82, 102, 126, 156, 194, 240, 296, 364, 448, 550, 576 ],
            s: [0, 4, 8, 12, 16, 22, 30, 42, 58, 78, 104, 138, 180, 192]
        }
    ],
    [ /* MPEG-2 (22, 24, 16 kHz) */
        SFBandTable   {
            l: [0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 ],
            s: [0, 4,  8, 12, 18, 24, 32, 42, 56, 74, 100, 132, 174, 192]
        },
        SFBandTable {
            l: [0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 114, 136, 162, 194, 232, 278, 332, 394, 464, 540, 576 ],
            s: [0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 136, 180, 192]
        },
        SFBandTable {
            l: [0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80, 96,  116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 ],
            s: [0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192]
        },
    ],
    [ /* MPEG-2.5 (11, 12, 8 kHz) */
        SFBandTable {
            l :[0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 ],
            s: [0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192 ]
        },
        SFBandTable {
            l: [0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 ],
            s: [0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192 ]
        },
        SFBandTable {
            l: [0, 12, 24, 36, 48, 60, 72, 88, 108, 132, 160, 192, 232, 280, 336, 400, 476, 566, 568, 570, 572, 574, 576 ],
            s: [0,  8, 16, 24, 36, 52, 72, 96, 124, 160, 162, 164, 166, 192]
        }
    ],
];


pub enum MPEGVersion {          /* map to 0,1,2 to make table indexing easier */
    MPEG1 =  0,
    MPEG2 =  1,
    MPEG25 = 2
}

#[repr(C)]
pub struct MP3FrameInfo {
    bitrate: i32,
    nChans: i32,
    samprate: i32,
    bitsPerSample: i32,
    outputSamps: i32,
    layer: i32,
    version: i32,
}

#[repr(C)]
pub struct SFBandTable {
    l: [i32; 23],
    s: [i32; 14],
}

#[repr(C)]
enum StereoMode_t {          /* map these to the corresponding 2-bit values in the frame header */
    Stereo = 0x00,      /* two independent channels, but L and R frames might have different # of bits */
    Joint = 0x01,       /* coupled channels - layer III: mix of M-S and intensity, Layers I/II: intensity and direct coding only */
    Dual = 0x02,        /* two independent channels, L and R always have exactly 1/2 the total bitrate */
    Mono = 0x03         /* one channel */
}

