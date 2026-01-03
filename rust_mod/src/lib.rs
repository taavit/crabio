#![no_std]
#![feature(asm_experimental_arch)]
use core::panic::PanicInfo;

use crabio::mp3_decoder::{
    BitStreamInfo, FrameHeader, MAX_NCHAN,MP3DecInfo, MPEGVersion, NBANDS, POLY_COEF, SFBandTable, StereoMode, VBUF_LENGTH, clip_2n, clip_to_short, fdct_32, freq_invert_rescale, get_bits, idct_9, imdct_12, madd_64, mp3_find_free_sync, mp3_find_sync_word, mulshift_32, polyphase_mono, polyphase_stereo, refill_bitstream_cache, sar_64, unpack_frame_header, win_previous
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

#[unsafe(no_mangle)]
pub unsafe fn UnpackFrameHeader(
    buf: *const u8,
    inbuf_len: usize,
    m_FrameHeader: *mut FrameHeader,
    m_MP3DecInfo: *mut MP3DecInfo,
    m_MPEGVersion: *mut MPEGVersion,
    m_sMode: *mut StereoMode,
    m_SFBandTable: *mut SFBandTable,
) -> i32 {
    let buf = core::slice::from_raw_parts(buf, inbuf_len);
    let m_FrameHeader = unsafe { &mut *m_FrameHeader};
    let m_MP3DecInfo = unsafe { &mut *m_MP3DecInfo};
    let m_MPEGVersion = unsafe { &mut *m_MPEGVersion};
    let m_sMode = unsafe { &mut *m_sMode};
    let m_SFBandTable = unsafe { &mut *m_SFBandTable};
    
    unpack_frame_header(buf, m_FrameHeader, m_MP3DecInfo, m_MPEGVersion, m_sMode, m_SFBandTable)
}

/***********************************************************************************************************************
 * B I T S T R E A M
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
pub unsafe fn SetBitstreamPointer(bsi: *mut BitStreamInfoC, nBytes: i32, buf: *const u8) {
    let bsi = unsafe { &mut *bsi};
    /* init bitstream */
    bsi.byte_ptr = buf;
    bsi.i_cache = 0; /* 4-byte unsigned int */
    bsi.cached_bits = 0; /* i.e. zero bits in cache */
    bsi.n_bytes = nBytes;
}

//----------------------------------------------------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub unsafe fn CalcBitsUsed(bsi: *const BitStreamInfoC, startBuf: *const u8, startOffset: usize) -> i32 {
    let bsi = unsafe { & *bsi};
    let mut bitsUsed = unsafe { (bsi.byte_ptr.offset_from(startBuf) * 8) as i32 };
    bitsUsed -= bsi.cached_bits;
    bitsUsed -= startOffset as i32;
    bitsUsed
}
//----------------------------------------------------------------------------------------------------------------------
#[unsafe(no_mangle)]
pub fn CheckPadBit(m_FrameHeader: *const FrameHeader) -> i32 {
    let m_FrameHeader = unsafe { & *m_FrameHeader};
    if m_FrameHeader.paddingBit != 0 {
        1
     } else {
        0
    }
}