#![no_std]
#![feature(asm_experimental_arch)]
use core::panic::PanicInfo;

use crabio::mp3_decoder::{
    BitStreamInfo, MAX_NCHAN, NBANDS, POLY_COEF, VBUF_LENGTH, clip_2n, clip_to_short, get_bits, idct_9, imdct_12, madd_64, mp3_find_free_sync, mp3_find_sync_word, mulshift_32, polyphase_mono, polyphase_stereo, refill_bitstream_cache, sar_64
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
const FDCT32s1s2: [i32; 16] = [5,3,3,2,2,1,1,1, 1,1,1,1,1,2,2,4];

const m_COS0_0: i32 = 0x4013c251;  /* Q31 */
const m_COS0_1: i32 = 0x40b345bd;  /* Q31 */
const m_COS0_2: i32 = 0x41fa2d6d;  /* Q31 */
const m_COS0_3: i32 = 0x43f93421;  /* Q31 */
const m_COS0_4: i32 = 0x46cc1bc4;  /* Q31 */
const m_COS0_5: i32 = 0x4a9d9cf0;  /* Q31 */
const m_COS0_6: i32 = 0x4fae3711;  /* Q31 */
const m_COS0_7: i32 = 0x56601ea7;  /* Q31 */
const m_COS0_8: i32 = 0x5f4cf6eb;  /* Q31 */
const m_COS0_9: i32 = 0x6b6fcf26;  /* Q31 */
const m_COS0_10: i32= 0x7c7d1db3;  /* Q31 */
const m_COS0_11: i32= 0x4ad81a97;  /* Q30 */
const m_COS0_12: i32= 0x5efc8d96;  /* Q30 */
const m_COS0_13: i32= 0x41d95790;  /* Q29 */
const m_COS0_14: i32= 0x6d0b20cf;  /* Q29 */
const m_COS0_15: i32= 0x518522fb;  /* Q27 */
const m_COS1_0: i32 = 0x404f4672;  /* Q31 */
const m_COS1_1: i32 = 0x42e13c10;  /* Q31 */
const m_COS1_2: i32 = 0x48919f44;  /* Q31 */
const m_COS1_3: i32 = 0x52cb0e63;  /* Q31 */
const m_COS1_4: i32 = 0x64e2402e;  /* Q31 */
const m_COS1_5: i32 = 0x43e224a9;  /* Q30 */
const m_COS1_6: i32 = 0x6e3c92c1;  /* Q30 */
const m_COS1_7: i32 = 0x519e4e04;  /* Q28 */
const m_COS2_0: i32 = 0x4140fb46;  /* Q31 */
const m_COS2_1: i32 = 0x4cf8de88;  /* Q31 */
const m_COS2_2: i32 = 0x73326bbf;  /* Q31 */
const m_COS2_3: i32 = 0x52036742;  /* Q29 */
const m_COS3_0: i32 = 0x4545e9ef;  /* Q31 */
const m_COS3_1: i32 = 0x539eba45;  /* Q30 */
const m_COS4_0: i32 = 0x5a82799a;  /* Q31 */

const M_DCTTAB: [i32; 48] = [
    /* first pass */
     m_COS0_0,  m_COS0_15, m_COS1_0,    /* 31, 27, 31 */
     m_COS0_1,  m_COS0_14, m_COS1_1,    /* 31, 29, 31 */
     m_COS0_2,  m_COS0_13, m_COS1_2,    /* 31, 29, 31 */
     m_COS0_3,  m_COS0_12, m_COS1_3,    /* 31, 30, 31 */
     m_COS0_4,  m_COS0_11, m_COS1_4,    /* 31, 30, 31 */
     m_COS0_5,  m_COS0_10, m_COS1_5,    /* 31, 31, 30 */
     m_COS0_6,  m_COS0_9,  m_COS1_6,    /* 31, 31, 30 */
     m_COS0_7,  m_COS0_8,  m_COS1_7,    /* 31, 31, 28 */
    /* second pass */
     m_COS2_0,  m_COS2_3,  m_COS3_0,   /* 31, 29, 31 */
     m_COS2_1,  m_COS2_2,  m_COS3_1,   /* 31, 31, 30 */
    -m_COS2_0, -m_COS2_3,  m_COS3_0,   /* 31, 29, 31 */
    -m_COS2_1, -m_COS2_2,  m_COS3_1,   /* 31, 31, 30 */
     m_COS2_0,  m_COS2_3,  m_COS3_0,   /* 31, 29, 31 */
     m_COS2_1,  m_COS2_2,  m_COS3_1,   /* 31, 31, 30 */
    -m_COS2_0, -m_COS2_3,  m_COS3_0,   /* 31, 29, 31 */
    -m_COS2_1, -m_COS2_2,  m_COS3_1,   /* 31, 31, 30 */
];

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub unsafe fn FDCT32(buf: *mut i32, dest: *mut i32, offset: i32, odd_block: i32, gb: i32) {
    let buf_slice = core::slice::from_raw_parts_mut(buf, 32);

    let mut es = 0;
    if gb < 6 {
        es = 6 - gb;
        for d in buf_slice.iter_mut() {
            *d >>= es;
        }
    }

    /* first pass */
    for i in 0..8 {
        let a0 = buf_slice[i];
        let a1 = buf_slice[15 - i];
        let a2 = buf_slice[16 + i];
        let a3 = buf_slice[31 - i];

        let b0 = a0 + a3;
        let b3 = mulshift_32(M_DCTTAB[i * 3], a0 - a3) << 1;

        let b1 = a1 + a2;
        let b2 = mulshift_32(M_DCTTAB[i * 3 + 1], a1 - a2) << FDCT32s1s2[i] as i32;

        let coeff = M_DCTTAB[i * 3 + 2]; // shared for next two uses
        buf_slice[i] = b0 + b1;
        buf_slice[15 - i] = mulshift_32(coeff, b0 - b1) << FDCT32s1s2[8 + i] as i32;

        buf_slice[16 + i] = b2 + b3;
        buf_slice[31 - i] = mulshift_32(coeff, b3 - b2) << FDCT32s1s2[8 + i] as i32;
    }

    let cptr_slice_second = &M_DCTTAB[24..];
    /* second pass */
    for (idx, buf_chunk) in buf_slice.chunks_exact_mut(8).enumerate() {
        let a0 = buf_chunk[0];
        let a3 = buf_chunk[3];
        let a4 = buf_chunk[4];
        let a7 = buf_chunk[7];

        let b0 = a0 + a7;
        let b7 = mulshift_32(cptr_slice_second[idx * 6], a0 - a7) << 1;

        let b3 = a3 + a4;
        let b4 = mulshift_32(cptr_slice_second[idx * 6 + 1], a3 - a4) << 3;


        let t0 = b0 + b3;
        let t3 = mulshift_32(cptr_slice_second[idx * 6 + 2], b0 - b3) << 1;

        let t4 = b4 + b7;
        let t7 = mulshift_32(cptr_slice_second[idx * 6 + 2], b7 - b4) << 1;


        let a1 = buf_chunk[1];
        let a6 = buf_chunk[6];
        let a2 = buf_chunk[2];
        let a5 = buf_chunk[5];

        let b1 = a1 + a6;
        let b6 = mulshift_32(cptr_slice_second[idx * 6 + 3], a1 - a6) << 1;

        let b2 = a2 + a5;
        let b5 = mulshift_32(cptr_slice_second[idx * 6 + 4], a2 - a5) << 1;

        let t1 = b1 + b2;
        let t2 = mulshift_32(cptr_slice_second[idx * 6 + 5], b1 - b2) << 2;

        let t5 = b5 + b6;
        let t6 = mulshift_32(cptr_slice_second[idx * 6 + 5], b6 - b5) << 2;

        let bb0 = t0 + t1;
        let bb1 = mulshift_32(m_COS4_0, t0 - t1) << 1;
        let bb2 = t2 + t3;
        let bb3 = mulshift_32(m_COS4_0, t3 - t2) << 1;

        buf_chunk[0] = bb0;
        buf_chunk[1] = bb1;
        buf_chunk[2] = bb2 + bb3;
        buf_chunk[3] = bb3;

        let bb4 = t4 + t5;
        let bb5 = mulshift_32(m_COS4_0, t4 - t5) << 1;
        let bb6 = t6 + t7;
        let bb7 = mulshift_32(m_COS4_0, t7 - t6) << 1;
        let bb6_sum = bb6 + bb7;

        buf_chunk[4] = bb4 + bb6_sum;
        buf_chunk[5] = bb5 + bb7;
        buf_chunk[6] = bb5 + bb6_sum;
        buf_chunk[7] = bb7;
    }

    /* sample 0 - always delayed one block */
    let mut d = dest.add(64 * 16 + ((offset - odd_block) & 7) as usize + if odd_block != 0 { 0 } else { VBUF_LENGTH });
    let s = buf_slice[0];
    *d.add(0) = s;
    *d.add(8) = s;

    /* samples 16 to 31 */
    d = dest.add(offset as usize + if odd_block != 0 { VBUF_LENGTH } else { 0 });

    let mut s = buf_slice[1];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    let mut tmp = buf_slice[25] + buf_slice[29];
    s = buf_slice[17] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[9] + buf_slice[13];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[21] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[29] + buf_slice[27];
    s = buf_slice[5];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[21] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[13] + buf_slice[11];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[19] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[27] + buf_slice[31];
    s = buf_slice[3];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[19] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[11] + buf_slice[15];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[23] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[31];
    s = buf_slice[7];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[23] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[15];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = tmp;
    *d.add(0) = s; *d.add(8) = s;

    /* samples 1 to 16 */
    d = dest.add(16 + ((offset - odd_block) & 7) as usize + if odd_block != 0 { 0 } else { VBUF_LENGTH });

    s = buf_slice[1];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[30] + buf_slice[25];
    s = buf_slice[17] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[14] + buf_slice[9];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[22] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[6];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[26] + buf_slice[30];
    s = buf_slice[22] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[10] + buf_slice[14];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[18] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[2];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[28] + buf_slice[26];
    s = buf_slice[18] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[12] + buf_slice[10];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[20] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[4];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);

    tmp = buf_slice[24] + buf_slice[28];
    s = buf_slice[20] + tmp;
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[8] + buf_slice[12];
    *d.add(0) = s; *d.add(8) = s; d = d.add(64);
    s = buf_slice[16] + tmp;
    *d.add(0) = s; *d.add(8) = s;

    /* final rescale + clip if es > 0 (rare) */
    if es != 0 {
        let n_clip = (31 - es) as u32;

        d = dest.add(64 * 16 + ((offset - odd_block) & 7) as usize + if odd_block != 0 { 0 } else { VBUF_LENGTH });
        s = *d.add(0);
        s = clip_2n(s, n_clip);
        s <<= es;
        *d.add(0) = s;
        *d.add(8) = s;

        d = dest.add(offset as usize + if odd_block != 0 { VBUF_LENGTH } else { 0 });
        for _ in 16..=31 {
            s = *d.add(0);
            s = clip_2n(s, n_clip);
            s <<= es;
            *d.add(0) = s;
            *d.add(8) = s;
            d = d.add(64);
        }

        d = dest.add(16 + ((offset - odd_block) & 7) as usize + if odd_block != 0 { 0 } else { VBUF_LENGTH });
        for _ in 0..16 {
            s = *d.add(0);
            s = clip_2n(s, n_clip);
            s <<= es;
            *d.add(0) = s;
            *d.add(8) = s;
            d = d.add(64);
        }
    }
}