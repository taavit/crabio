#![no_std]
#![feature(asm_experimental_arch)]
use core::panic::PanicInfo;

use crate::mp3_decoder::{BitStreamInfo, clip_to_short, get_bits, idct_9, imdct_12, madd_64, mp3_find_free_sync, mp3_find_sync_word, mulshift_32, refill_bitstream_cache, sar_64};

mod mp3_decoder;

#[repr(C)]
pub struct BitStreamInfoC {
    pub byte_ptr: *const u8,        // unsigned char *bytePtr;
    pub i_cache: u32,             // unsigned int iCache;
    pub cached_bits: i32,         // int cachedBits;
    pub n_bytes: i32,             // int nBytes;
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
        core::slice::from_raw_parts(x, 18).try_into().unwrap_unchecked() 
    };
    let out_arr: &mut [i32; 6] = unsafe { 
        core::slice::from_raw_parts_mut(out, 6).try_into().unwrap_unchecked() 
    };

    imdct_12(x_arr, out_arr);
}

#[unsafe(no_mangle)]
pub extern "C" fn idct9(x: *mut i32) {
    let x_arr: &mut [i32; 9] = unsafe { 
        core::slice::from_raw_parts_mut(x, 9).try_into().unwrap_unchecked() 
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
    let first_header: [u8; 4] = unsafe { [
        *first_fh.offset(0),
        *first_fh.offset(1),
        *first_fh.offset(2),
        *first_fh.offset(3),
    ] };
    mp3_find_free_sync(data, first_header)
        .map(|off| off as i32)
        .unwrap_or(-1)
}
