#![no_std]
#![feature(asm_experimental_arch)]
use core::panic::PanicInfo;

use crabio::mp3_decoder::{
    BitStreamInfo, FrameHeader, MAX_NCHAN, MAX_NGRAN, MAX_SCFBD, MP3DecInfo, MPEGVersion, NBANDS, POLY_COEF, SFBandTable, SIBYTES_MPEG1_MONO, SIBYTES_MPEG1_STEREO, SIBYTES_MPEG2_MONO, SIBYTES_MPEG2_STEREO, ScaleFactorInfoSub, SideInfoSub, StereoMode, VBUF_LENGTH, clip_2n, clip_to_short, fdct_32, freq_invert_rescale, get_bits, idct_9, imdct_12, madd_64, mp3_find_free_sync, mp3_find_sync_word, mulshift_32, polyphase_mono, polyphase_stereo, refill_bitstream_cache, sar_64, unpack_frame_header, win_previous
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

#[repr(C)]
pub struct SideInfo {
    mainDataBegin: i32,
    privateBits: i32,
    scfsi: [[i32; MAX_SCFBD]; MAX_NCHAN],                /* 4 scalefactor bands per channel */
}

#[unsafe(no_mangle)]
pub unsafe fn UnpackSideInfo(
    buf: *const u8,
    m_SideInfo: *mut SideInfo,
    m_SideInfoSub: *mut [[SideInfoSub; MAX_NCHAN]; MAX_NGRAN],
    // m_SideInfoSub: *mut SideInfoSub,
    m_MP3DecInfo: *mut MP3DecInfo,
    m_MPEGVersion: i32,     // 1 = MPEG1, 0 = MPEG2/2.5
    m_sMode: i32,
) -> i32 {
    let mut gr: i32;
    let mut ch: i32;
    let mut bd: i32;
    let mut nBytes: i32;

    let m_SideInfoSub = unsafe { &mut *m_SideInfoSub };
    let m_SideInfo = unsafe { &mut *m_SideInfo };
    let m_MP3DecInfo = unsafe { &mut *m_MP3DecInfo };

    let mut bitStreamInfo: BitStreamInfoC = core::mem::zeroed();
    let mut bsi: *mut BitStreamInfoC = &mut bitStreamInfo;

    /* validate pointers and sync word */
    if (m_MPEGVersion == MPEGVersion::MPEG1 as i32) {
        /* MPEG 1 */
        nBytes= if m_sMode == StereoMode::Mono as i32 { SIBYTES_MPEG1_MONO as i32 } else { SIBYTES_MPEG1_STEREO as i32 };
        SetBitstreamPointer(bsi, nBytes, buf);
        m_SideInfo.mainDataBegin = GetBits(bsi, 9) as i32;
        m_SideInfo.privateBits= GetBits(
            bsi,
            (if m_sMode == StereoMode::Mono as i32 { 5 } else { 3 })
        ) as i32;
        for ch in 0..m_MP3DecInfo.nChans {
            for bd in 0..MAX_SCFBD {
                m_SideInfo.scfsi[ch as usize][bd] = GetBits(bsi, 1) as i32;
            }
        }
    } else {
        /* MPEG 2, MPEG 2.5 */
        nBytes = if m_sMode == StereoMode::Mono as i32 { SIBYTES_MPEG2_MONO as i32 } else { SIBYTES_MPEG2_STEREO as i32};
        SetBitstreamPointer(bsi, nBytes, buf);
        m_SideInfo.mainDataBegin = GetBits(bsi, 8) as i32;
        m_SideInfo.privateBits = GetBits(bsi, if m_sMode == StereoMode::Mono as i32 { 1 } else { 2 }) as i32;
    }
    for gr in 0..m_MP3DecInfo.nGrans {
        for ch in  0..m_MP3DecInfo.nChans {
            let sis =  &mut m_SideInfoSub[gr as usize][ch as usize]; /* side info subblock for this granule, channel */
            sis.part23_length = GetBits(bsi, 12) as i32;
            sis.n_bigvals = GetBits(bsi, 9) as i32;
            sis.global_gain = GetBits(bsi, 8) as i32;
            sis.sfCompress = GetBits(bsi, if m_MPEGVersion == MPEGVersion::MPEG1 as i32 { 4 } else { 9 }) as i32;
            sis.win_switch_flag = GetBits(bsi, 1) as i32;
            if sis.win_switch_flag != 0 {
                /* this is a start, stop, short, or mixed block */
                sis.blockType = GetBits(bsi, 2) as i32; /* 0 = normal, 1 = start, 2 = short, 3 = stop */
                sis.mixedBlock = GetBits(bsi, 1) as i32; /* 0 = not mixed, 1 = mixed */
                sis.tableSelect[0] = GetBits(bsi, 5) as i32;
                sis.tableSelect[1] = GetBits(bsi, 5) as i32;
                sis.tableSelect[2] = 0; /* unused */
                sis.subBlockGain[0] = GetBits(bsi, 3) as i32;
                sis.subBlockGain[1] = GetBits(bsi, 3) as i32;
                sis.subBlockGain[2] = GetBits(bsi, 3) as i32;
                if (sis.blockType == 0) {
                    /* this should not be allowed, according to spec */
                    sis.n_bigvals = 0;
                    sis.part23_length = 0;
                    sis.sfCompress = 0;
                } else if (sis.blockType == 2 && sis.mixedBlock == 0) {
                    /* short block, not mixed */
                    sis.region0Count = 8;
                } else {
                    /* start, stop, or short-mixed */
                    sis.region0Count = 7;
                }
                sis.region1Count = 20 - sis.region0Count;
            } else {
                /* this is a normal block */
                sis.blockType = 0;
                sis.mixedBlock = 0;
                sis.tableSelect[0] = GetBits(bsi, 5) as i32;
                sis.tableSelect[1] = GetBits(bsi, 5) as i32;
                sis.tableSelect[2] = GetBits(bsi, 5) as i32;
                sis.region0Count = GetBits(bsi, 4) as i32;
                sis.region1Count = GetBits(bsi, 3) as i32;
            }
            sis.preFlag = if m_MPEGVersion == MPEGVersion::MPEG1 as i32 { GetBits(bsi, 1) as i32 } else { 0 };
            sis.sfactScale = GetBits(bsi, 1) as i32;
            sis.count1TableSelect = GetBits(bsi, 1) as i32;
        }
    }
    m_MP3DecInfo.mainDataBegin = m_SideInfo.mainDataBegin; /* needed by main decode loop */
    // assert(nBytes == CalcBitsUsed(bsi, buf, 0) >> 3);
    return nBytes;
}

/***********************************************************************************************************************
 * Function:    MP3ClearBadFrame
 *
 * Description: zero out pcm buffer if error decoding MP3 frame
 *
 * Inputs:      mp3DecInfo struct with correct frame size parameters filled in
 *              pointer pcm output buffer
 *
 * Outputs:     zeroed out pcm buffer
 *
 * Return:      none
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
pub unsafe fn MP3ClearBadFrame(m_MP3DecInfo: *const MP3DecInfo, outbuf: *mut i16) {
    let m_MP3DecInfo = unsafe { &*m_MP3DecInfo };
    let outbuf = unsafe {
        core::slice::from_raw_parts_mut(outbuf, (m_MP3DecInfo.nGrans * m_MP3DecInfo.nGranSamps * m_MP3DecInfo.nChans) as usize)
    };
    outbuf.iter_mut().for_each(|e| *e = 0 );
}

const m_SFLenTab: [[u8; 2]; 16] = [
    [0, 0],
    [0, 1],
    [0, 2],
    [0, 3],
    [3, 0],
    [1, 1],
    [1, 2],
    [1, 3],
    [2, 1],
    [2, 2],
    [2, 3],
    [3, 1],
    [3, 2],
    [3, 3],
    [4, 2],
    [4, 3]
];

//----------------------------------------------------------------------------------------------------------------------
/***********************************************************************************************************************
 * Function:    UnpackSFMPEG1
 *
 * Description: unpack MPEG 1 scalefactors from bitstream
 *
 * Inputs:      BitStreamInfo, SideInfoSub, ScaleFactorInfoSub structs for this
 *                granule/channel
 *              vector of scfsi flags from side info, length = 4 (MAX_SCFBD)
 *              index of current granule
 *              ScaleFactorInfoSub from granule 0 (for granule 1, if scfsi[i] is set,
 *                then we just replicate the scale factors from granule 0 in the
 *                i'th set of scalefactor bands)
 *
 * Outputs:     updated BitStreamInfo struct
 *              scalefactors in sfis (short and/or long arrays, as appropriate)
 *
 * Return:      none
 *
 * Notes:       set order of short blocks to s[band][window] instead of s[window][band]
 *                so that we index through consectutive memory locations when unpacking
 *                (make sure dequantizer follows same convention)
 *              Illegal Intensity Position = 7 (always) for MPEG1 scale factors
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
pub unsafe fn UnpackSFMPEG1(bsi: *mut BitStreamInfoC, sis: *mut SideInfoSub,
                   sfis: *mut ScaleFactorInfoSub, scfsi: *const i32, gr: i32, sfisGr0: *mut ScaleFactorInfoSub) {
    let mut sfb: i32;
    let mut slen0: i32;
    let mut slen1: i32;

    let sis = unsafe { &*sis };
    let bsi = unsafe { &mut *bsi };
    let sfis = unsafe { &mut *sfis };
    let scfsi = unsafe { core::slice::from_raw_parts(scfsi, 4) };
    let sfisGr0 = unsafe { &* sfisGr0 };
    /* these can be 0, so make sure GetBits(bsi, 0) returns 0 (no >> 32 or anything) */
    slen0 = m_SFLenTab[sis.sfCompress as usize][0] as i32;
    slen1 = m_SFLenTab[sis.sfCompress as usize][1] as i32;
    if (sis.blockType == 2){
        /* short block, type 2 (implies winSwitchFlag == 1) */
        if (sis.mixedBlock != 0){
            /* do long block portion */
            for sfb in 0..8 {
                sfis.l[sfb]= GetBits(bsi, slen0 as u32) as u8;
            }
            sfb=3;
        }
        else {
            /* all short blocks */
            sfb=0;
        }
        for sfb in sfb..6 {
            sfis.s[sfb as usize][0] = GetBits(bsi, slen0 as u32) as u8;
            sfis.s[sfb as usize][1] = GetBits(bsi, slen0 as u32) as u8;
            sfis.s[sfb as usize][2] = GetBits(bsi, slen0 as u32) as u8;
        }
        for sfb in 6..12 {
            sfis.s[sfb][0] = GetBits(bsi, slen1 as u32) as u8;
            sfis.s[sfb][1] = GetBits(bsi, slen1 as u32) as u8;
            sfis.s[sfb][2] = GetBits(bsi, slen1 as u32) as u8;
        }
        /* last sf band not transmitted */
        sfis.s[12][0] = 0;
        sfis.s[12][1] = 0;
        sfis.s[12][2] = 0;
    }
    else{
        /* long blocks, type 0, 1, or 3 */
        if(gr == 0) {
            /* first granule */
            for sfb in 0..11 {
                sfis.l[sfb] = GetBits(bsi, slen0 as u32) as u8;
            }
            for sfb in 11..21 {
                sfis.l[sfb] = GetBits(bsi, slen1 as u32) as u8;
            }
            return;
        }
        else{
            /* second granule
             * scfsi: 0 = different scalefactors for each granule,
             *        1 = copy sf's from granule 0 into granule 1
             * for block type == 2, scfsi is always 0
             */
            sfb = 0;
            if(scfsi[0] != 0) {
                for sfb in 0..6 {
                    sfis.l[sfb] = sfisGr0.l[sfb];
                }
            } else {
                for sfb in 0..6 {
                    sfis.l[sfb] = GetBits(bsi, slen0 as u32) as u8;
                }
            }

            if(scfsi[1] != 0) {
                for sfb in 6..11 {
                    sfis.l[sfb] = sfisGr0.l[sfb];
                }
            } else {
                for sfb in 6..11 {
                    sfis.l[sfb] = GetBits(bsi, slen0 as u32) as u8;
                }
            }
            if(scfsi[2] != 0) {
                for sfb in 11..16 {
                    sfis.l[sfb] = sfisGr0.l[sfb];
                }
            } else {
                for sfb in 11..16 {
                    sfis.l[sfb] = GetBits(bsi, slen1 as u32) as u8;
                }
            }

            if(scfsi[3] != 0) {
                for sfb in 16..21 {
                    sfis.l[sfb] = sfisGr0.l[sfb];
                }
            } else {
                for sfb in 16..21 {
                    sfis.l[sfb] = GetBits(bsi, slen1 as u32) as u8;
                }
            }
        }
        /* last sf band not transmitted */
        sfis.l[21] = 0;
        sfis.l[22] = 0;
    }
}