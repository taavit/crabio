#![no_std]
#![feature(asm_experimental_arch)]
use core::panic::PanicInfo;

use crabio::{
    mp3_decoder::{
        BLOCK_SIZE, BlockType, ChannelIndex, CriticalBandInfo, ERR_MP3_FREE_BITRATE_SYNC,
        ERR_MP3_INDATA_UNDERFLOW, ERR_MP3_INVALID_DEQUANTIZE, ERR_MP3_INVALID_FRAMEHEADER,
        ERR_MP3_INVALID_HUFFCODES, ERR_MP3_INVALID_IMDCT, ERR_MP3_INVALID_SCALEFACT,
        ERR_MP3_INVALID_SIDEINFO, ERR_MP3_INVALID_SUBBAND, ERR_MP3_MAINDATA_UNDERFLOW,
        ERR_MP3_NONE, FrameHeader, GranuleIndex, HUFF_PAIRTABS, HuffTabLookup,
        HuffTabType, HuffmanInfo, IMDCT_SCALE, IMDCTInfo, MAX_NCHAN, MAX_NGRAN, MAX_NSAMP,
        MAX_SCFBD, MP3DecInfo, MP3Decoder, MPEGVersion, NBANDS, SFBandTable, SQRTHALF,
        ScaleFactorInfoSub, ScaleFactorJS, SideInfo, SideInfoSub, clip_2n, freq_invert_rescale,
        idct_9, imdct_12, mp3_find_free_sync, mp3_find_sync_word, mulshift_32, win_previous,
    },
    utils::bit_stream_cache::BitStreamInfo,
};

macro_rules! profile_block {
    ($name:expr, $code:block) => {{
        let start_cycles: u32;
        unsafe {
            core::arch::asm!("rsr.ccount {0}", out(reg) start_cycles);
        }

        let result = unsafe { $code };

        let end_cycles: u32;
        unsafe {
            core::arch::asm!("rsr.ccount {0}", out(reg) end_cycles);
        }

        let diff = end_cycles.wrapping_sub(start_cycles);
        esp_println::println!("PERF: {} | {} cycles | {} us", $name, diff, diff / 240);

        result
    }};
}

#[repr(C)]
pub struct BlockCount {
    nBlocksLong: i32,
    nBlocksTotal: i32,
    nBlocksPrev: i32,
    prev_type: BlockType,
    prevWinSwitch: i32,
    currWinSwitch: i32,
    gbIn: i32,
    gbOut: i32,
}

#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub fn CLIP_2N(y: i32, n: u32) -> i32 {
    clip_2n(y, n)
}

#[panic_handler]
fn panic_handler(_: &PanicInfo) -> ! {
    loop {}
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
#[allow(non_snake_case)]
pub unsafe fn FreqInvertRescale(y: *mut i32, x_prev: &mut [i32], block_idx: i32, es: i32) -> i32 {
    let y_slice = unsafe { core::slice::from_raw_parts_mut(y, 9 * NBANDS * 2 + NBANDS) };

    freq_invert_rescale(y_slice, x_prev, block_idx, es)
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
pub fn MP3ClearBadFrame(outbuf: &mut [i16]) {
    outbuf.iter_mut().for_each(|e| *e = 0);
}

const M_SFLEN_TAB: [[u8; 2]; 16] = [
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
    [4, 3],
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
pub fn unpack_sfmpeg1(
    bsi: &mut BitStreamInfo,
    sis: &SideInfoSub,
    m_scale_factor_info_sub: &mut [[ScaleFactorInfoSub; MAX_NCHAN]; MAX_NGRAN],
    scfsi: &[i32; MAX_SCFBD],
    gr: GranuleIndex,
    ch: ChannelIndex,
) {
    let sfb: usize;
    let slen0: i32;
    let slen1: i32;

    /* these can be 0, so make sure GetBits(bsi, 0) returns 0 (no >> 32 or anything) */
    slen0 = M_SFLEN_TAB[sis.sfCompress as usize][0] as i32;
    slen1 = M_SFLEN_TAB[sis.sfCompress as usize][1] as i32;
    if sis.blockType == BlockType::Short {
        /* short block, type 2 (implies winSwitchFlag == 1) */
        if sis.mixedBlock != 0 {
            /* do long block portion */
            for sfb in 0..8 {
                m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                    bsi.get_bits(slen0 as u32) as u8;
            }
            sfb = 3;
        } else {
            /* all short blocks */
            sfb = 0;
        }
        for sfb in sfb..6 {
            m_scale_factor_info_sub[gr as usize][ch as usize].s[sfb][0] =
                bsi.get_bits(slen0 as u32) as u8;
            m_scale_factor_info_sub[gr as usize][ch as usize].s[sfb][1] =
                bsi.get_bits(slen0 as u32) as u8;
            m_scale_factor_info_sub[gr as usize][ch as usize].s[sfb][2] =
                bsi.get_bits(slen0 as u32) as u8;
        }
        for sfb in 6..12 {
            m_scale_factor_info_sub[gr as usize][ch as usize].s[sfb][0] =
                bsi.get_bits(slen1 as u32) as u8;
            m_scale_factor_info_sub[gr as usize][ch as usize].s[sfb][1] =
                bsi.get_bits(slen1 as u32) as u8;
            m_scale_factor_info_sub[gr as usize][ch as usize].s[sfb][2] =
                bsi.get_bits(slen1 as u32) as u8;
        }
        /* last sf band not transmitted */
        m_scale_factor_info_sub[gr as usize][ch as usize].s[12][0] = 0;
        m_scale_factor_info_sub[gr as usize][ch as usize].s[12][1] = 0;
        m_scale_factor_info_sub[gr as usize][ch as usize].s[12][2] = 0;
    } else {
        /* long blocks, type 0, 1, or 3 */
        if gr == GranuleIndex::Granule0 {
            /* first granule */
            for sfb in 0..11 {
                m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                    bsi.get_bits(slen0 as u32) as u8;
            }
            for sfb in 11..21 {
                m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                    bsi.get_bits(slen1 as u32) as u8;
            }
            return;
        } else {
            /* second granule
             * scfsi: 0 = different scalefactors for each granule,
             *        1 = copy sf's from granule 0 into granule 1
             * for block type == 2, scfsi is always 0
             */
            if scfsi[0] != 0 {
                for sfb in 0..6 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        m_scale_factor_info_sub[0][ch as usize].l[sfb];
                }
            } else {
                for sfb in 0..6 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        bsi.get_bits(slen0 as u32) as u8;
                }
            }

            if scfsi[1] != 0 {
                for sfb in 6..11 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        m_scale_factor_info_sub[0][ch as usize].l[sfb];
                }
            } else {
                for sfb in 6..11 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        bsi.get_bits(slen0 as u32) as u8;
                }
            }
            if scfsi[2] != 0 {
                for sfb in 11..16 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        m_scale_factor_info_sub[0][ch as usize].l[sfb];
                }
            } else {
                for sfb in 11..16 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        bsi.get_bits(slen1 as u32) as u8;
                }
            }

            if scfsi[3] != 0 {
                for sfb in 16..21 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        m_scale_factor_info_sub[0][ch as usize].l[sfb];
                }
            } else {
                for sfb in 16..21 {
                    m_scale_factor_info_sub[gr as usize][ch as usize].l[sfb] =
                        bsi.get_bits(slen1 as u32) as u8;
                }
            }
        }
        /* last sf band not transmitted */
        m_scale_factor_info_sub[gr as usize][ch as usize].l[21] = 0;
        m_scale_factor_info_sub[gr as usize][ch as usize].l[22] = 0;
    }
}

/* NRTab[size + 3*is_right][block type][partition]
 *   block type index: 0 = (bt0,bt1,bt3), 1 = bt2 non-mixed, 2 = bt2 mixed
 *   partition: scale factor groups (sfb1 through sfb4)
 * for block type = 2 (mixed or non-mixed) / by 3 is rolled into this table
 *   (for 3 short blocks per long block)
 * see 2.4.3.2 in MPEG 2 (low sample rate) spec
 * stuff rolled into this table:
 *   NRTab[x][1][y]   --> (NRTab[x][1][y])   / 3
 *   NRTab[x][2][>=1] --> (NRTab[x][2][>=1]) / 3  (first partition is long block)
 */
const NRTAB: [[[u8; 4]; 3]; 6] = [
    [[6, 5, 5, 5], [3, 3, 3, 3], [6, 3, 3, 3]],
    [[6, 5, 7, 3], [3, 3, 4, 2], [6, 3, 4, 2]],
    [[11, 10, 0, 0], [6, 6, 0, 0], [6, 3, 6, 0]],
    [[7, 7, 7, 0], [4, 4, 4, 0], [6, 5, 4, 0]],
    [[6, 6, 6, 3], [4, 3, 3, 2], [6, 4, 3, 2]],
    [[8, 8, 5, 0], [5, 4, 3, 0], [6, 6, 3, 0]],
];

pub fn unpack_sfmpeg2(
    bsi: &mut BitStreamInfo,
    sis: &mut SideInfoSub,
    sfis: &mut ScaleFactorInfoSub,
    _gr: GranuleIndex, // nieużywane, zachowane dla sygnatury
    ch: ChannelIndex,
    mode_ext: usize,
    sfjs: &mut ScaleFactorJS,
) {
    let mut sfb: usize;
    let sfc_idx: usize;
    let mut bt_idx: usize;
    let mut nr_idx: usize;
    let mut slen = [0i32; 4];
    let mut nr = [0i32; 4];

    let mut sf_compress = sis.sfCompress;
    let mut pre_flag = 0;
    let mut intensity_scale = 0;

    /* stereo mode bits (1 = on): bit 1 = mid-side on/off, bit 0 = intensity on/off */
    if !((mode_ext & 0x01 != 0) && (ch == ChannelIndex::Channel1)) {
        if sf_compress < 400 {
            slen[0] = (sf_compress >> 4) / 5;
            slen[1] = (sf_compress >> 4) % 5;
            slen[2] = (sf_compress & 0x0f) >> 2;
            slen[3] = sf_compress & 0x03;
            sfc_idx = 0;
        } else if sf_compress < 500 {
            sf_compress -= 400;
            slen[0] = (sf_compress >> 2) / 5;
            slen[1] = (sf_compress >> 2) % 5;
            slen[2] = sf_compress & 0x03;
            slen[3] = 0;
            sfc_idx = 1;
        } else {
            sf_compress -= 500;
            slen[0] = sf_compress / 3;
            slen[1] = sf_compress % 3;
            slen[2] = 0;
            slen[3] = 0;
            if sis.mixedBlock != 0 {
                slen[2] = slen[1];
                slen[1] = slen[0];
            }
            pre_flag = 1;
            sfc_idx = 2;
        }
    } else {
        /* intensity stereo ch = 1 (right) */
        intensity_scale = sf_compress & 0x01;
        sf_compress >>= 1;
        if sf_compress < 180 {
            slen[0] = sf_compress / 36;
            slen[1] = (sf_compress % 36) / 6;
            slen[2] = (sf_compress % 36) % 6;
            slen[3] = 0;
            sfc_idx = 3;
        } else if sf_compress < 244 {
            sf_compress -= 180;
            slen[0] = (sf_compress & 0x3f) >> 4;
            slen[1] = (sf_compress & 0x0f) >> 2;
            slen[2] = sf_compress & 0x03;
            slen[3] = 0;
            sfc_idx = 4;
        } else {
            sf_compress -= 244;
            slen[0] = sf_compress / 3;
            slen[1] = sf_compress % 3;
            slen[2] = 0;
            slen[3] = 0;
            sfc_idx = 5;
        }
    }

    /* btIdx: (0,1,3) --> 0, (2 non-mixed) --> 1, (2 mixed) ---> 2 */
    bt_idx = 0;
    if sis.blockType == BlockType::Short {
        bt_idx = if sis.mixedBlock != 0 { 2 } else { 1 };
    }

    for i in 0..4 {
        // Zakładamy, że NRTab jest dostępny jako static/extern
        nr[i] = NRTAB[sfc_idx][bt_idx][i] as i32;
    }

    /* save intensity stereo scale factor info */
    if (mode_ext & 0x01 != 0) && (ch == ChannelIndex::Channel1) {
        for i in 0..4 {
            sfjs.slen[i] = slen[i];
            sfjs.nr[i] = nr[i];
        }
        sfjs.intensity_scale = intensity_scale;
    }
    sis.preFlag = pre_flag;

    /* Rozpakowywanie skal */
    if sis.blockType == BlockType::Short {
        if sis.mixedBlock != 0 {
            /* Część dla bloków długich (long) */
            for sfb in 0..6 {
                sfis.l[sfb] = bsi.get_bits(slen[0] as u32) as u8;
            }
            sfb = 3; /* Startowy indeks sfb dla krótkich */
            nr_idx = 1;
        } else {
            sfb = 0;
            nr_idx = 0;
        }

        /* Pozostałe bloki krótkie */
        while nr_idx <= 3 {
            for _ in 0..nr[nr_idx] {
                sfis.s[sfb][0] = bsi.get_bits(slen[nr_idx] as u32) as u8;
                sfis.s[sfb][1] = bsi.get_bits(slen[nr_idx] as u32) as u8;
                sfis.s[sfb][2] = bsi.get_bits(slen[nr_idx] as u32) as u8;
                sfb += 1;
            }
            nr_idx += 1;
        }
        /* Ostatnie pasmo nie jest przesyłane */
        sfis.s[12][0] = 0;
        sfis.s[12][1] = 0;
        sfis.s[12][2] = 0;
    } else {
        /* Bloki długie (long) */
        sfb = 0;
        for nr_idx in 0..=3 {
            for _ in 0..nr[nr_idx] {
                sfis.l[sfb] = bsi.get_bits(slen[nr_idx] as u32) as u8;
                sfb += 1;
            }
        }
        /* Ostatnie pasma nie są przesyłane */
        sfis.l[21] = 0;
        sfis.l[22] = 0;
    }
}

/// HUFFMAN
/***********************************************************************************************************************
 * Function:    DecodeHuffmanPairs
 *
 * Description: decode 2-way vector Huffman codes in the "bigValues" region of spectrum
 *
 * Inputs:      valid BitStreamInfo struct, pointing to start of pair-wise codes
 *              pointer to xy buffer to received decoded values
 *              number of codewords to decode
 *              index of Huffman table to use
 *              number of bits remaining in bitstream
 *
 * Outputs:     pairs of decoded coefficients in vwxy
 *              updated BitStreamInfo struct
 *
 * Return:      number of bits used, or -1 if out of bits
 *
 * Notes:       assumes that nVals is an even number
 *              si_huff.bit tests every Huffman codeword in every table (though not
 *                necessarily all linBits outputs for x,y > 15)
 **********************************************************************************************************************/
// no improvement with section=data

const HUFF_TABLE: [u16; 4242] = [
    /* huffTable01[9] */
    0xf003, 0x3112, 0x3101, 0x2011, 0x2011, 0x1000, 0x1000, 0x1000, 0x1000,
    /* huffTable02[65] */
    0xf006, 0x6222, 0x6201, 0x5212, 0x5212, 0x5122, 0x5122, 0x5021, 0x5021, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, /* huffTable03[65] */
    0xf006, 0x6222, 0x6201, 0x5212, 0x5212, 0x5122, 0x5122, 0x5021, 0x5021, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2101, 0x2101, 0x2101,
    0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101, 0x2101,
    0x2101, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, /* huffTable05[257] */
    0xf008, 0x8332, 0x8322, 0x7232, 0x7232, 0x6132, 0x6132, 0x6132, 0x6132, 0x7312, 0x7312, 0x7301,
    0x7301, 0x7031, 0x7031, 0x7222, 0x7222, 0x6212, 0x6212, 0x6212, 0x6212, 0x6122, 0x6122, 0x6122,
    0x6122, 0x6201, 0x6201, 0x6201, 0x6201, 0x6021, 0x6021, 0x6021, 0x6021, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, /* huffTable06[129] */
    0xf007, 0x7332, 0x7301, 0x6322, 0x6322, 0x6232, 0x6232, 0x6031, 0x6031, 0x5312, 0x5312, 0x5312,
    0x5312, 0x5132, 0x5132, 0x5132, 0x5132, 0x5222, 0x5222, 0x5222, 0x5222, 0x5201, 0x5201, 0x5201,
    0x5201, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4122, 0x4122, 0x4122,
    0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4021, 0x4021, 0x4021, 0x4021, 0x4021, 0x4021, 0x4021,
    0x4021, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000,
    0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000,
    /* huffTable07[110] */
    0xf006, 0x0041, 0x0052, 0x005b, 0x0060, 0x0063, 0x0068, 0x006b, 0x6212, 0x5122, 0x5122, 0x6201,
    0x6021, 0x4112, 0x4112, 0x4112, 0x4112, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0xf004, 0x4552, 0x4542, 0x4452, 0x4352, 0x3532, 0x3532,
    0x3442, 0x3442, 0x3522, 0x3522, 0x3252, 0x3252, 0x2512, 0x2512, 0x2512, 0x2512, 0xf003, 0x2152,
    0x2152, 0x3501, 0x3432, 0x2051, 0x2051, 0x3342, 0x3332, 0xf002, 0x2422, 0x2242, 0x1412, 0x1412,
    0xf001, 0x1142, 0x1041, 0xf002, 0x2401, 0x2322, 0x2232, 0x2301, 0xf001, 0x1312, 0x1132, 0xf001,
    0x1031, 0x1222, /* huffTable08[280] */
    0xf008, 0x0101, 0x010a, 0x010f, 0x8512, 0x8152, 0x0112, 0x0115, 0x8422, 0x8242, 0x8412, 0x7142,
    0x7142, 0x8401, 0x8041, 0x8322, 0x8232, 0x8312, 0x8132, 0x8301, 0x8031, 0x6222, 0x6222, 0x6222,
    0x6222, 0x6201, 0x6201, 0x6201, 0x6201, 0x6021, 0x6021, 0x6021, 0x6021, 0x4212, 0x4212, 0x4212,
    0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212,
    0x4212, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x4122,
    0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112,
    0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x2112, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0xf003, 0x3552, 0x3452, 0x2542, 0x2542, 0x1352, 0x1352,
    0x1352, 0x1352, 0xf002, 0x2532, 0x2442, 0x1522, 0x1522, 0xf001, 0x1252, 0x1501, 0xf001, 0x1432,
    0x1342, 0xf001, 0x1051, 0x1332, /* huffTable09[93] */
    0xf006, 0x0041, 0x004a, 0x004f, 0x0052, 0x0057, 0x005a, 0x6412, 0x6142, 0x6322, 0x6232, 0x5312,
    0x5312, 0x5132, 0x5132, 0x6301, 0x6031, 0x5222, 0x5222, 0x5201, 0x5201, 0x4212, 0x4212, 0x4212,
    0x4212, 0x4122, 0x4122, 0x4122, 0x4122, 0x4021, 0x4021, 0x4021, 0x4021, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3000, 0x3000, 0x3000,
    0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0xf003, 0x3552, 0x3542, 0x2532, 0x2532, 0x2352, 0x2352,
    0x3452, 0x3501, 0xf002, 0x2442, 0x2522, 0x2252, 0x2512, 0xf001, 0x1152, 0x1432, 0xf002, 0x1342,
    0x1342, 0x2051, 0x2401, 0xf001, 0x1422, 0x1242, 0xf001, 0x1332, 0x1041,
    /* huffTable10[320] */
    0xf008, 0x0101, 0x010a, 0x010f, 0x0118, 0x011b, 0x0120, 0x0125, 0x8712, 0x8172, 0x012a, 0x012d,
    0x0132, 0x8612, 0x8162, 0x8061, 0x0137, 0x013a, 0x013d, 0x8412, 0x8142, 0x8041, 0x8322, 0x8232,
    0x8301, 0x7312, 0x7312, 0x7132, 0x7132, 0x7031, 0x7031, 0x7222, 0x7222, 0x6212, 0x6212, 0x6212,
    0x6212, 0x6122, 0x6122, 0x6122, 0x6122, 0x6201, 0x6201, 0x6201, 0x6201, 0x6021, 0x6021, 0x6021,
    0x6021, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112,
    0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0xf003, 0x3772, 0x3762, 0x3672, 0x3752, 0x3572, 0x3662,
    0x2742, 0x2742, 0xf002, 0x2472, 0x2652, 0x2562, 0x2732, 0xf003, 0x2372, 0x2372, 0x2642, 0x2642,
    0x3552, 0x3452, 0x2362, 0x2362, 0xf001, 0x1722, 0x1272, 0xf002, 0x2462, 0x2701, 0x1071, 0x1071,
    0xf002, 0x1262, 0x1262, 0x2542, 0x2532, 0xf002, 0x1601, 0x1601, 0x2352, 0x2442, 0xf001, 0x1632,
    0x1622, 0xf002, 0x2522, 0x2252, 0x1512, 0x1512, 0xf002, 0x1152, 0x1152, 0x2432, 0x2342, 0xf001,
    0x1501, 0x1051, 0xf001, 0x1422, 0x1242, 0xf001, 0x1332, 0x1401,
    /* huffTable11[296] */
    0xf008, 0x0101, 0x0106, 0x010f, 0x0114, 0x0117, 0x8722, 0x8272, 0x011c, 0x7172, 0x7172, 0x8712,
    0x8071, 0x8632, 0x8362, 0x8061, 0x011f, 0x0122, 0x8512, 0x7262, 0x7262, 0x8622, 0x8601, 0x7612,
    0x7612, 0x7162, 0x7162, 0x8152, 0x8432, 0x8051, 0x0125, 0x8422, 0x8242, 0x8412, 0x8142, 0x8401,
    0x8041, 0x7322, 0x7322, 0x7232, 0x7232, 0x6312, 0x6312, 0x6312, 0x6312, 0x6132, 0x6132, 0x6132,
    0x6132, 0x7301, 0x7301, 0x7031, 0x7031, 0x6222, 0x6222, 0x6222, 0x6222, 0x5122, 0x5122, 0x5122,
    0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212,
    0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x5201, 0x5201, 0x5201,
    0x5201, 0x5201, 0x5201, 0x5201, 0x5201, 0x5021, 0x5021, 0x5021, 0x5021, 0x5021, 0x5021, 0x5021,
    0x5021, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0x2000,
    0x2000, 0x2000, 0x2000, 0x2000, 0x2000, 0xf002, 0x2772, 0x2762, 0x2672, 0x2572, 0xf003, 0x2662,
    0x2662, 0x2742, 0x2742, 0x2472, 0x2472, 0x3752, 0x3552, 0xf002, 0x2652, 0x2562, 0x1732, 0x1732,
    0xf001, 0x1372, 0x1642, 0xf002, 0x2542, 0x2452, 0x2532, 0x2352, 0xf001, 0x1462, 0x1701, 0xf001,
    0x1442, 0x1522, 0xf001, 0x1252, 0x1501, 0xf001, 0x1342, 0x1332,
    /* huffTable12[185] */
    0xf007, 0x0081, 0x008a, 0x008f, 0x0092, 0x0097, 0x009a, 0x009d, 0x00a2, 0x00a5, 0x00a8, 0x7622,
    0x7262, 0x7162, 0x00ad, 0x00b0, 0x00b3, 0x7512, 0x7152, 0x7432, 0x7342, 0x00b6, 0x7422, 0x7242,
    0x7412, 0x6332, 0x6332, 0x6142, 0x6142, 0x6322, 0x6322, 0x6232, 0x6232, 0x7041, 0x7301, 0x6031,
    0x6031, 0x5312, 0x5312, 0x5312, 0x5312, 0x5132, 0x5132, 0x5132, 0x5132, 0x5222, 0x5222, 0x5222,
    0x5222, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4212, 0x4122, 0x4122, 0x4122,
    0x4122, 0x4122, 0x4122, 0x4122, 0x4122, 0x5201, 0x5201, 0x5201, 0x5201, 0x5021, 0x5021, 0x5021,
    0x5021, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3101,
    0x3101, 0x3101, 0x3101, 0x3101, 0x3101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0xf003, 0x3772, 0x3762,
    0x2672, 0x2672, 0x2752, 0x2752, 0x2572, 0x2572, 0xf002, 0x2662, 0x2742, 0x2472, 0x2562, 0xf001,
    0x1652, 0x1732, 0xf002, 0x2372, 0x2552, 0x1722, 0x1722, 0xf001, 0x1272, 0x1642, 0xf001, 0x1462,
    0x1712, 0xf002, 0x1172, 0x1172, 0x2701, 0x2071, 0xf001, 0x1632, 0x1362, 0xf001, 0x1542, 0x1452,
    0xf002, 0x1442, 0x1442, 0x2601, 0x2501, 0xf001, 0x1612, 0x1061, 0xf001, 0x1532, 0x1352, 0xf001,
    0x1522, 0x1252, 0xf001, 0x1051, 0x1401, /* huffTable13[497] */
    0xf006, 0x0041, 0x0082, 0x00c3, 0x00e4, 0x0105, 0x0116, 0x011f, 0x0130, 0x0139, 0x013e, 0x0143,
    0x0146, 0x6212, 0x6122, 0x6201, 0x6021, 0x4112, 0x4112, 0x4112, 0x4112, 0x4101, 0x4101, 0x4101,
    0x4101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0xf006, 0x0108, 0x0111, 0x011a, 0x0123, 0x012c, 0x0131,
    0x0136, 0x013f, 0x0144, 0x0147, 0x014c, 0x0151, 0x0156, 0x015b, 0x6f12, 0x61f2, 0x60f1, 0x0160,
    0x0163, 0x0166, 0x62e2, 0x0169, 0x6e12, 0x61e2, 0x016c, 0x016f, 0x0172, 0x0175, 0x0178, 0x017b,
    0x66c2, 0x6d32, 0x017e, 0x6d22, 0x62d2, 0x6d12, 0x67b2, 0x0181, 0x0184, 0x63c2, 0x0187, 0x6b42,
    0x51d2, 0x51d2, 0x6d01, 0x60d1, 0x6a82, 0x68a2, 0x6c42, 0x64c2, 0x6b62, 0x66b2, 0x5c32, 0x5c32,
    0x5c22, 0x5c22, 0x52c2, 0x52c2, 0x5b52, 0x5b52, 0x65b2, 0x6982, 0x5c12, 0x5c12, 0xf006, 0x51c2,
    0x51c2, 0x6892, 0x6c01, 0x50c1, 0x50c1, 0x64b2, 0x6a62, 0x66a2, 0x6972, 0x5b32, 0x5b32, 0x53b2,
    0x53b2, 0x6882, 0x6a52, 0x5b22, 0x5b22, 0x65a2, 0x6962, 0x54a2, 0x54a2, 0x6872, 0x6782, 0x5492,
    0x5492, 0x6772, 0x6672, 0x42b2, 0x42b2, 0x42b2, 0x42b2, 0x4b12, 0x4b12, 0x4b12, 0x4b12, 0x41b2,
    0x41b2, 0x41b2, 0x41b2, 0x5b01, 0x5b01, 0x50b1, 0x50b1, 0x5692, 0x5692, 0x5a42, 0x5a42, 0x5a32,
    0x5a32, 0x53a2, 0x53a2, 0x5952, 0x5952, 0x5592, 0x5592, 0x4a22, 0x4a22, 0x4a22, 0x4a22, 0x42a2,
    0x42a2, 0x42a2, 0x42a2, 0xf005, 0x4a12, 0x4a12, 0x41a2, 0x41a2, 0x5a01, 0x5862, 0x40a1, 0x40a1,
    0x5682, 0x5942, 0x4392, 0x4392, 0x5932, 0x5852, 0x5582, 0x5762, 0x4922, 0x4922, 0x4292, 0x4292,
    0x5752, 0x5572, 0x4832, 0x4832, 0x4382, 0x4382, 0x5662, 0x5742, 0x5472, 0x5652, 0x5562, 0x5372,
    0xf005, 0x3912, 0x3912, 0x3912, 0x3912, 0x3192, 0x3192, 0x3192, 0x3192, 0x4901, 0x4901, 0x4091,
    0x4091, 0x4842, 0x4842, 0x4482, 0x4482, 0x4272, 0x4272, 0x5642, 0x5462, 0x3822, 0x3822, 0x3822,
    0x3822, 0x3282, 0x3282, 0x3282, 0x3282, 0x3812, 0x3812, 0x3812, 0x3812, 0xf004, 0x4732, 0x4722,
    0x3712, 0x3712, 0x3172, 0x3172, 0x4552, 0x4701, 0x4071, 0x4632, 0x4362, 0x4542, 0x4452, 0x4622,
    0x4262, 0x4532, 0xf003, 0x2182, 0x2182, 0x3801, 0x3081, 0x3612, 0x3162, 0x3601, 0x3061, 0xf004,
    0x4352, 0x4442, 0x3522, 0x3522, 0x3252, 0x3252, 0x3501, 0x3501, 0x2512, 0x2512, 0x2512, 0x2512,
    0x2152, 0x2152, 0x2152, 0x2152, 0xf003, 0x3432, 0x3342, 0x3051, 0x3422, 0x3242, 0x3332, 0x2412,
    0x2412, 0xf002, 0x1142, 0x1142, 0x2401, 0x2041, 0xf002, 0x2322, 0x2232, 0x1312, 0x1312, 0xf001,
    0x1132, 0x1301, 0xf001, 0x1031, 0x1222, 0xf003, 0x0082, 0x008b, 0x008e, 0x0091, 0x0094, 0x0097,
    0x3ce2, 0x3dd2, 0xf003, 0x0093, 0x3eb2, 0x3be2, 0x3f92, 0x39f2, 0x3ae2, 0x3db2, 0x3bd2, 0xf003,
    0x3f82, 0x38f2, 0x3cc2, 0x008d, 0x3e82, 0x0090, 0x27f2, 0x27f2, 0xf003, 0x2ad2, 0x2ad2, 0x3da2,
    0x3cb2, 0x3bc2, 0x36f2, 0x2f62, 0x2f62, 0xf002, 0x28e2, 0x2f52, 0x2d92, 0x29d2, 0xf002, 0x25f2,
    0x27e2, 0x2ca2, 0x2bb2, 0xf003, 0x2f42, 0x2f42, 0x24f2, 0x24f2, 0x3ac2, 0x36e2, 0x23f2, 0x23f2,
    0xf002, 0x1f32, 0x1f32, 0x2d82, 0x28d2, 0xf001, 0x1f22, 0x12f2, 0xf002, 0x2e62, 0x2c92, 0x1f01,
    0x1f01, 0xf002, 0x29c2, 0x2e52, 0x1ba2, 0x1ba2, 0xf002, 0x2d72, 0x27d2, 0x1e42, 0x1e42, 0xf002,
    0x28c2, 0x26d2, 0x1e32, 0x1e32, 0xf002, 0x19b2, 0x19b2, 0x2b92, 0x2aa2, 0xf001, 0x1ab2, 0x15e2,
    0xf001, 0x14e2, 0x1c82, 0xf001, 0x1d62, 0x13e2, 0xf001, 0x1e22, 0x1e01, 0xf001, 0x10e1, 0x1d52,
    0xf001, 0x15d2, 0x1c72, 0xf001, 0x17c2, 0x1d42, 0xf001, 0x1b82, 0x18b2, 0xf001, 0x14d2, 0x1a92,
    0xf001, 0x19a2, 0x1c62, 0xf001, 0x13d2, 0x1b72, 0xf001, 0x1c52, 0x15c2, 0xf001, 0x1992, 0x1a72,
    0xf001, 0x17a2, 0x1792, 0xf003, 0x0023, 0x3df2, 0x2de2, 0x2de2, 0x1ff2, 0x1ff2, 0x1ff2, 0x1ff2,
    0xf001, 0x1fe2, 0x1fd2, 0xf001, 0x1ee2, 0x1fc2, 0xf001, 0x1ed2, 0x1fb2, 0xf001, 0x1bf2, 0x1ec2,
    0xf002, 0x1cd2, 0x1cd2, 0x2fa2, 0x29e2, 0xf001, 0x1af2, 0x1dc2, 0xf001, 0x1ea2, 0x1e92, 0xf001,
    0x1f72, 0x1e72, 0xf001, 0x1ef2, 0x1cf2, /* huffTable15[580] */
    0xf008, 0x0101, 0x0122, 0x0143, 0x0154, 0x0165, 0x0176, 0x017f, 0x0188, 0x0199, 0x01a2, 0x01ab,
    0x01b4, 0x01bd, 0x01c2, 0x01cb, 0x01d4, 0x01d9, 0x01de, 0x01e3, 0x01e8, 0x01ed, 0x01f2, 0x01f7,
    0x01fc, 0x0201, 0x0204, 0x0207, 0x020a, 0x020f, 0x0212, 0x0215, 0x021a, 0x021d, 0x0220, 0x8192,
    0x0223, 0x0226, 0x0229, 0x022c, 0x022f, 0x8822, 0x8282, 0x8812, 0x8182, 0x0232, 0x0235, 0x0238,
    0x023b, 0x8722, 0x8272, 0x8462, 0x8712, 0x8552, 0x8172, 0x023e, 0x8632, 0x8362, 0x8542, 0x8452,
    0x8622, 0x8262, 0x8612, 0x0241, 0x8532, 0x7162, 0x7162, 0x8352, 0x8442, 0x7522, 0x7522, 0x7252,
    0x7252, 0x7512, 0x7512, 0x7152, 0x7152, 0x8501, 0x8051, 0x7432, 0x7432, 0x7342, 0x7342, 0x7422,
    0x7422, 0x7242, 0x7242, 0x7332, 0x7332, 0x6142, 0x6142, 0x6142, 0x6142, 0x7412, 0x7412, 0x7401,
    0x7401, 0x6322, 0x6322, 0x6322, 0x6322, 0x6232, 0x6232, 0x6232, 0x6232, 0x7041, 0x7041, 0x7301,
    0x7301, 0x6312, 0x6312, 0x6312, 0x6312, 0x6132, 0x6132, 0x6132, 0x6132, 0x6031, 0x6031, 0x6031,
    0x6031, 0x5222, 0x5222, 0x5222, 0x5222, 0x5222, 0x5222, 0x5222, 0x5222, 0x5212, 0x5212, 0x5212,
    0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122,
    0x5122, 0x5201, 0x5201, 0x5201, 0x5201, 0x5201, 0x5201, 0x5201, 0x5201, 0x5021, 0x5021, 0x5021,
    0x5021, 0x5021, 0x5021, 0x5021, 0x5021, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112, 0x3112,
    0x3112, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101,
    0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011,
    0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x3000, 0x3000, 0x3000,
    0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000,
    0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0x3000,
    0x3000, 0x3000, 0x3000, 0x3000, 0x3000, 0xf005, 0x5ff2, 0x5fe2, 0x5ef2, 0x5fd2, 0x4ee2, 0x4ee2,
    0x5df2, 0x5fc2, 0x5cf2, 0x5ed2, 0x5de2, 0x5fb2, 0x4bf2, 0x4bf2, 0x5ec2, 0x5ce2, 0x4dd2, 0x4dd2,
    0x4fa2, 0x4fa2, 0x4af2, 0x4af2, 0x4eb2, 0x4eb2, 0x4be2, 0x4be2, 0x4dc2, 0x4dc2, 0x4cd2, 0x4cd2,
    0x4f92, 0x4f92, 0xf005, 0x49f2, 0x49f2, 0x4ae2, 0x4ae2, 0x4db2, 0x4db2, 0x4bd2, 0x4bd2, 0x4f82,
    0x4f82, 0x48f2, 0x48f2, 0x4cc2, 0x4cc2, 0x4e92, 0x4e92, 0x49e2, 0x49e2, 0x4f72, 0x4f72, 0x47f2,
    0x47f2, 0x4da2, 0x4da2, 0x4ad2, 0x4ad2, 0x4cb2, 0x4cb2, 0x4f62, 0x4f62, 0x5ea2, 0x5f01, 0xf004,
    0x3bc2, 0x3bc2, 0x36f2, 0x36f2, 0x4e82, 0x48e2, 0x4f52, 0x4d92, 0x35f2, 0x35f2, 0x3e72, 0x3e72,
    0x37e2, 0x37e2, 0x3ca2, 0x3ca2, 0xf004, 0x3ac2, 0x3ac2, 0x3bb2, 0x3bb2, 0x49d2, 0x4d82, 0x3f42,
    0x3f42, 0x34f2, 0x34f2, 0x3f32, 0x3f32, 0x33f2, 0x33f2, 0x38d2, 0x38d2, 0xf004, 0x36e2, 0x36e2,
    0x3f22, 0x3f22, 0x32f2, 0x32f2, 0x4e62, 0x40f1, 0x3f12, 0x3f12, 0x31f2, 0x31f2, 0x3c92, 0x3c92,
    0x39c2, 0x39c2, 0xf003, 0x3e52, 0x3ba2, 0x3ab2, 0x35e2, 0x3d72, 0x37d2, 0x3e42, 0x34e2, 0xf003,
    0x3c82, 0x38c2, 0x3e32, 0x3d62, 0x36d2, 0x33e2, 0x3b92, 0x39b2, 0xf004, 0x3e22, 0x3e22, 0x3aa2,
    0x3aa2, 0x32e2, 0x32e2, 0x3e12, 0x3e12, 0x31e2, 0x31e2, 0x4e01, 0x40e1, 0x3d52, 0x3d52, 0x35d2,
    0x35d2, 0xf003, 0x3c72, 0x37c2, 0x3d42, 0x3b82, 0x24d2, 0x24d2, 0x38b2, 0x3a92, 0xf003, 0x39a2,
    0x3c62, 0x36c2, 0x3d32, 0x23d2, 0x23d2, 0x22d2, 0x22d2, 0xf003, 0x3d22, 0x3d01, 0x2d12, 0x2d12,
    0x2b72, 0x2b72, 0x27b2, 0x27b2, 0xf003, 0x21d2, 0x21d2, 0x3c52, 0x30d1, 0x25c2, 0x25c2, 0x2a82,
    0x2a82, 0xf002, 0x28a2, 0x2c42, 0x24c2, 0x2b62, 0xf003, 0x26b2, 0x26b2, 0x3992, 0x3c01, 0x2c32,
    0x2c32, 0x23c2, 0x23c2, 0xf003, 0x2a72, 0x2a72, 0x27a2, 0x27a2, 0x26a2, 0x26a2, 0x30c1, 0x3b01,
    0xf002, 0x12c2, 0x12c2, 0x2c22, 0x2b52, 0xf002, 0x25b2, 0x2c12, 0x2982, 0x2892, 0xf002, 0x21c2,
    0x2b42, 0x24b2, 0x2a62, 0xf002, 0x2b32, 0x2972, 0x13b2, 0x13b2, 0xf002, 0x2792, 0x2882, 0x2b22,
    0x2a52, 0xf002, 0x12b2, 0x12b2, 0x25a2, 0x2b12, 0xf002, 0x11b2, 0x11b2, 0x20b1, 0x2962, 0xf002,
    0x2692, 0x2a42, 0x24a2, 0x2872, 0xf002, 0x2782, 0x2a32, 0x13a2, 0x13a2, 0xf001, 0x1952, 0x1592,
    0xf001, 0x1a22, 0x12a2, 0xf001, 0x1a12, 0x11a2, 0xf002, 0x2a01, 0x20a1, 0x1862, 0x1862, 0xf001,
    0x1682, 0x1942, 0xf001, 0x1492, 0x1932, 0xf002, 0x1392, 0x1392, 0x2772, 0x2901, 0xf001, 0x1852,
    0x1582, 0xf001, 0x1922, 0x1762, 0xf001, 0x1672, 0x1292, 0xf001, 0x1912, 0x1091, 0xf001, 0x1842,
    0x1482, 0xf001, 0x1752, 0x1572, 0xf001, 0x1832, 0x1382, 0xf001, 0x1662, 0x1742, 0xf001, 0x1472,
    0x1801, 0xf001, 0x1081, 0x1652, 0xf001, 0x1562, 0x1732, 0xf001, 0x1372, 0x1642, 0xf001, 0x1701,
    0x1071, 0xf001, 0x1601, 0x1061, /* huffTable16[651] */
    0xf008, 0x0101, 0x010a, 0x0113, 0x8ff2, 0x0118, 0x011d, 0x0120, 0x82f2, 0x0131, 0x8f12, 0x81f2,
    0x0134, 0x0145, 0x0156, 0x0167, 0x0178, 0x0189, 0x019a, 0x01a3, 0x01ac, 0x01b5, 0x01be, 0x01c7,
    0x01d0, 0x01d9, 0x01de, 0x01e3, 0x01e6, 0x01eb, 0x01f0, 0x8152, 0x01f3, 0x01f6, 0x01f9, 0x01fc,
    0x8412, 0x8142, 0x01ff, 0x8322, 0x8232, 0x7312, 0x7312, 0x7132, 0x7132, 0x8301, 0x8031, 0x7222,
    0x7222, 0x6212, 0x6212, 0x6212, 0x6212, 0x6122, 0x6122, 0x6122, 0x6122, 0x6201, 0x6201, 0x6201,
    0x6201, 0x6021, 0x6021, 0x6021, 0x6021, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112,
    0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4101, 0x4101, 0x4101,
    0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101,
    0x4101, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011,
    0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x3011, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0x1000,
    0x1000, 0x1000, 0x1000, 0x1000, 0x1000, 0xf003, 0x3fe2, 0x3ef2, 0x3fd2, 0x3df2, 0x3fc2, 0x3cf2,
    0x3fb2, 0x3bf2, 0xf003, 0x2fa2, 0x2fa2, 0x3af2, 0x3f92, 0x39f2, 0x38f2, 0x2f82, 0x2f82, 0xf002,
    0x2f72, 0x27f2, 0x2f62, 0x26f2, 0xf002, 0x2f52, 0x25f2, 0x1f42, 0x1f42, 0xf001, 0x14f2, 0x13f2,
    0xf004, 0x10f1, 0x10f1, 0x10f1, 0x10f1, 0x10f1, 0x10f1, 0x10f1, 0x10f1, 0x2f32, 0x2f32, 0x2f32,
    0x2f32, 0x00e2, 0x00f3, 0x00fc, 0x0105, 0xf001, 0x1f22, 0x1f01, 0xf004, 0x00fa, 0x00ff, 0x0104,
    0x0109, 0x010c, 0x0111, 0x0116, 0x0119, 0x011e, 0x0123, 0x0128, 0x43e2, 0x012d, 0x0130, 0x0133,
    0x0136, 0xf004, 0x0128, 0x012b, 0x012e, 0x4d01, 0x0131, 0x0134, 0x0137, 0x4c32, 0x013a, 0x4c12,
    0x40c1, 0x013d, 0x32e2, 0x32e2, 0x4e22, 0x4e12, 0xf004, 0x43d2, 0x4d22, 0x42d2, 0x41d2, 0x4b32,
    0x012f, 0x3d12, 0x3d12, 0x44c2, 0x4b62, 0x43c2, 0x47a2, 0x3c22, 0x3c22, 0x42c2, 0x45b2, 0xf004,
    0x41c2, 0x4c01, 0x4b42, 0x44b2, 0x4a62, 0x46a2, 0x33b2, 0x33b2, 0x4a52, 0x45a2, 0x3b22, 0x3b22,
    0x32b2, 0x32b2, 0x3b12, 0x3b12, 0xf004, 0x31b2, 0x31b2, 0x4b01, 0x40b1, 0x4962, 0x4692, 0x4a42,
    0x44a2, 0x4872, 0x4782, 0x33a2, 0x33a2, 0x4a32, 0x4952, 0x3a22, 0x3a22, 0xf004, 0x4592, 0x4862,
    0x31a2, 0x31a2, 0x4682, 0x4772, 0x3492, 0x3492, 0x4942, 0x4752, 0x3762, 0x3762, 0x22a2, 0x22a2,
    0x22a2, 0x22a2, 0xf003, 0x2a12, 0x2a12, 0x3a01, 0x30a1, 0x3932, 0x3392, 0x3852, 0x3582, 0xf003,
    0x2922, 0x2922, 0x2292, 0x2292, 0x3672, 0x3901, 0x2912, 0x2912, 0xf003, 0x2192, 0x2192, 0x3091,
    0x3842, 0x3482, 0x3572, 0x3832, 0x3382, 0xf003, 0x3662, 0x3822, 0x2282, 0x2282, 0x3742, 0x3472,
    0x2812, 0x2812, 0xf003, 0x2182, 0x2182, 0x2081, 0x2081, 0x3801, 0x3652, 0x2732, 0x2732, 0xf003,
    0x2372, 0x2372, 0x3562, 0x3642, 0x2722, 0x2722, 0x2272, 0x2272, 0xf003, 0x3462, 0x3552, 0x2701,
    0x2701, 0x1712, 0x1712, 0x1712, 0x1712, 0xf002, 0x1172, 0x1172, 0x2071, 0x2632, 0xf002, 0x2362,
    0x2542, 0x2452, 0x2622, 0xf001, 0x1262, 0x1612, 0xf002, 0x1162, 0x1162, 0x2601, 0x2061, 0xf002,
    0x1352, 0x1352, 0x2532, 0x2442, 0xf001, 0x1522, 0x1252, 0xf001, 0x1512, 0x1501, 0xf001, 0x1432,
    0x1342, 0xf001, 0x1051, 0x1422, 0xf001, 0x1242, 0x1332, 0xf001, 0x1401, 0x1041, 0xf004, 0x4ec2,
    0x0086, 0x3ed2, 0x3ed2, 0x39e2, 0x39e2, 0x4ae2, 0x49d2, 0x2ee2, 0x2ee2, 0x2ee2, 0x2ee2, 0x3de2,
    0x3de2, 0x3be2, 0x3be2, 0xf003, 0x2eb2, 0x2eb2, 0x2dc2, 0x2dc2, 0x3cd2, 0x3bd2, 0x2ea2, 0x2ea2,
    0xf003, 0x2cc2, 0x2cc2, 0x3da2, 0x3ad2, 0x3e72, 0x3ca2, 0x2ac2, 0x2ac2, 0xf003, 0x39c2, 0x3d72,
    0x2e52, 0x2e52, 0x1db2, 0x1db2, 0x1db2, 0x1db2, 0xf002, 0x1e92, 0x1e92, 0x2cb2, 0x2bc2, 0xf002,
    0x2e82, 0x28e2, 0x2d92, 0x27e2, 0xf002, 0x2bb2, 0x2d82, 0x28d2, 0x2e62, 0xf001, 0x16e2, 0x1c92,
    0xf002, 0x2ba2, 0x2ab2, 0x25e2, 0x27d2, 0xf002, 0x1e42, 0x1e42, 0x24e2, 0x2c82, 0xf001, 0x18c2,
    0x1e32, 0xf002, 0x1d62, 0x1d62, 0x26d2, 0x2b92, 0xf002, 0x29b2, 0x2aa2, 0x11e2, 0x11e2, 0xf002,
    0x14d2, 0x14d2, 0x28b2, 0x29a2, 0xf002, 0x1b72, 0x1b72, 0x27b2, 0x20d1, 0xf001, 0x1e01, 0x10e1,
    0xf001, 0x1d52, 0x15d2, 0xf001, 0x1c72, 0x17c2, 0xf001, 0x1d42, 0x1b82, 0xf001, 0x1a92, 0x1c62,
    0xf001, 0x16c2, 0x1d32, 0xf001, 0x1c52, 0x15c2, 0xf001, 0x1a82, 0x18a2, 0xf001, 0x1992, 0x1c42,
    0xf001, 0x16b2, 0x1a72, 0xf001, 0x1b52, 0x1982, 0xf001, 0x1892, 0x1972, 0xf001, 0x1792, 0x1882,
    0xf001, 0x1ce2, 0x1dd2, /* huffTable24[705] */
    0xf009, 0x8fe2, 0x8fe2, 0x8ef2, 0x8ef2, 0x8fd2, 0x8fd2, 0x8df2, 0x8df2, 0x8fc2, 0x8fc2, 0x8cf2,
    0x8cf2, 0x8fb2, 0x8fb2, 0x8bf2, 0x8bf2, 0x7af2, 0x7af2, 0x7af2, 0x7af2, 0x8fa2, 0x8fa2, 0x8f92,
    0x8f92, 0x79f2, 0x79f2, 0x79f2, 0x79f2, 0x78f2, 0x78f2, 0x78f2, 0x78f2, 0x8f82, 0x8f82, 0x8f72,
    0x8f72, 0x77f2, 0x77f2, 0x77f2, 0x77f2, 0x7f62, 0x7f62, 0x7f62, 0x7f62, 0x76f2, 0x76f2, 0x76f2,
    0x76f2, 0x7f52, 0x7f52, 0x7f52, 0x7f52, 0x75f2, 0x75f2, 0x75f2, 0x75f2, 0x7f42, 0x7f42, 0x7f42,
    0x7f42, 0x74f2, 0x74f2, 0x74f2, 0x74f2, 0x7f32, 0x7f32, 0x7f32, 0x7f32, 0x73f2, 0x73f2, 0x73f2,
    0x73f2, 0x7f22, 0x7f22, 0x7f22, 0x7f22, 0x72f2, 0x72f2, 0x72f2, 0x72f2, 0x71f2, 0x71f2, 0x71f2,
    0x71f2, 0x8f12, 0x8f12, 0x80f1, 0x80f1, 0x9f01, 0x0201, 0x0206, 0x020b, 0x0210, 0x0215, 0x021a,
    0x021f, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2,
    0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2,
    0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x4ff2, 0x0224, 0x0229, 0x0232,
    0x0237, 0x023a, 0x023f, 0x0242, 0x0245, 0x024a, 0x024d, 0x0250, 0x0253, 0x0256, 0x0259, 0x025c,
    0x025f, 0x0262, 0x0265, 0x0268, 0x026b, 0x026e, 0x0271, 0x0274, 0x0277, 0x027a, 0x027d, 0x0280,
    0x0283, 0x0288, 0x028b, 0x028e, 0x0291, 0x0294, 0x0297, 0x029a, 0x029f, 0x94b2, 0x02a4, 0x02a7,
    0x02aa, 0x93b2, 0x9882, 0x02af, 0x92b2, 0x02b2, 0x02b5, 0x9692, 0x94a2, 0x02b8, 0x9782, 0x9a32,
    0x93a2, 0x9952, 0x9592, 0x9a22, 0x92a2, 0x91a2, 0x9862, 0x9682, 0x9772, 0x9942, 0x9492, 0x9932,
    0x9392, 0x9852, 0x9582, 0x9922, 0x9762, 0x9672, 0x9292, 0x9912, 0x9192, 0x9842, 0x9482, 0x9752,
    0x9572, 0x9832, 0x9382, 0x9662, 0x9822, 0x9282, 0x9812, 0x9742, 0x9472, 0x9182, 0x02bb, 0x9652,
    0x9562, 0x9712, 0x02be, 0x8372, 0x8372, 0x9732, 0x9722, 0x8272, 0x8272, 0x8642, 0x8642, 0x8462,
    0x8462, 0x8552, 0x8552, 0x8172, 0x8172, 0x8632, 0x8632, 0x8362, 0x8362, 0x8542, 0x8542, 0x8452,
    0x8452, 0x8622, 0x8622, 0x8262, 0x8262, 0x8612, 0x8612, 0x8162, 0x8162, 0x9601, 0x9061, 0x8532,
    0x8532, 0x8352, 0x8352, 0x8442, 0x8442, 0x8522, 0x8522, 0x8252, 0x8252, 0x8512, 0x8512, 0x9501,
    0x9051, 0x7152, 0x7152, 0x7152, 0x7152, 0x8432, 0x8432, 0x8342, 0x8342, 0x7422, 0x7422, 0x7422,
    0x7422, 0x7242, 0x7242, 0x7242, 0x7242, 0x7332, 0x7332, 0x7332, 0x7332, 0x7412, 0x7412, 0x7412,
    0x7412, 0x7142, 0x7142, 0x7142, 0x7142, 0x8401, 0x8401, 0x8041, 0x8041, 0x7322, 0x7322, 0x7322,
    0x7322, 0x7232, 0x7232, 0x7232, 0x7232, 0x6312, 0x6312, 0x6312, 0x6312, 0x6312, 0x6312, 0x6312,
    0x6312, 0x6132, 0x6132, 0x6132, 0x6132, 0x6132, 0x6132, 0x6132, 0x6132, 0x7301, 0x7301, 0x7301,
    0x7301, 0x7031, 0x7031, 0x7031, 0x7031, 0x6222, 0x6222, 0x6222, 0x6222, 0x6222, 0x6222, 0x6222,
    0x6222, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5212,
    0x5212, 0x5212, 0x5212, 0x5212, 0x5212, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122,
    0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x5122, 0x6201, 0x6201, 0x6201,
    0x6201, 0x6201, 0x6201, 0x6201, 0x6201, 0x6021, 0x6021, 0x6021, 0x6021, 0x6021, 0x6021, 0x6021,
    0x6021, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112,
    0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112,
    0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4112, 0x4101, 0x4101, 0x4101,
    0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101,
    0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4101,
    0x4101, 0x4101, 0x4101, 0x4101, 0x4101, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011,
    0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011,
    0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011, 0x4011,
    0x4011, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000,
    0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000,
    0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0x4000, 0xf002, 0x2ee2, 0x2ed2,
    0x2de2, 0x2ec2, 0xf002, 0x2ce2, 0x2dd2, 0x2eb2, 0x2be2, 0xf002, 0x2dc2, 0x2cd2, 0x2ea2, 0x2ae2,
    0xf002, 0x2db2, 0x2bd2, 0x2cc2, 0x2e92, 0xf002, 0x29e2, 0x2da2, 0x2ad2, 0x2cb2, 0xf002, 0x2bc2,
    0x2e82, 0x28e2, 0x2d92, 0xf002, 0x29d2, 0x2e72, 0x27e2, 0x2ca2, 0xf002, 0x2ac2, 0x2bb2, 0x2d82,
    0x28d2, 0xf003, 0x3e01, 0x30e1, 0x2d01, 0x2d01, 0x16e2, 0x16e2, 0x16e2, 0x16e2, 0xf002, 0x2e62,
    0x2c92, 0x19c2, 0x19c2, 0xf001, 0x1e52, 0x1ab2, 0xf002, 0x15e2, 0x15e2, 0x2ba2, 0x2d72, 0xf001,
    0x17d2, 0x14e2, 0xf001, 0x1c82, 0x18c2, 0xf002, 0x2e42, 0x2e22, 0x1e32, 0x1e32, 0xf001, 0x1d62,
    0x16d2, 0xf001, 0x13e2, 0x1b92, 0xf001, 0x19b2, 0x1aa2, 0xf001, 0x12e2, 0x1e12, 0xf001, 0x11e2,
    0x1d52, 0xf001, 0x15d2, 0x1c72, 0xf001, 0x17c2, 0x1d42, 0xf001, 0x1b82, 0x18b2, 0xf001, 0x14d2,
    0x1a92, 0xf001, 0x19a2, 0x1c62, 0xf001, 0x16c2, 0x1d32, 0xf001, 0x13d2, 0x1d22, 0xf001, 0x12d2,
    0x1d12, 0xf001, 0x1b72, 0x17b2, 0xf001, 0x11d2, 0x1c52, 0xf001, 0x15c2, 0x1a82, 0xf001, 0x18a2,
    0x1992, 0xf001, 0x1c42, 0x14c2, 0xf001, 0x1b62, 0x16b2, 0xf002, 0x20d1, 0x2c01, 0x1c32, 0x1c32,
    0xf001, 0x13c2, 0x1a72, 0xf001, 0x17a2, 0x1c22, 0xf001, 0x12c2, 0x1b52, 0xf001, 0x15b2, 0x1c12,
    0xf001, 0x1982, 0x1892, 0xf001, 0x11c2, 0x1b42, 0xf002, 0x20c1, 0x2b01, 0x1b32, 0x1b32, 0xf002,
    0x20b1, 0x2a01, 0x1a12, 0x1a12, 0xf001, 0x1a62, 0x16a2, 0xf001, 0x1972, 0x1792, 0xf002, 0x20a1,
    0x2901, 0x1091, 0x1091, 0xf001, 0x1b22, 0x1a52, 0xf001, 0x15a2, 0x1b12, 0xf001, 0x11b2, 0x1962,
    0xf001, 0x1a42, 0x1872, 0xf001, 0x1801, 0x1081, 0xf001, 0x1701, 0x1071,
];

const HUFF_OFFSET_01: u16 = 0;
const HUFF_OFFSET_02: u16 = 9 + HUFF_OFFSET_01;
const HUFF_OFFSET_03: u16 = 65 + HUFF_OFFSET_02;
const HUFF_OFFSET_05: u16 = 65 + HUFF_OFFSET_03;
const HUFF_OFFSET_06: u16 = 257 + HUFF_OFFSET_05;
const HUFF_OFFSET_07: u16 = 129 + HUFF_OFFSET_06;
const HUFF_OFFSET_08: u16 = 110 + HUFF_OFFSET_07;
const HUFF_OFFSET_09: u16 = 280 + HUFF_OFFSET_08;
const HUFF_OFFSET_10: u16 = 93 + HUFF_OFFSET_09;
const HUFF_OFFSET_11: u16 = 320 + HUFF_OFFSET_10;
const HUFF_OFFSET_12: u16 = 296 + HUFF_OFFSET_11;
const HUFF_OFFSET_13: u16 = 185 + HUFF_OFFSET_12;
const HUFF_OFFSET_15: u16 = 497 + HUFF_OFFSET_13;
const HUFF_OFFSET_16: u16 = 580 + HUFF_OFFSET_15;
const HUFF_OFFSET_24: u16 = 651 + HUFF_OFFSET_16;

const HUFF_TAB_OFFSET: [u16; HUFF_PAIRTABS] = [
    0,
    HUFF_OFFSET_01,
    HUFF_OFFSET_02,
    HUFF_OFFSET_03,
    0,
    HUFF_OFFSET_05,
    HUFF_OFFSET_06,
    HUFF_OFFSET_07,
    HUFF_OFFSET_08,
    HUFF_OFFSET_09,
    HUFF_OFFSET_10,
    HUFF_OFFSET_11,
    HUFF_OFFSET_12,
    HUFF_OFFSET_13,
    0,
    HUFF_OFFSET_15,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_16,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
    HUFF_OFFSET_24,
];

const HUFF_TAB_LOOKUP: [HuffTabLookup; HUFF_PAIRTABS] = [
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::NoBits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::OneShot,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::OneShot,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::OneShot,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::InvalidTab,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::OneShot,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::OneShot,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::InvalidTab,
    },
    HuffTabLookup {
        lin_bits: 0,
        tab_type: HuffTabType::LoopNoLinbits,
    },
    HuffTabLookup {
        lin_bits: 1,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 2,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 3,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 4,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 6,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 8,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 10,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 13,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 4,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 5,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 6,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 7,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 8,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 9,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 11,
        tab_type: HuffTabType::LoopLinbits,
    },
    HuffTabLookup {
        lin_bits: 13,
        tab_type: HuffTabType::LoopLinbits,
    },
];

const QUAD_TAB_OFFSET: [i32; 2] = [0, 64];
const QUAD_TAB_MAX_BITS: [i32; 2] = [6, 4];

pub unsafe fn DecodeHuffmanPairs(
    mut xy: &mut [i32],
    tab_idx: i32,
    mut bits_left: i32,
    mut buf: *const u8,
    bit_offset: i32,
) -> i32 {
    let mut x: i32;
    let mut y: i32;
    let mut cachedBits: i32;
    let mut padBits: i32;
    let mut len: i32;
    let startBits: i32;
    let linBits: i32;
    let mut maxBits: i32;
    let mut minBits: i32;
    let tabType: HuffTabType; // HuffTabType_t
    let mut cw: u16;
    let tBase: *const u16;
    let mut tCurr: *const u16;
    let mut cache: u32;

    let n_vals = xy.len();

    if xy.is_empty() {
        return 0;
    }

    if bits_left < 0 {
        return -1;
    }
    startBits = bits_left;

    // Uzyskiwanie dostępu do tablic huffmana (zakładam nazwy z Twojego kodu)
    tBase = (HUFF_TABLE.as_ptr() as *const u16).add(HUFF_TAB_OFFSET[tab_idx as usize] as usize);
    linBits = HUFF_TAB_LOOKUP[tab_idx as usize].lin_bits as i32;
    tabType = HUFF_TAB_LOOKUP[tab_idx as usize].tab_type;

    /* Walidacja - zachowanie logiki z log_i */
    if (n_vals & 0x01) != 0 {
        return -1;
    }
    if tab_idx >= HUFF_PAIRTABS as i32 {
        return -1;
    }
    if tab_idx < 0 {
        return -1;
    }
    if tabType == HuffTabType::InvalidTab {
        return -1;
    }

    /* initially fill cache with any partial byte */
    cache = 0;
    cachedBits = (8 - bit_offset) & 0x07;
    if cachedBits != 0 {
        cache = (*buf as u32) << (32 - cachedBits);
        buf = buf.add(1);
    }
    bits_left -= cachedBits;

    match tabType {
        HuffTabType::NoBits => {
            for i in (0..n_vals).step_by(2) {
                xy[i as usize] = 0;
                xy[(i + 1) as usize] = 0;
            }
            return 0;
        }
        HuffTabType::OneShot => {
            maxBits = ((*(tBase) >> 0) & 0x000f) as i32;
            let tBase_one_shot = tBase.add(1);
            padBits = 0;

            while xy.len() > 0 {
                if bits_left >= 16 {
                    cache |= (*buf as u32) << (24 - cachedBits);
                    buf = buf.add(1);
                    cache |= (*buf as u32) << (16 - cachedBits);
                    buf = buf.add(1);
                    cachedBits += 16;
                    bits_left -= 16;
                } else {
                    if cachedBits + bits_left <= 0 {
                        return -1;
                    }
                    if bits_left > 0 {
                        cache |= (*buf as u32) << (24 - cachedBits);
                        buf = buf.add(1);
                    }
                    if bits_left > 8 {
                        cache |= (*buf as u32) << (16 - cachedBits);
                        buf = buf.add(1);
                    }
                    cachedBits += bits_left;
                    bits_left = 0;

                    cache &= (0x80000000u32 as i32 >> (cachedBits - 1)) as u32;
                    padBits = 11;
                    cachedBits += padBits;
                }

                while xy.len() > 0 && cachedBits >= 11 {
                    cw = (*tBase_one_shot.add((cache >> (32 - maxBits)) as usize));

                    len = ((cw >> 12) & 0x000f) as i32;
                    cachedBits -= len;
                    cache <<= len;

                    x = ((cw >> 4) & 0x000f) as i32;
                    if x != 0 {
                        x |= (cache & 0x80000000) as i32;
                        cache <<= 1;
                        cachedBits -= 1;
                    }

                    y = ((cw >> 8) & 0x000f) as i32;
                    if y != 0 {
                        y |= (cache & 0x80000000) as i32;
                        cache <<= 1;
                        cachedBits -= 1;
                    }

                    if cachedBits < padBits {
                        return -1;
                    }

                    xy[0] = x;
                    xy[1] = y;
                    xy = &mut xy[2..];
                }
            }
            bits_left += cachedBits - padBits;
            return startBits - bits_left;
        }
        HuffTabType::LoopLinbits | HuffTabType::LoopNoLinbits => {
            tCurr = tBase;
            padBits = 0;
            while xy.len() > 0 {
                if bits_left >= 16 {
                    cache |= (*buf as u32) << (24 - cachedBits);
                    buf = buf.add(1);
                    cache |= (*buf as u32) << (16 - cachedBits);
                    buf = buf.add(1);
                    cachedBits += 16;
                    bits_left -= 16;
                } else {
                    if cachedBits + bits_left <= 0 {
                        return -1;
                    }
                    if bits_left > 0 {
                        cache |= (*buf as u32) << (24 - cachedBits);
                        buf = buf.add(1);
                    }
                    if bits_left > 8 {
                        cache |= (*buf as u32) << (16 - cachedBits);
                        buf = buf.add(1);
                    }
                    cachedBits += bits_left;
                    bits_left = 0;
                    cache &= (0x80000000u32 as i32 >> (cachedBits - 1)) as u32;
                    padBits = 11;
                    cachedBits += padBits;
                }

                while xy.len() > 0 && cachedBits >= 11 {
                    maxBits = (*tCurr & 0x000f) as i32;
                    cw = *(tCurr.add(((cache >> (32 - maxBits)) + 1) as usize));
                    len = ((cw >> 12) & 0x000f) as i32;

                    if len == 0 {
                        cachedBits -= maxBits;
                        cache <<= maxBits;
                        tCurr = tCurr.add(cw as usize);
                        continue;
                    }
                    cachedBits -= len;
                    cache <<= len;

                    x = ((cw >> 4) & 0x000f) as i32;
                    y = ((cw >> 8) & 0x000f) as i32;

                    if x == 15 && tabType == HuffTabType::LoopLinbits {
                        minBits = linBits + 1 + (if y != 0 { 1 } else { 0 });
                        if cachedBits + bits_left < minBits {
                            return -1;
                        }
                        while cachedBits < minBits {
                            cache |= (*buf as u32) << (24 - cachedBits);
                            buf = buf.add(1);
                            cachedBits += 8;
                            bits_left -= 8;
                        }
                        if bits_left < 0 {
                            cachedBits += bits_left;
                            bits_left = 0;
                            cache &= (0x80000000u32 as i32 >> (cachedBits - 1)) as u32;
                        }
                        x += (cache >> (32 - linBits as u32)) as i32;
                        cachedBits -= linBits;
                        cache <<= linBits;
                    }
                    if x != 0 {
                        x |= (cache & 0x80000000) as i32;
                        cache <<= 1;
                        cachedBits -= 1;
                    }

                    if y == 15 && tabType == HuffTabType::LoopLinbits {
                        minBits = linBits + 1;
                        if cachedBits + bits_left < minBits {
                            return -1;
                        }
                        while cachedBits < minBits {
                            cache |= (*buf as u32) << (24 - cachedBits);
                            buf = buf.add(1);
                            cachedBits += 8;
                            bits_left -= 8;
                        }
                        if bits_left < 0 {
                            cachedBits += bits_left;
                            bits_left = 0;
                            cache &= (0x80000000u32 as i32 >> (cachedBits - 1)) as u32;
                        }
                        y += (cache >> (32 - linBits as u32)) as i32;
                        cachedBits -= linBits;
                        cache <<= linBits;
                    }
                    if y != 0 {
                        y |= (cache & 0x80000000) as i32;
                        cache <<= 1;
                        cachedBits -= 1;
                    }

                    if cachedBits < padBits {
                        return -1;
                    }

                    xy[0] = x;
                    xy[1] = y;
                    xy = &mut xy[2..];
                    tCurr = tBase;
                }
            }
            bits_left += cachedBits - padBits;
            return startBits - bits_left;
        }
        _ => return -1,
    }
}

/* tables for quadruples
 * format 0xAB
 *  A = length of codeword
 *  B = codeword
 */
const QUAD_TABLE: [u8; 64 + 16] = [
    /* table A */
    0x6b, 0x6f, 0x6d, 0x6e, 0x67, 0x65, 0x59, 0x59, 0x56, 0x56, 0x53, 0x53, 0x5a, 0x5a, 0x5c, 0x5c,
    0x42, 0x42, 0x42, 0x42, 0x41, 0x41, 0x41, 0x41, 0x44, 0x44, 0x44, 0x44, 0x48, 0x48, 0x48, 0x48,
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
    0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x10,
    /* table B */
    0x4f, 0x4e, 0x4d, 0x4c, 0x4b, 0x4a, 0x49, 0x48, 0x47, 0x46, 0x45, 0x44, 0x43, 0x42, 0x41, 0x40,
];

/***********************************************************************************************************************
 * Function:    DecodeHuffmanQuads
 *
 * Description: decode 4-way vector Huffman codes in the "count1" region of spectrum
 *
 * Inputs:      valid BitStreamInfo struct, pointing to start of quadword codes
 *              pointer to vwxy buffer to received decoded values
 *              maximum number of codewords to decode
 *              index of quadword table (0 = table A, 1 = table B)
 *              number of bits remaining in bitstream
 *
 * Outputs:     quadruples of decoded coefficients in vwxy
 *              updated BitStreamInfo struct
 *
 * Return:      index of the first "zero_part" value (index of the first sample
 *                of the quad word after which all samples are 0)
 *
 * Notes:        si_huff.bit tests every vwxy output in both quad tables
 **********************************************************************************************************************/
// no improvement with section=data
pub unsafe fn DecodeHuffmanQuads(
    vwxy: &mut [i32],
    n_vals: i32,
    tab_idx: i32,
    mut bits_left: i32,
    mut buf: *mut u8,
    bit_offset: i32,
) -> i32 {
    let mut v: i32;
    let mut w: i32;
    let mut x: i32;
    let mut y: i32;
    let mut len: i32;
    let max_bits: i32;
    let mut cached_bits: i32;
    let mut pad_bits: i32;
    let mut cache: u32;
    let mut cw: u8;

    if bits_left <= 0 {
        return 0;
    }

    let mut vwxy_idx = 0;

    // Pobieranie bazy tabeli i parametrów (zakładamy dostęp do globalnych tablic)
    // tBase = (unsigned char *) quadTable + quadTabOffset[tabIdx];
    let t_base = &QUAD_TABLE[QUAD_TAB_OFFSET[tab_idx as usize] as usize..];
    max_bits = QUAD_TAB_MAX_BITS[tab_idx as usize] as i32;

    /* Inicjalizacja cache partial byte */
    cache = 0;
    cached_bits = (8 - bit_offset) & 0x07;
    if cached_bits != 0 {
        cache = (*buf as u32) << (32 - cached_bits);
        buf = buf.add(1);
    }
    bits_left -= cached_bits;

    let mut i = 0;
    pad_bits = 0;

    while i < (n_vals - 3) {
        /* Uzupełnianie cache - ładowanie 16 bitów */
        if bits_left >= 16 {
            cache |= (*buf as u32) << (24 - cached_bits);
            buf = buf.add(1);
            cache |= (*buf as u32) << (16 - cached_bits);
            buf = buf.add(1);
            cached_bits += 16;
            bits_left -= 16;
        } else {
            /* Ostatnia partia bitów, wyrównanie i padBits */
            if cached_bits + bits_left <= 0 {
                return i;
            }
            if bits_left > 0 {
                cache |= (*buf as u32) << (24 - cached_bits);
                buf = buf.add(1);
            }
            if bits_left > 8 {
                cache |= (*buf as u32) << (16 - cached_bits);
                buf = buf.add(1);
            }
            cached_bits += bits_left;
            bits_left = 0;

            // cache &= (signed int) 0x80000000 >> (cachedBits - 1);
            // W Rust przesunięcie i32 jest arytmetyczne (zachowuje bit znaku)
            let mask = ((0x80000000u32 as i32) >> (cached_bits.wrapping_sub(1))) as u32;
            cache &= mask;

            pad_bits = 10;
            cached_bits += pad_bits;
        }

        /* Dekodowanie kwadratów */
        while i < (n_vals - 3) && cached_bits >= 10 {
            // cw = pgm_read_byte(&tBase[cache >> (32 - maxBits)]);
            cw = t_base[(cache >> (32 - max_bits)) as usize];

            len = ((cw >> 4) & 0x0f) as i32;
            cached_bits -= len;
            cache <<= len;

            // V
            v = ((cw >> 3) & 0x01) as i32;
            if v != 0 {
                v |= (cache & 0x80000000) as i32;
                cache <<= 1;
                cached_bits -= 1;
            }

            // W
            w = ((cw >> 2) & 0x01) as i32;
            if w != 0 {
                w |= (cache & 0x80000000) as i32;
                cache <<= 1;
                cached_bits -= 1;
            }

            // X
            x = ((cw >> 1) & 0x01) as i32;
            if x != 0 {
                x |= (cache & 0x80000000) as i32;
                cache <<= 1;
                cached_bits -= 1;
            }

            // Y
            y = ((cw >> 0) & 0x01) as i32;
            if y != 0 {
                y |= (cache & 0x80000000) as i32;
                cache <<= 1;
                cached_bits -= 1;
            }

            if cached_bits < pad_bits {
                return i;
            }

            // Zapis do bufora i inkrementacja wskaźnika (jak vwxy++)
            vwxy[vwxy_idx] = v;
            vwxy[vwxy_idx + 1] = w;
            vwxy[vwxy_idx + 2] = x;
            vwxy[vwxy_idx + 3] = y;
            i += 4;
            vwxy_idx += 4;
        }
    }

    i
}

/***********************************************************************************************************************
 * Function:    DecodeHuffman
 *
 * Description: decode one granule, one channel worth of Huffman codes
 *
 * Inputs:      MP3DecInfo structure filled by UnpackFrameHeader(), UnpackSideInfo(),
 *                and UnpackScaleFactors() (for this granule)
 *              buffer pointing to start of Huffman data in MP3 frame
 *              pointer to bit offset (0-7) indicating starting bit in buf[0]
 *              number of bits in the Huffman data section of the frame
 *                (could include padding bits)
 *              index of current granule and channel
 *
 * Outputs:     decoded coefficients in hi->huffDecBuf[ch] (hi pointer in mp3DecInfo)
 *              updated bitOffset
 *
 * Return:      length (in bytes) of Huffman codes
 *              bitOffset also returned in parameter (0 = MSB, 7 = LSB of
 *                byte located at buf + offset)
 *              -1 if null input pointers, huffBlockBits < 0, or decoder runs
 *                out of bits prematurely (invalid bitstream)
 **********************************************************************************************************************/

pub unsafe fn DecodeHuffman(
    mut buf: *mut u8,
    bitOffset: *mut i32,
    huffBlockBits: i32,
    gr: GranuleIndex,
    ch: ChannelIndex,
    m_HuffmanInfo: &mut HuffmanInfo,
    m_SFBandTable: &SFBandTable, // SFBandTable_t*
    m_SideInfoSub: &mut [[SideInfoSub; MAX_NCHAN]; MAX_NGRAN],
    m_MPEGVersion: MPEGVersion,
) -> i32 {
    let mut rEnd = [0; 4];
    let r1_start;
    let r2_start;
    let w;
    let mut bitsLeft;
    let startBuf = buf;

    let sis = &m_SideInfoSub[gr as usize][ch as usize];

    if huffBlockBits < 0 {
        return -1;
    }

    if sis.win_switch_flag != 0 && sis.blockType == BlockType::Short {
        // Short blocks lub mixed blocks
        if sis.mixedBlock == 0 {
            // Czyste short blocks
            r1_start = m_SFBandTable.s[((sis.region0Count + 1) / 3) as usize] as i32 * 3;
        } else {
            // Mixed block
            if m_MPEGVersion == MPEGVersion::MPEG1 {
                r1_start = m_SFBandTable.l[(sis.region0Count + 1) as usize] as i32;
            } else {
                // MPEG2 / MPEG2.5 – spec wymaga specjalnego obliczenia
                w = m_SFBandTable.s[4] as i32 - m_SFBandTable.s[3] as i32;
                r1_start = m_SFBandTable.l[6] as i32 + 2 * w;
            }
        }
        r2_start = MAX_NSAMP as i32; // short blocks nie mają regionu 2
    } else {
        // Long blocks
        r1_start = m_SFBandTable.l[(sis.region0Count + 1) as usize] as i32;
        r2_start = m_SFBandTable.l[(sis.region0Count + 1 + sis.region1Count + 1) as usize] as i32;
    }

    /* offset rEnd index by 1 so first region = rEnd[1] - rEnd[0], etc. */
    rEnd[3] = if MAX_NSAMP < (2 * sis.n_bigvals as usize) {
        MAX_NSAMP as i32
    } else {
        2 * sis.n_bigvals
    };
    rEnd[2] = if r2_start < rEnd[3] {
        r2_start
    } else {
        rEnd[3]
    };
    rEnd[1] = if r1_start < rEnd[3] {
        r1_start
    } else {
        rEnd[3]
    };
    rEnd[0] = 0;

    /* rounds up to first all-zero pair (we don't check last pair for (x,y) == (non-zero, zero)) */
    (*m_HuffmanInfo).non_zero_bound[ch as usize] = rEnd[3];

    /* decode Huffman pairs (rEnd[i] are always even numbers) */
    bitsLeft = huffBlockBits;

    let mut bits_used;
    for i in 0..3 {
        bits_used = unsafe {
            DecodeHuffmanPairs(
                &mut m_HuffmanInfo.huff_dec_buf[ch as usize]
                    [rEnd[i] as usize..rEnd[i] as usize + (rEnd[i + 1] - rEnd[i]) as usize],
                sis.tableSelect[i],
                bitsLeft,
                buf,
                *bitOffset,
            )
        };
        if (bits_used < 0 || bits_used > bitsLeft)
        /* error - overran end of bitstream */
        {
            return -1;
        }

        /* update bitstream position */
        buf = buf.add((bits_used + *bitOffset) as usize >> 3);
        *bitOffset = (bits_used + *bitOffset) & 0x07;
        bitsLeft -= bits_used;
    }

    /* decode Huffman quads (if any) */
    m_HuffmanInfo.non_zero_bound[ch as usize] += unsafe {
        DecodeHuffmanQuads(
            &mut m_HuffmanInfo.huff_dec_buf[ch as usize][rEnd[3] as usize..],
            MAX_NSAMP as i32 - rEnd[3],
            sis.count1TableSelect,
            bitsLeft,
            buf,
            *bitOffset,
        )
    };

    assert!(m_HuffmanInfo.non_zero_bound[ch as usize] <= MAX_NSAMP as i32);

    for i in m_HuffmanInfo.non_zero_bound[ch as usize]..MAX_NSAMP as i32 {
        m_HuffmanInfo.huff_dec_buf[ch as usize][i as usize] = 0;
    }
    /* If bits used for 576 samples < huffBlockBits, then the extras are considered
     *  to be stuffing bits (throw away, but need to return correct bitstream position)
     */
    buf = buf.add((bitsLeft + *bitOffset) as usize >> 3);
    *bitOffset = (bitsLeft + *bitOffset) & 0x07;

    buf.offset_from(startBuf) as i32
}

//

/***********************************************************************************************************************
 * Function:    IMDCT36
 *
 * Description: 36-point modified DCT, with windowing and overlap-add (50% overlap)
 *
 * Inputs:      vector of 18 coefficients (N/2 inputs produces N outputs, by symmetry)
 *              overlap part of last IMDCT (9 samples - see output comments)
 *              window type (0,1,2,3) of current and previous block
 *              current block index (for deciding whether to do frequency inversion)
 *              number of guard bits in input vector
 *
 * Outputs:     18 output samples, after windowing and overlap-add with last frame
 *              second half of (unwindowed) 36-point IMDCT - save for next time
 *                only save 9 xPrev samples, using symmetry (see WinPrevious())
 *
 * Notes:       this is Ken's hyper-fast algorithm, including symmetric sin window
 *                optimization, if applicable
 *              total number of multiplies, general case:
 *                2*10 (idct9) + 9 (last stage imdct) + 36 (for windowing) = 65
 *              total number of multiplies, btCurr == 0 && btPrev == 0:
 *                2*10 (idct9) + 9 (last stage imdct) + 18 (for windowing) = 47
 *
 *              blockType == 0 is by far the most common case, so it should be
 *                possible to use the fast path most of the time
 *              this is the fastest known algorithm for performing
 *                long IMDCT + windowing + overlap-add in MP3
 *
 * Return:      mOut (OR of abs(y) for all y calculated here)
 **********************************************************************************************************************/
// barely faster in RAM
const C18: [u32; 9] = [
    0x7f834ed0, 0x7ba3751d, 0x7401e4c1, 0x68d9f964, 0x5a82799a, 0x496af3e2, 0x36185aee, 0x2120fb83,
    0x0b27eb5c,
];
const FAST_WIN36: [u32; 18] = [
    0x42aace8b, 0xc2e92724, 0x47311c28, 0xc95f619a, 0x4a868feb, 0xd0859d8c, 0x4c913b51, 0xd8243ea0,
    0x4d413ccc, 0xe0000000, 0x4c913b51, 0xe7dbc161, 0x4a868feb, 0xef7a6275, 0x47311c28, 0xf6a09e67,
    0x42aace8b, 0xfd16d8dd,
];
pub const IMDCT_WIN: [[u32; 36]; 4] = [
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

pub fn imdct36(
    x_curr: &mut [i32; BLOCK_SIZE],
    x_prev: &mut [i32; BLOCK_SIZE / 2],
    y: &mut [i32],
    bt_curr: BlockType,
    bt_prev: BlockType,
    block_idx: i32,
    gb: i32,
) -> i32 {
    let mut acc1 = 0;
    let mut acc2 = 0;
    let es;
    let mut x_buf: [i32; BLOCK_SIZE] = [0; 18];
    let mut x_prev_win: [i32; BLOCK_SIZE] = [0; 18];
    let mut c;
    let mut xo;
    let mut xe;
    let mut s;
    let mut d;
    let mut t;
    let mut y_lo;
    let mut y_hi;
    let mut x_curr_idx = 17;
    let mut x_prev_idx = 0;
    /* 7 gb is always adequate for antialias + accumulator loop + idct9 */
    if gb < 7 {
        /* rarely triggered - 5% to 10% of the time on normal clips (with Q25 input) */
        es = 7 - gb;
        for i in (0..=8).rev() {
            acc1 = ((x_curr[x_curr_idx]) >> es) - acc1;
            x_curr_idx -= 1;
            acc2 = acc1 - acc2;
            acc1 = ((x_curr[x_curr_idx]) >> es) - acc1;
            x_curr_idx -= 1;
            x_buf[i + 9] = acc2; /* odd */
            x_buf[i + 0] = acc1; /* even */
            x_prev[i] >>= es;
        }
    } else {
        es = 0;
        /* max gain = 18, assume adequate guard bits */
        for i in (0..=8).rev() {
            acc1 = (x_curr[x_curr_idx]) - acc1;
            x_curr_idx -= 1;
            acc2 = acc1 - acc2;
            acc1 = (x_curr[x_curr_idx]) - acc1;
            x_curr_idx -= 1;
            x_buf[i + 9] = acc2; /* odd */
            x_buf[i + 0] = acc1; /* even */
        }
    }
    /* xEven[0] and xOdd[0] scaled by 0.5 */
    x_buf[9] >>= 1;
    x_buf[0] >>= 1;
    let (spliced_buf, _) = x_buf.as_chunks_mut::<9>();
    /* do 9-point IDCT on even and odd */
    idct_9(&mut spliced_buf[0]); /* even */
    idct_9(&mut spliced_buf[1]); /* odd */
    let mut xp_idx = 8;
    let mut cp_idx = 8;
    let mut m_out = 0;
    if bt_prev == BlockType::Normal && bt_curr == BlockType::Normal {
        /* fast path - use symmetry of sin window to reduce windowing multiplies to 18 (N/2) */
        for (i, e) in FAST_WIN36.chunks_exact(2).enumerate() {
            /* do ARM-style pointer arithmetic (i still needed for y[] indexing - compiler spills if 2 y pointers) */
            c = C18[cp_idx];
            cp_idx -= 1;
            xo = x_buf[xp_idx + 9];
            xe = x_buf[xp_idx];
            xp_idx -= 1;
            /* gain 2 int bits here */
            xo = mulshift_32(c as i32, xo); /* 2*c18*xOdd (mul by 2 implicit in scaling)  */
            xe >>= 2;

            s = -(x_prev[x_prev_idx]); /* sum from last block (always at least 2 guard bits) */
            d = -(xe - xo); /* gain 2 int bits, don't shift xo (effective << 1 to eat sign bit, << 1 for mul by 2) */
            x_prev[x_prev_idx] = xe + xo; /* symmetry - xPrev[i] = xPrev[17-i] for long blocks */
            x_prev_idx += 1;
            t = s - d;

            y_lo = d + (mulshift_32(t, e[0] as i32) << 2);
            y_hi = s + (mulshift_32(t, e[1] as i32) << 2);
            y[(i) * NBANDS as usize] = y_lo;
            y[(17 - i) * NBANDS as usize] = y_hi;
            m_out |= y_lo.abs();
            m_out |= y_hi.abs();
        }
    } else {
        /* slower method - either prev or curr is using window type != 0 so do full 36-point window
         * output xPrevWin has at least 3 guard bits (xPrev has 2, gain 1 in WinPrevious)
         */
        win_previous(x_prev, &mut x_prev_win, bt_prev);

        let wp = IMDCT_WIN[bt_curr as usize];
        for i in 0..9 {
            c = C18[cp_idx];
            cp_idx -= 1;
            xo = x_buf[xp_idx + 9];
            xe = x_buf[xp_idx];
            xp_idx -= 1;
            /* gain 2 int bits here */
            xo = mulshift_32(c as i32, xo); /* 2*c18*xOdd (mul by 2 implicit in scaling)  */
            xe >>= 2;

            d = xe - xo;
            x_prev[x_prev_idx] = xe + xo; /* symmetry - xPrev[i] = xPrev[17-i] for long blocks */
            x_prev_idx += 1;

            y_lo = (x_prev_win[i] + mulshift_32(d, wp[i] as i32)) << 2;
            y_hi = (x_prev_win[17 - i] + mulshift_32(d, wp[17 - i] as i32)) << 2;
            y[(i) * NBANDS] = y_lo;
            y[(17 - i) * NBANDS] = y_hi;
            m_out |= y_lo.abs();
            m_out |= y_hi.abs();
        }
    }

    m_out |= freq_invert_rescale(y, x_prev, block_idx, es);

    m_out
}

/***********************************************************************************************************************
 * Function:    AntiAlias
 *
 * Description: smooth transition across DCT block boundaries (every 18 coefficients)
 *
 * Inputs:      vector of dequantized coefficients, length = (nBfly+1) * 18
 *              number of "butterflies" to perform (one butterfly means one
 *                inter-block smoothing operation)
 *
 * Outputs:     updated coefficient vector x
 *
 * Return:      none
 *
 * Notes:       weighted average of opposite bands (pairwise) from the 8 samples
 *                before and after each block boundary
 *              nBlocks = (nonZeroBound + 7) / 18, since nZB is the first ZERO sample
 *                above which all other samples are also zero
 *              max gain per sample = 1.372
 *                MAX(i) (abs(csa[i][0]) + abs(csa[i][1]))
 *              bits gained = 0
 *              assume at least 1 guard bit in x[] to avoid overflow
 *                (should be guaranteed from dequant, and max gain from stproc * max
 *                 gain from AntiAlias < 2.0)
 **********************************************************************************************************************/
// a little bit faster in RAM (< 1 ms per block)
/* __attribute__ ((section (".data"))) */
const CSA: [[u32; 2]; 8] = [
    [0x6dc253f0, 0xbe2500aa],
    [0x70dcebe4, 0xc39e4949],
    [0x798d6e73, 0xd7e33f4a],
    [0x7ddd40a7, 0xe8b71176],
    [0x7f6d20b7, 0xf3e4fe2f],
    [0x7fe47e40, 0xfac1a3c7],
    [0x7ffcb263, 0xfe2ebdc6],
    [0x7fffc694, 0xff86c25d],
];

pub fn anti_alias(x: &mut [i32], n_bfly: usize) {
    if n_bfly <= 0 {
        return;
    }

    // Tworzymy slice (bezpieczny widok na pamięć C++)
    let total_len = (n_bfly * 18) + 8;
    let samples = &mut x[..total_len];

    for k in 1..=n_bfly {
        let center = k * 18;

        // Iterujemy bezpośrednio po parach współczynników w CSA
        // enumerate() daje nam indeks 'i', którego używamy do sięgania w głąb bloku audio
        for (i, &[c0, c1]) in CSA.iter().enumerate() {
            // Wyliczamy indeksy próbek wokół granicy (center)
            let idx_a = center - (i + 1);
            let idx_b = center + i;

            // Pobieramy próbki - używamy get_unchecked dla maksymalnej wydajności
            // (wiemy, że indeksy są poprawne dzięki total_len) lub zwykłego indeksowania
            let a0 = samples[idx_a];
            let b0 = samples[idx_b];

            // Obliczenia (Q31 butterfly)
            let tmp1 = mulshift_32(a0, c0 as i32) - mulshift_32(b0, c1 as i32);
            let tmp2 = mulshift_32(b0, c0 as i32) + mulshift_32(a0, c1 as i32);

            // Zapis z powrotem
            samples[idx_a] = tmp1 << 1;
            samples[idx_b] = tmp2 << 1;
        }
    }
}

#[unsafe(no_mangle)]
pub fn hybrid_transform(
    x_curr: &mut [i32; MAX_NSAMP],
    x_prev: &mut [i32; MAX_NSAMP / 2],
    y: &mut [[i32; NBANDS]; BLOCK_SIZE], // Tablica y[18][32] przekazana jako wskaźnik
    sis: &SideInfoSub,
    bc: &mut BlockCount,
) -> i32 {
    let mut x_prev_win = [0i32; 18];
    let mut m_out = 0i32;
    let mut n_blocks_out;
    let (x_curr, _) = x_curr.as_chunks_mut::<18>();
    let (x_prev, _) = x_prev.as_chunks_mut::<9>();

    let mut i = 0;

    // 1. Bloky długie (Long Blocks)
    if bc.nBlocksLong > NBANDS as i32
        || bc.nBlocksTotal > NBANDS as i32
        || bc.nBlocksPrev > NBANDS as i32
    {
        return -1;
    }
    while i < bc.nBlocksLong {
        let mut curr_win_idx = sis.blockType;
        if sis.mixedBlock != 0 && i < bc.currWinSwitch {
            curr_win_idx = BlockType::Normal;
        }

        let mut prev_win_idx = bc.prev_type;
        if i < bc.prevWinSwitch {
            prev_win_idx = BlockType::Normal;
        }

        // Adresowanie y[0][i] w tablicy y[18][32] to po prostu y + i
        // ponieważ y[row][col] = y[row * 32 + col]
        m_out |= imdct36(
            &mut x_curr[i as usize],
            &mut x_prev[i as usize],
            &mut y.as_flattened_mut()[i as usize..],
            curr_win_idx,
            prev_win_idx,
            i,
            bc.gbIn,
        );

        i += 1;
    }

    // 2. Bloki krótkie (Short Blocks)
    while i < bc.nBlocksTotal {
        let mut prev_win_idx = bc.prev_type;
        if i < bc.prevWinSwitch {
            prev_win_idx = BlockType::Normal;
        }

        m_out |= imdct12x3(
            &mut x_curr[i as usize],
            &mut x_prev[i as usize],
            &mut y.as_flattened_mut()[i as usize..],
            prev_win_idx,
            i,
            bc.gbIn,
        );

        i += 1;
    }
    n_blocks_out = i;

    // 3. Okienkowanie i Overlap dla pozostałych bloków poprzednich
    while i < bc.nBlocksPrev {
        let mut prev_win_idx = bc.prev_type;
        if i < bc.prevWinSwitch {
            prev_win_idx = BlockType::Normal;
        }

        win_previous(&mut x_prev[i as usize], &mut x_prev_win, prev_win_idx);

        let mut non_zero = 0i32;
        let fi_bit = (i as i32) << 31;

        for j in 0..9 {
            // Próbki parzyste (2*j)
            let mut xp = x_prev_win[2 * j] << 2;
            non_zero |= xp;
            y[2 * j][i as usize] = xp;
            m_out |= xp.abs();

            // Próbki nieparzyste (2*j + 1) + Inwersja Częstotliwości
            // Logika: (xp ^ -1) + 1 to zmiana znaku (2's complement)
            xp = x_prev_win[2 * j + 1] << 2;
            let mask = fi_bit >> 31; // Arytmetyczne przesunięcie: i odd -> 0xFFFFFFFF, i even -> 0
            xp = (xp ^ mask).wrapping_add(i & 0x01);

            non_zero |= xp;
            y[2 * j + 1][i as usize] = xp;
            m_out |= xp.abs();

            x_prev[i as usize][j] = 0;
        }

        if non_zero != 0 {
            n_blocks_out = i;
        }
        i += 1;
    }

    // 4. Czyszczenie pozostałych bloków (do 32 pasm)
    while i < NBANDS as i32 {
        for j in 0..BLOCK_SIZE {
            y[j][i as usize] = 0;
        }
        i += 1;
    }

    // Obliczanie Guard Bits dla wyjścia (CLZ - Count Leading Zeros)
    // m_out.leading_zeros() zwraca u32, musimy rzutować
    bc.gbOut = (m_out.leading_zeros() as i32) - 1;

    n_blocks_out
}

#[unsafe(no_mangle)]
pub fn imdct12x3(
    x_curr: &mut [i32; 18],
    x_prev: &mut [i32; 9],
    y: &mut [i32],
    bt_prev: BlockType,
    block_idx: i32,
    gb: i32,
) -> i32 {
    let mut x_buf: [i32; BLOCK_SIZE] = [0i32; BLOCK_SIZE];
    let mut x_prev_win = [0i32; BLOCK_SIZE];
    let mut es = 0;

    // 1. Skalowanie (Guard Bits)
    // Jeśli mamy za mało bitów strażniczych, przesuwamy dane w prawo
    if gb < 7 {
        es = 7 - gb;
        for i in 0..9 {
            // x_curr jest interleafed (3 bloki po 6 próbek)
            x_curr[i * 2] >>= es;
            x_curr[i * 2 + 1] >>= es;
            x_prev[i] >>= es;
        }
    }

    // 2. Trzy transformaty IMDCT 12-punktowe
    // Dane wejściowe są przeplatane: b0[0], b1[0], b2[0], b0[1]...
    let (c1, _) = x_buf.as_chunks_mut::<6>();
    imdct_12(&x_curr[..16].try_into().unwrap(), &mut c1[0]); // Block 0
    imdct_12(&x_curr[1..17].try_into().unwrap(), &mut c1[1]); // Block 1
    imdct_12(&x_curr[2..18].try_into().unwrap(), &mut c1[2]); // Block 2

    // 3. Okienkowanie poprzedniego bloku (Overlap z poprzedniej ramki)
    win_previous(x_prev, &mut x_prev_win, bt_prev);
    // Pobranie wskaźnika do okna krótkiego (index 2)
    let wp = IMDCT_WIN[2];
    let mut m_out = 0i32;

    // 4. Nakładanie i dodawanie (Overlap-Add) dla krótkich bloków
    for i in 0..3 {
        let mut y_lo: i32;

        // Pierwsze 6 próbek pochodzi tylko z poprzedniego okna (xPrevWin)
        y_lo = x_prev_win[i] << 2;
        m_out |= y_lo.abs();
        y[i * NBANDS] = y_lo;

        y_lo = x_prev_win[3 + i] << 2;
        m_out |= y_lo.abs();
        y[(3 + i) * NBANDS] = y_lo;

        // Kolejne próbki to suma poprzedniego okna i nowych danych (xBuf) z oknem wp
        y_lo = (x_prev_win[6 + i] << 2) + mulshift_32(wp[i] as i32, x_buf[3 + i]);
        m_out |= y_lo.abs();
        y[(6 + i) * NBANDS] = y_lo;

        y_lo = (x_prev_win[9 + i] << 2) + mulshift_32(wp[3 + i] as i32, x_buf[5 - i]);
        m_out |= y_lo.abs();
        y[(9 + i) * NBANDS] = y_lo;

        // Składanie na stykach bloków wewnętrznych (short block concatenation)
        y_lo = (x_prev_win[12 + i] << 2)
            + mulshift_32(wp[6 + i] as i32, x_buf[2 - i])
            + mulshift_32(wp[i] as i32, x_buf[9 + i]);
        m_out |= y_lo.abs();
        y[(12 + i) * NBANDS] = y_lo;

        y_lo = (x_prev_win[15 + i] << 2)
            + mulshift_32(wp[9 + i] as i32, x_buf[i])
            + mulshift_32(wp[3 + i] as i32, x_buf[11 - i]);
        m_out |= y_lo.abs();
        y[(15 + i) * NBANDS] = y_lo;
    }

    // 5. Zapisanie części do overlapu na następną ramkę (tylko 9 próbek)
    // Wykorzystujemy symetrię IMDCT
    for i in 0..3 {
        x_prev[i] = x_buf[6 + i] >> 2;
    }
    for i in 0..6 {
        x_prev[3 + i] = x_buf[12 + i] >> 2;
    }

    // 6. Korekta końcowa: inwersja i skalowanie
    m_out |= freq_invert_rescale(y, x_prev, block_idx, es);

    m_out
}

#[unsafe(no_mangle)]
pub fn imdct(
    gr: GranuleIndex,
    ch: ChannelIndex,
    sfb: &SFBandTable,
    m_mpegversion: MPEGVersion,
    m_side_info_sub: &[[SideInfoSub; 2]; 2],
    m_huffman_info: &mut HuffmanInfo,
    m_imdctinfo: &mut IMDCTInfo,
) -> i32 {
    let mut bc = BlockCount {
        nBlocksLong: 0,
        nBlocksTotal: 0,
        nBlocksPrev: 0,
        prev_type: BlockType::Normal,
        prevWinSwitch: 0,
        currWinSwitch: 0,
        gbIn: 0,
        gbOut: 0,
    };

    let n_bfly: i32;
    let sis = &m_side_info_sub[gr as usize][ch as usize];

    // blockCutoff logic
    // MPEG1 = 0, inne = MPEG2/2.5
    let cutoff_idx = if m_mpegversion == MPEGVersion::MPEG1 {
        8
    } else {
        6
    };
    let block_cutoff = (sfb.l[cutoff_idx] as i32) / 18;

    if sis.blockType != BlockType::Short {
        /* all long transforms */
        let x = (m_huffman_info.non_zero_bound[ch as usize] + 7) / 18 + 1;
        bc.nBlocksLong = if x < 32 { x } else { 32 };
        n_bfly = bc.nBlocksLong - 1;
    } else if sis.blockType == BlockType::Short && sis.mixedBlock != 0 {
        /* mixed block */
        bc.nBlocksLong = block_cutoff;
        n_bfly = bc.nBlocksLong - 1;
    } else {
        /* all short transforms */
        bc.nBlocksLong = 0;
        n_bfly = 0;
    }

    // Wywołanie AntiAlias na buforze konkretnego kanału
    // huffDecBuf[ch] to tablica 576 intów
    if let Ok(size) = n_bfly.try_into() {
        anti_alias(&mut m_huffman_info.huff_dec_buf[ch as usize], size);
    }

    // Aktualizacja nonZeroBound
    let x_nz = m_huffman_info.non_zero_bound[ch as usize];
    let y_nz = n_bfly * 18 + 8;
    m_huffman_info.non_zero_bound[ch as usize] = if x_nz > y_nz { x_nz } else { y_nz };

    // bc setup
    bc.nBlocksTotal = (m_huffman_info.non_zero_bound[ch as usize] + 17) / 18;
    bc.nBlocksPrev = m_imdctinfo.numPrevIMDCT[ch as usize];
    bc.prev_type = m_imdctinfo.prevType[ch as usize];
    bc.prevWinSwitch = m_imdctinfo.prevWinSwitch[ch as usize];
    bc.currWinSwitch = if sis.mixedBlock != 0 { block_cutoff } else { 0 };
    // Założenie: HuffmanInfo ma pole gb (guard bits)
    // bc.gbIn = hi.gb[ch as usize];

    // Wywołanie HybridTransform
    m_imdctinfo.numPrevIMDCT[ch as usize] = hybrid_transform(
        &mut m_huffman_info.huff_dec_buf[ch as usize],
        &mut m_imdctinfo.overBuf[ch as usize],
        &mut m_imdctinfo.outBuf[ch as usize],
        sis,
        &mut bc,
    );

    m_imdctinfo.prevType[ch as usize] = sis.blockType;
    m_imdctinfo.prevWinSwitch[ch as usize] = bc.currWinSwitch;
    // im.gb[ch as usize] = bc.gbOut;

    0
}

/* pow(2,-i/4) * pow(j,4/3) for i=0..3 j=0..15, Q25 format */
const POW43_14: [[i32; 16]; 4] = [
    /* Q28 */
    [
        0x00000000, 0x10000000, 0x285145f3, 0x453a5cdb, 0x0cb2ff53, 0x111989d6, 0x15ce31c8,
        0x1ac7f203, 0x20000000, 0x257106b9, 0x2b16b4a3, 0x30ed74b4, 0x36f23fa5, 0x3d227bd3,
        0x437be656, 0x49fc823c,
    ],
    [
        0x00000000, 0x0d744fcd, 0x21e71f26, 0x3a36abd9, 0x0aadc084, 0x0e610e6e, 0x12560c1d,
        0x168523cf, 0x1ae89f99, 0x1f7c03a4, 0x243bae49, 0x29249c67, 0x2e34420f, 0x33686f85,
        0x38bf3dff, 0x3e370182,
    ],
    [
        0x00000000, 0x0b504f33, 0x1c823e07, 0x30f39a55, 0x08facd62, 0x0c176319, 0x0f6b3522,
        0x12efe2ad, 0x16a09e66, 0x1a79a317, 0x1e77e301, 0x2298d5b4, 0x26da56fc, 0x2b3a902a,
        0x2fb7e7e7, 0x3450f650,
    ],
    [
        0x00000000, 0x09837f05, 0x17f910d7, 0x2929c7a9, 0x078d0dfa, 0x0a2ae661, 0x0cf73154,
        0x0fec91cb, 0x1306fe0a, 0x16434a6c, 0x199ee595, 0x1d17ae3d, 0x20abd76a, 0x2459d551,
        0x28204fbb, 0x2bfe1808,
    ],
];

/* pow(2,-i/4) for i=0..3, Q31 format */
const POW14: [i32; 4] = [0x7fffffff, 0x6ba27e65, 0x5a82799a, 0x4c1bf829];

/* pow(j,4/3) for j=16..63, Q23 format */
const POW43: [i32; 48] = [
    0x1428a2fa, 0x15db1bd6, 0x1796302c, 0x19598d85, 0x1b24e8bb, 0x1cf7fcfa, 0x1ed28af2, 0x20b4582a,
    0x229d2e6e, 0x248cdb55, 0x26832fda, 0x28800000, 0x2a832287, 0x2c8c70a8, 0x2e9bc5d8, 0x30b0ff99,
    0x32cbfd4a, 0x34eca001, 0x3712ca62, 0x393e6088, 0x3b6f47e0, 0x3da56717, 0x3fe0a5fc, 0x4220ed72,
    0x44662758, 0x46b03e7c, 0x48ff1e87, 0x4b52b3f3, 0x4daaebfd, 0x5007b497, 0x5268fc62, 0x54ceb29c,
    0x5738c721, 0x59a72a59, 0x5c19cd35, 0x5e90a129, 0x610b9821, 0x638aa47f, 0x660db90f, 0x6894c90b,
    0x6b1fc80c, 0x6daeaa0d, 0x70416360, 0x72d7e8b0, 0x75722ef9, 0x78102b85, 0x7ab1d3ec, 0x7d571e09,
];

/*
 * Minimax polynomial approximation to pow(x, 4/3), over the range
 *  poly43lo: x = [0.5, 0.7071]
 *  poly43hi: x = [0.7071, 1.0]
 *
 * Relative error < 1E-7
 * Coefs are scaled by 4, 2, 1, 0.5, 0.25
 */
const POLY43LO: [u32; 5] = [0x29a0bda9, 0xb02e4828, 0x5957aa1b, 0x236c498d, 0xff581859];
const POLY43HI: [u32; 5] = [0x10852163, 0xd333f6a4, 0x46e9408b, 0x27c2cef0, 0xfef577b4];

/* pow(2, i*4/3) as exp and frac */
const POW2EXP: [i32; 8] = [14, 13, 11, 10, 9, 7, 6, 5];

const POW2FRAC: [i32; 8] = [
    0x6597fa94, 0x50a28be6, 0x7fffffff, 0x6597fa94, 0x50a28be6, 0x7fffffff, 0x6597fa94, 0x50a28be6,
];

pub fn dequant_block(in_buf: &[i32], out_buf: &mut [i32], num: i32, scale: i32) -> i32 {
    if num <= 0 {
        return 0;
    }

    let mut tab4 = [0i32; 4];
    let mut mask = 0i32;

    // Pobranie tablicy dla skali ułamkowej
    let tab16 = &POW43_14[(scale & 0x3) as usize];
    let scalef = POW14[(scale & 0x3) as usize];

    // scalei = min(scale >> 2, 31)
    let mut scalei = scale >> 2;
    if scalei > 31 {
        scalei = 31;
    }

    /* Cache first 4 values */
    let mut shift_init = scalei + 3;
    if shift_init > 31 {
        shift_init = 31;
    }
    if shift_init < 0 {
        shift_init = 0;
    }

    tab4[0] = 0;
    tab4[1] = tab16[1] >> shift_init;
    tab4[2] = tab16[2] >> shift_init;
    tab4[3] = tab16[3] >> shift_init;

    // Przetwarzamy num próbek
    for i in 0..num {
        let sx = in_buf[i as usize];

        let x = (sx & 0x7fffffff) as u32; // x = magnitude
        let mut y: i32;
        let mut shift: i32;

        if x < 4 {
            y = tab4[x as usize];
        } else if x < 16 {
            y = tab16[x as usize];
            if scalei < 0 {
                y <<= -scalei;
            } else {
                y >>= scalei;
            }
        } else {
            if x < 64 {
                y = POW43[(x - 16) as usize];
                y = mulshift_32(y, scalef);
                shift = scalei - 3;
            } else {
                /* Normalizacja do [0x40000000, 0x7fffffff] */
                let mut x_norm = x << 17;
                shift = 0;
                if x_norm < 0x08000000 {
                    x_norm <<= 4;
                    shift += 4;
                }
                if x_norm < 0x20000000 {
                    x_norm <<= 2;
                    shift += 2;
                }
                if x_norm < 0x40000000 {
                    x_norm <<= 1;
                    shift += 1;
                }

                let coef = if x_norm < SQRTHALF {
                    &POLY43LO
                } else {
                    &POLY43HI
                };

                /* Aproksymacja wielomianowa */
                y = coef[0] as i32;
                y = mulshift_32(y, x_norm as i32) + (coef[1] as i32);
                y = mulshift_32(y, x_norm as i32) + (coef[2] as i32);
                y = mulshift_32(y, x_norm as i32) + (coef[3] as i32);
                y = mulshift_32(y, x_norm as i32) + (coef[4] as i32);

                // y = (y * pow2frac[shift]) << 3
                y = mulshift_32(y, POW2FRAC[shift as usize]) << 3;

                /* Skala ułamkowa */
                y = mulshift_32(y, scalef);
                shift = scalei - POW2EXP[shift as usize];
            }

            /* Skala całkowita z clippingiem */
            if shift < 0 {
                shift = -shift;
                if y > (0x7fffffff >> shift) {
                    y = 0x7fffffff;
                } else {
                    y <<= shift;
                }
            } else {
                y >>= shift;
            }
        }

        /* Przywrócenie znaku i zapis */
        mask |= y;
        let final_y = if sx < 0 { -y } else { y };
        out_buf[i as usize] = final_y;
    }

    mask
}

pub fn dequant_block_in_place(buf: &mut [i32], num: i32, scale: i32) -> i32 {
    if num <= 0 {
        return 0;
    }

    let mut tab4 = [0i32; 4];
    let mut mask = 0i32;

    // Pobranie tablicy dla skali ułamkowej
    let tab16 = &POW43_14[(scale & 0x3) as usize];
    let scalef = POW14[(scale & 0x3) as usize];

    // scalei = min(scale >> 2, 31)
    let mut scalei = scale >> 2;
    if scalei > 31 {
        scalei = 31;
    }

    /* Cache first 4 values */
    let mut shift_init = scalei + 3;
    if shift_init > 31 {
        shift_init = 31;
    }
    if shift_init < 0 {
        shift_init = 0;
    }

    tab4[0] = 0;
    tab4[1] = tab16[1] >> shift_init;
    tab4[2] = tab16[2] >> shift_init;
    tab4[3] = tab16[3] >> shift_init;

    // Przetwarzamy num próbek
    for i in 0..num {
        let sx = buf[i as usize];

        let x = (sx & 0x7fffffff) as u32; // x = magnitude
        let mut y: i32;
        let mut shift: i32;

        if x < 4 {
            y = tab4[x as usize];
        } else if x < 16 {
            y = tab16[x as usize];
            if scalei < 0 {
                y <<= -scalei;
            } else {
                y >>= scalei;
            }
        } else {
            if x < 64 {
                y = POW43[(x - 16) as usize];
                y = mulshift_32(y, scalef);
                shift = scalei - 3;
            } else {
                /* Normalizacja do [0x40000000, 0x7fffffff] */
                let mut x_norm = x << 17;
                shift = 0;
                if x_norm < 0x08000000 {
                    x_norm <<= 4;
                    shift += 4;
                }
                if x_norm < 0x20000000 {
                    x_norm <<= 2;
                    shift += 2;
                }
                if x_norm < 0x40000000 {
                    x_norm <<= 1;
                    shift += 1;
                }

                let coef = if x_norm < SQRTHALF {
                    &POLY43LO
                } else {
                    &POLY43HI
                };

                /* Aproksymacja wielomianowa */
                y = coef[0] as i32;
                y = mulshift_32(y, x_norm as i32) + (coef[1] as i32);
                y = mulshift_32(y, x_norm as i32) + (coef[2] as i32);
                y = mulshift_32(y, x_norm as i32) + (coef[3] as i32);
                y = mulshift_32(y, x_norm as i32) + (coef[4] as i32);

                // y = (y * pow2frac[shift]) << 3
                y = mulshift_32(y, POW2FRAC[shift as usize]) << 3;

                /* Skala ułamkowa */
                y = mulshift_32(y, scalef);
                shift = scalei - POW2EXP[shift as usize];
            }

            /* Skala całkowita z clippingiem */
            if shift < 0 {
                shift = -shift;
                if y > (0x7fffffff >> shift) {
                    y = 0x7fffffff;
                } else {
                    y <<= shift;
                }
            } else {
                y >>= shift;
            }
        }

        /* Przywrócenie znaku i zapis */
        mask |= y;
        let final_y = if sx < 0 { -y } else { y };
        buf[i as usize] = final_y;
    }

    mask
}

/* optional pre-emphasis for high-frequency scale factor bands */
const PRE_TAB: [u8; 22] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 3, 2, 0,
];

#[unsafe(no_mangle)]
pub fn dequant_channel(
    sample_buf: &mut [i32; MAX_NSAMP],
    work_buf: &mut [i32],
    non_zero_bound: &mut i32,
    sis: &SideInfoSub,
    sfis: &ScaleFactorInfoSub,
    cbi: &mut CriticalBandInfo,
    m_frame_header: &FrameHeader,
    m_sf_band_table: &SFBandTable,
    m_mpegversion: MPEGVersion,
) -> i32 {
    let cb_end_l: i32;
    let cb_start_s: i32;
    let cb_end_s: i32;

    // 1. Ustalenie granic dla bloków długich i krótkich
    if sis.blockType == BlockType::Short {
        if sis.mixedBlock != 0 {
            cb_end_l = if m_mpegversion == MPEGVersion::MPEG1 {
                8
            } else {
                6
            }; // MPEG1 vs MPEG2
            cb_start_s = 3;
        } else {
            cb_end_l = 0;
            cb_start_s = 0;
        }
        cb_end_s = 13;
    } else {
        cb_end_l = 22;
        cb_start_s = 13;
        cb_end_s = 13;
    }

    let mut cb_max = [0i32; 3];
    let mut gb_mask = 0i32;
    let mut i: usize = 0;

    // sfactMultiplier = 2 lub 4
    let s_multiplier = 2 * (sis.sfactScale + 1);

    // Obliczenie globalGain z uwzględnieniem MidSide i skali IMDCT
    let mut global_gain = sis.global_gain;
    if (m_frame_header.modeExt >> 1) != 0 {
        global_gain -= IMDCT_SCALE as i32;
    }
    global_gain += IMDCT_SCALE as i32;

    // 2. Dekwantyzacja bloków długich
    for cb in 0..cb_end_l {
        let n_samps =
            (m_sf_band_table.l[(cb + 1) as usize] - m_sf_band_table.l[cb as usize]) as i32;

        let pre_val = if sis.preFlag != 0 {
            PRE_TAB[cb as usize] as i32
        } else {
            0
        };
        let gain_i = 210 - global_gain + s_multiplier * (sfis.l[cb as usize] as i32 + pre_val);

        let non_zero = dequant_block_in_place(&mut sample_buf[i..], n_samps, gain_i);

        if non_zero != 0 {
            cb_max[0] = cb;
        }
        gb_mask |= non_zero;
        i += n_samps as usize;

        if i >= (*non_zero_bound) as usize {
            break;
        }
    }

    // Wstępne ustawienie CBI
    cbi.cbType = 0;
    cbi.cbEndL = cb_max[0];
    cbi.cbEndS = [0, 0, 0];
    cbi.cbEndSMax = 0;

    if cb_start_s >= 12 {
        return (gb_mask.leading_zeros() as i32) - 1;
    }

    // 3. Dekwantyzacja bloków krótkich
    cb_max = [cb_start_s, cb_start_s, cb_start_s];
    for cb in cb_start_s..cb_end_s {
        let n_samps =
            (m_sf_band_table.s[(cb + 1) as usize] - m_sf_band_table.s[cb as usize]) as i32;

        for w in 0..3 {
            let gain_i = 210 - global_gain
                + 8 * sis.subBlockGain[w]
                + s_multiplier * (sfis.s[cb as usize][w] as i32);

            // Dekwantyzujemy do workBuf, aby móc potem bezpiecznie przełożyć dane do sampleBuf
            let non_zero = dequant_block(
                &sample_buf[i + (n_samps * w as i32) as usize..],
                &mut work_buf[(n_samps * w as i32) as usize..],
                n_samps,
                gain_i,
            );

            if non_zero != 0 {
                cb_max[w] = cb;
            }
            gb_mask |= non_zero;
        }

        // 4. Reordering: Przeplatanie próbek z 3 bloków krótkich
        // C: buf[j][0] = workBuf[0*nSamps + j]
        let n = n_samps as usize;
        let start = i;
        let end = start + n * 3;

        // Bezpośrednie wycięcie fragmentu (rzuci panic jeśli poza zakresem)
        let target_slice = &mut sample_buf[start..end];
        let (rows, _) = target_slice.as_chunks_mut::<3>();

        for (j, row) in rows.iter_mut().enumerate() {
            // row jest &mut [i32; 3], kompilator usuwa sprawdzenie granic dla row[0,1,2]
            row[0] = work_buf[j];
            row[1] = work_buf[n + j];
            row[2] = work_buf[2 * n + j];
        }

        i += (3 * n_samps) as usize;
        if i >= (*non_zero_bound) as usize {
            break;
        }
    }

    // Aktualizacja non_zero_bound i CBI
    *non_zero_bound = i as i32;
    cbi.cbType = if sis.mixedBlock != 0 { 2 } else { 1 };
    cbi.cbEndS = cb_max;
    cbi.cbEndSMax = cb_max[0].max(cb_max[1]).max(cb_max[2]);

    (gb_mask.leading_zeros() as i32) - 1
}

const ISFMPEG1: [[i32; 7]; 2] = [
    [
        0x00000000, 0x0d8658ba, 0x176cf5d0, 0x20000000, 0x28930a2f, 0x3279a745, 0x40000000,
    ],
    [
        0x00000000, 0x13207f5c, 0x2120fb83, 0x2d413ccc, 0x39617e16, 0x4761fa3d, 0x5a827999,
    ],
];

const ISFMPEG2: [[[i32; 16]; 2]; 2] = [
    [
        [
            /* intensityScale off, mid-side off */
            0x40000000, 0x35d13f32, 0x2d413ccc, 0x260dfc14, 0x1fffffff, 0x1ae89f99, 0x16a09e66,
            0x1306fe0a, 0x0fffffff, 0x0d744fcc, 0x0b504f33, 0x09837f05, 0x07ffffff, 0x06ba27e6,
            0x05a82799, 0x04c1bf82,
        ],
        [
            /* intensityScale off, mid-side on */
            0x5a827999, 0x4c1bf827, 0x3fffffff, 0x35d13f32, 0x2d413ccc, 0x260dfc13, 0x1fffffff,
            0x1ae89f99, 0x16a09e66, 0x1306fe09, 0x0fffffff, 0x0d744fcc, 0x0b504f33, 0x09837f04,
            0x07ffffff, 0x06ba27e6,
        ],
    ],
    [
        [
            /* intensityScale on, mid-side off */
            0x40000000, 0x2d413ccc, 0x20000000, 0x16a09e66, 0x10000000, 0x0b504f33, 0x08000000,
            0x05a82799, 0x04000000, 0x02d413cc, 0x02000000, 0x016a09e6, 0x01000000, 0x00b504f3,
            0x00800000, 0x005a8279,
        ],
        [
            /* intensityScale on, mid-side on */
            0x5a827999, 0x3fffffff, 0x2d413ccc, 0x1fffffff, 0x16a09e66, 0x0fffffff, 0x0b504f33,
            0x07ffffff, 0x05a82799, 0x03ffffff, 0x02d413cc, 0x01ffffff, 0x016a09e6, 0x00ffffff,
            0x00b504f3, 0x007fffff,
        ],
    ],
];
/* indexing = [intensity scale on/off][left/right]
 * format = Q30, range = [0.0, 1.414]
 *
 * illegal intensity position scalefactors (see comments on ISFMpeg1)
 */

const ISFIIP: [[i32; 2]; 2] = [
    [0x40000000, 0x00000000], /* mid-side off */
    [0x40000000, 0x40000000], /* mid-side on */
];
#[unsafe(no_mangle)]
pub fn intensity_proc_mpeg1(
    x: &mut [[i32; MAX_NSAMP]; MAX_NCHAN], // x[2][576]
    n_samps: i32,
    sfis: &ScaleFactorInfoSub,
    cbi: &[CriticalBandInfo; 2],
    mid_side_flag: usize,
    m_out: &mut [i32; 2], // mOut[2]
    sfbt: &SFBandTable,
) {
    let mut i: usize;
    let (cb_start_l, cb_end_l, cb_start_s, cb_end_s);

    if cbi[1].cbType == 0 {
        cb_start_l = (cbi[1].cbEndL + 1) as usize;
        cb_end_l = (cbi[0].cbEndL + 1) as usize;
        cb_start_s = 0;
        cb_end_s = 0;
        i = sfbt.l[cb_start_l] as usize;
    } else {
        cb_start_s = (cbi[1].cbEndSMax + 1) as usize;
        cb_end_s = (cbi[0].cbEndSMax + 1) as usize;
        cb_start_l = 0;
        cb_end_l = 0;
        i = (3 * sfbt.s[cb_start_s]) as usize;
    }

    let mut samps_left = n_samps - i as i32;
    let isf_tab = ISFMPEG1[mid_side_flag];
    let mut m_out_l = 0;
    let mut m_out_r = 0;

    // Bloki długie
    for cb in cb_start_l..cb_end_l {
        if samps_left <= 0 {
            break;
        }
        let isf = sfis.l[cb] as usize;
        let (fl, fr) = if isf == 7 {
            (ISFIIP[mid_side_flag][0], ISFIIP[mid_side_flag][1])
        } else {
            (isf_tab[isf], isf_tab[6] - isf_tab[isf])
        };

        let n = (sfbt.l[cb + 1] - sfbt.l[cb]) as usize;
        for _ in 0..n {
            if samps_left <= 0 {
                break;
            }
            let common = x[0][i];

            let xr = mulshift_32(fr, common) << 2;
            x[1][i] = xr;
            m_out_r |= xr.abs();

            let xl = mulshift_32(fl, common) << 2;
            x[0][i] = xl;
            m_out_l |= xl.abs();

            i += 1;
            samps_left -= 1;
        }
    }

    // Bloki krótkie (uproszczona logika i += 3)
    for cb in cb_start_s..cb_end_s {
        if samps_left < 3 {
            break;
        }
        let mut fls = [0; 3];
        let mut frs = [0; 3];
        for w in 0..3 {
            let isf = sfis.s[cb][w] as usize;
            if isf == 7 {
                fls[w] = ISFIIP[mid_side_flag][0];
                frs[w] = ISFIIP[mid_side_flag][1];
            } else {
                fls[w] = isf_tab[isf];
                frs[w] = isf_tab[6] - isf_tab[isf];
            }
        }

        let n = (sfbt.s[cb + 1] - sfbt.s[cb]) as usize;
        for _ in 0..n {
            if samps_left < 3 {
                break;
            }
            for w in 0..3 {
                let common = x[0][i + w];
                let xr = mulshift_32(frs[w], common) << 2;
                x[1][i + w] = xr;
                m_out_r |= xr.abs();
                let xl = mulshift_32(fls[w], common) << 2;
                x[0][i + w] = xl;
                m_out_l |= xl.abs();
            }
            i += 3;
            samps_left -= 3;
        }
    }

    m_out[0] = m_out_l;
    m_out[1] = m_out_r;
}

#[unsafe(no_mangle)]
pub fn intensity_proc_mpeg2(
    x: &mut [[i32; MAX_NSAMP]; MAX_NCHAN], // x[2][576]
    n_samps: i32,
    sfis: &ScaleFactorInfoSub,
    cbi: &[CriticalBandInfo; 2],
    sfjs: &ScaleFactorJS,
    mid_side_flag: usize,
    _mix_flag: i32,
    m_out: &mut [i32; 2], // mOut[2]
    sfbt: &SFBandTable,
) {
    let mut m_out_l = 0i32;
    let mut m_out_r = 0i32;
    let mut il = [0i32; 23];

    // Pobranie odpowiedniej tabeli współczynników dla MPEG2
    // ISFMpeg2[intensityScale][midSideFlag]
    let isf_tab = ISFMPEG2[sfjs.intensity_scale as usize][mid_side_flag];
    // let isf_tab = core::slice::from_raw_parts(isf_tab_ptr, 16);

    // 1. Wypełnienie bufora il (illegal intensity positions)
    let mut k = 0;
    for r in 0..4 {
        let tmp = (1 << sfjs.slen[r as usize]) - 1;
        for _ in 0..sfjs.nr[r as usize] {
            if k < 23 {
                il[k] = tmp;
                k += 1;
            }
        }
    }

    if cbi[1].cbType == 0 {
        /* BLOKI DŁUGIE */
        il[21] = 1;
        il[22] = 1;
        let cb_start_l = (cbi[1].cbEndL + 1) as usize;
        let cb_end_l = (cbi[0].cbEndL + 1) as usize;
        let mut i = sfbt.l[cb_start_l] as usize;
        let mut samps_left = n_samps - i as i32;

        for cb in cb_start_l..cb_end_l {
            if samps_left <= 0 {
                break;
            }

            let sf_idx = sfis.l[cb] as i32;
            let (fl, fr);

            if sf_idx == il[cb] {
                fl = ISFIIP[mid_side_flag][0];
                fr = ISFIIP[mid_side_flag][1];
            } else {
                let isf = ((sf_idx + 1) >> 1) as usize;
                if sf_idx & 0x01 != 0 {
                    fl = isf_tab[isf];
                    fr = isf_tab[0];
                } else {
                    fl = isf_tab[0];
                    fr = isf_tab[isf];
                }
            }

            let band_len = (sfbt.l[cb + 1] - sfbt.l[cb]) as i32;
            let n = if band_len < samps_left {
                band_len
            } else {
                samps_left
            };

            for _ in 0..n {
                let common = x[0][i];
                let xr = mulshift_32(fr, common) << 2;
                let xl = mulshift_32(fl, common) << 2;

                x[1][i] = xr;
                x[0][i] = xl;

                m_out_r |= xr.abs();
                m_out_l |= xl.abs();
                i += 1;
            }
            samps_left -= n;
        }
    } else {
        /* BLOKI KRÓTKIE LUB MIESZANE */
        il[12] = 1;

        for w in 0..3 {
            let cb_start_s = (cbi[1].cbEndS[w] + 1) as usize;
            let cb_end_s = (cbi[0].cbEndS[w] + 1) as usize;
            let mut i = (3 * sfbt.s[cb_start_s] + w as i32) as usize;

            for cb in cb_start_s..cb_end_s {
                let sf_idx = sfis.s[cb][w] as i32;
                let (fl, fr);

                if sf_idx == il[cb] {
                    fl = ISFIIP[mid_side_flag][0];
                    fr = ISFIIP[mid_side_flag][1];
                } else {
                    let isf = ((sf_idx + 1) >> 1) as usize;
                    if sf_idx & 0x01 != 0 {
                        fl = isf_tab[isf];
                        fr = isf_tab[0];
                    } else {
                        fl = isf_tab[0];
                        fr = isf_tab[isf];
                    }
                }

                let n = (sfbt.s[cb + 1] - sfbt.s[cb]) as usize;
                for _ in 0..n {
                    if i < 576 {
                        let common = x[0][i];
                        let xr = mulshift_32(fr, common) << 2;
                        let xl = mulshift_32(fl, common) << 2;

                        x[1][i] = xr;
                        x[0][i] = xl;

                        m_out_r |= xr.abs();
                        m_out_l |= xl.abs();
                        i += 3;
                    }
                }
            }
        }
    }

    m_out[0] = m_out_l;
    m_out[1] = m_out_r;
}

#[unsafe(no_mangle)]
pub fn mid_side_proc(
    x: &mut [[i32; MAX_NSAMP]; MAX_NCHAN], // x[2][576]
    n_samps: usize,
    m_out: &mut [i32; MAX_NCHAN],
) {
    // 1. Ograniczamy n_samps do faktycznego rozmiaru tablicy,
    // co pomaga kompilatorowi wyeliminować checks w pętli.
    let n = n_samps.min(MAX_NSAMP);

    // 2. Rozbijamy x na dwa oddzielne slice'y, aby uniknąć indeksowania x[0][i] i x[1][i].
    // Dzięki temu procesor ma dwa stałe wskaźniki, które idą liniowo.
    let (left_chan, right_chan) = x.split_at_mut(1);
    let l_slice = &mut left_chan[0][..n];
    let r_slice = &mut right_chan[0][..n];

    let mut m_out_l = 0i32;
    let mut m_out_r = 0i32;

    // 3. zip() tworzy iterator par. LLVM zamieni to na pętlę operującą na dwóch wskaźnikach.
    // Użycie iteratora zamiast indeksowania pętlą 'for i in 0..n' jest kluczowe dla Xtensa.
    for (mid_ref, side_ref) in l_slice.iter_mut().zip(r_slice.iter_mut()) {
        let mid = *mid_ref;
        let side = *side_ref;

        let l = mid.wrapping_add(side);
        let r = mid.wrapping_sub(side);

        *mid_ref = l;
        *side_ref = r;

        // 4. Bitowe abs() jest często szybsze na S3 niż instrukcja warunkowa.
        // Chociaż S3 ma instrukcję ABS, LLVM przy |= abs() potrafi zrobić świetne unrollowanie.
        m_out_l |= l.abs();
        m_out_r |= r.abs();
    }

    m_out[0] |= m_out_l;
    m_out[1] |= m_out_r;
}

/***********************************************************************************************************************
 * Function:    UnpackScaleFactors
 *
 * Description: parse the fields of the MP3 scale factor data section
 *
 * Inputs:      MP3DecInfo structure filled by UnpackFrameHeader() and UnpackSideInfo()
 *              buffer pointing to the MP3 scale factor data
 *              pointer to bit offset (0-7) indicating starting bit in buf[0]
 *              number of bits available in data buffer
 *              index of current granule and channel
 *
 * Outputs:     updated platform-specific ScaleFactorInfo struct
 *              updated bitOffset
 *
 * Return:      length (in bytes) of scale factor data, -1 if null input pointers
 **********************************************************************************************************************/

#[unsafe(no_mangle)]
pub unsafe fn unpack_scale_factors(
    mut buf: *mut u8,
    bit_offset: &mut i32,
    bits_avail: i32,
    gr: GranuleIndex,
    ch: ChannelIndex,
    m_side_info_sub: &mut [[SideInfoSub; 2]; 2],
    m_scale_factor_info_sub: &mut [[ScaleFactorInfoSub; 2]; 2],
    m_mp3_dec_info: &mut MP3DecInfo,
    m_side_info: &mut SideInfo,
    m_frame_header: &mut FrameHeader,
    m_scale_factor_js: &mut ScaleFactorJS,
    m_mpegversion: MPEGVersion,
) -> i32 {
    /* init GetBits reader */
    let start_buf = unsafe {
        core::slice::from_raw_parts(buf, (bits_avail as usize + *bit_offset as usize + 7) / 8)
    };
    let mut bsi = BitStreamInfo::from_slice(start_buf);

    if *bit_offset != 0 {
        bsi.get_bits(*bit_offset as u32);
    }

    if m_mpegversion == MPEGVersion::MPEG1 {
        unpack_sfmpeg1(
            &mut bsi,
            &mut m_side_info_sub[gr as usize][ch as usize],
            m_scale_factor_info_sub,
            &m_side_info.scfsi[ch as usize],
            gr,
            ch,
        );
    } else {
        unpack_sfmpeg2(
            &mut bsi,
            &mut m_side_info_sub[gr as usize][ch as usize],
            &mut m_scale_factor_info_sub[gr as usize][ch as usize],
            gr,
            ch,
            m_frame_header.modeExt,
            m_scale_factor_js,
        );
    }

    m_mp3_dec_info.part23Length[gr as usize][ch as usize] =
        m_side_info_sub[gr as usize][ch as usize].part23_length;

    let bits_used = bsi.calc_bits_used(start_buf, *bit_offset as usize);
    buf = unsafe { buf.add((bits_used + *bit_offset) as usize >> 3) };
    *bit_offset = (bits_used + *bit_offset) & 0x07;

    (unsafe { buf.offset_from(start_buf.as_ptr()) }) as i32
}

/***********************************************************************************************************************
 * Function:    Subband
 *
 * Description: do subband transform on all the blocks in one granule, all channels
 *
 * Inputs:      filled MP3DecInfo structure, after calling IMDCT for all channels
 *              vbuf[ch] and vindex[ch] must be preserved between calls
 *
 * Outputs:     decoded PCM data, interleaved LRLRLR... if stereo
 *
 * Return:      0 on success,  -1 if null input pointers
 **********************************************************************************************************************/
#[unsafe(no_mangle)]
pub fn mp3_dequantize(gr: GranuleIndex, m_mp3_decoder: &mut MP3Decoder) -> i32 {
    let di = &mut m_mp3_decoder.m_MP3DecInfo;
    let hi = &mut m_mp3_decoder.m_HuffmanInfo;
    let dqi = &mut m_mp3_decoder.m_DequantInfo;
    let fh = &mut m_mp3_decoder.m_FrameHeader;
    let cbi = &mut m_mp3_decoder.m_CriticalBandInfo;
    let side_info_sub = &mut m_mp3_decoder.m_SideInfoSub;
    let sf_info_sub = &mut m_mp3_decoder.m_ScaleFactorInfoSub;
    let sfbt = &mut m_mp3_decoder.m_SFBandTable;
    let sf_js = &mut m_mp3_decoder.m_ScaleFactorJS;

    let mut m_out = [0i32; 2];
    let gr_idx = gr as usize;

    // 1. Dekwantyzacja każdego kanału
    for ch in 0..di.nChans as usize {
        hi.gb[ch] = dequant_channel(
            &mut hi.huff_dec_buf[ch],
            &mut dqi.work_buf[..],
            &mut hi.non_zero_bound[ch],
            &mut side_info_sub[gr_idx][ch],
            &mut (*sf_info_sub)[gr_idx][ch],
            &mut cbi[ch],
            fh,
            sfbt,
            m_mp3_decoder.m_MPEGVersion,
        );
    }

    // 2. Obsługa rzadkich przypadków braku bitów strażniczych (clipping)
    if fh.modeExt != 0 && (hi.gb[0] < 1 || hi.gb[1] < 1) {
        for ch in 0..2 {
            for i in 0..hi.non_zero_bound[ch] as usize {
                if hi.huff_dec_buf[ch][i] < -0x3fffffff {
                    hi.huff_dec_buf[ch][i] = -0x3fffffff;
                }
                if hi.huff_dec_buf[ch][i] > 0x3fffffff {
                    hi.huff_dec_buf[ch][i] = 0x3fffffff;
                }
            }
        }
    }

    // 3. Proces Mid-Side Stereo
    if (fh.modeExt >> 1) != 0 {
        let n_samps: i32;
        if (fh.modeExt & 0x01) != 0 {
            /* Intensity stereo włączone - Mid-Side tylko do początku regionu zero prawego kanału */
            if cbi[1].cbType == 0 {
                n_samps = sfbt.l[cbi[1].cbEndL as usize + 1];
            } else {
                n_samps = 3 * sfbt.s[cbi[1].cbEndSMax as usize + 1];
            }
        } else {
            /* Intensity stereo wyłączone - Mid-Side na całym widmie */
            n_samps = hi.non_zero_bound[0].max(hi.non_zero_bound[1]);
        }
        mid_side_proc(&mut hi.huff_dec_buf, n_samps as usize, &mut m_out);
    }

    // 4. Proces Intensity Stereo
    if (fh.modeExt & 0x01) != 0 {
        let n_samps = hi.non_zero_bound[0];
        if m_mp3_decoder.m_MPEGVersion == MPEGVersion::MPEG1 {
            // MPEG1
            intensity_proc_mpeg1(
                &mut hi.huff_dec_buf,
                n_samps,
                &mut sf_info_sub[gr_idx][1],
                cbi,
                (*side_info_sub)[gr_idx][1].mixedBlock as usize,
                &mut m_out,
                sfbt,
            );
        } else {
            // MPEG2
            intensity_proc_mpeg2(
                &mut hi.huff_dec_buf,
                n_samps,
                &mut (*sf_info_sub)[gr_idx][1],
                cbi,
                sf_js,
                fh.modeExt >> 1,
                side_info_sub[gr_idx][1].mixedBlock,
                &mut m_out,
                sfbt,
            );
        }
    }

    // 5. Aktualizacja Guard Bits i NonZeroBound po procesach stereo
    if fh.modeExt != 0 {
        // CLZ (Count Leading Zeros) - w Rust używamy leading_zeros()
        // Pamiętaj o obsłudze abs() dla m_out
        hi.gb[0] = (m_out[0].abs().leading_zeros() as i32) - 1;
        hi.gb[1] = (m_out[1].abs().leading_zeros() as i32) - 1;

        let max_n_samps = hi.non_zero_bound[0].max(hi.non_zero_bound[1]);
        hi.non_zero_bound[0] = max_n_samps;
        hi.non_zero_bound[1] = max_n_samps;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe fn MP3DecodeHelper(
    mut inbuf: *mut u8,
    inbuf_len: usize,
    bytes_left: &mut i32,
    outbuf: *mut i16,
    use_size: i32,
    // SELF
    m_mp3_decoder: &mut MP3Decoder,
) -> i8 {
    let mut offset: i32;
    let mut bit_offset: i32;
    let mut main_bits: i32;
    let si_bytes: i32;
    let free_frame_bytes: i32;
    let mut prev_bit_offset: i32;
    let mut sf_block_bits: i32;
    let mut huff_block_bits: i32;
    let mut main_ptr: *mut u8;

    // let m_mp3_decoder = unsafe { &mut *m_MP3Decoder };
    let mut buf = unsafe { core::slice::from_raw_parts(inbuf, inbuf_len) };
    /* unpack frame header */
    // esp_println::println!("{:?}", &buf[..5]);
    let fh_bytes = match m_mp3_decoder.unpack_frame_header(buf) {
        Ok(v) => v,
        Err(e) => {
            return e;
        }
    };
    inbuf = unsafe { inbuf.add(fh_bytes) };
    buf = &buf[fh_bytes..];
    /* unpack side info */
    si_bytes = m_mp3_decoder.unpack_side_info(buf) as i32;

    let outbuf = unsafe {
        core::slice::from_raw_parts_mut(
            outbuf,
            (m_mp3_decoder.m_MP3DecInfo.nGrans as i32
                * m_mp3_decoder.m_MP3DecInfo.nGranSamps
                * m_mp3_decoder.m_MP3DecInfo.nChans as i32) as usize,
        )
    };

    if si_bytes < 0 {
        MP3ClearBadFrame(outbuf);
        return ERR_MP3_INVALID_SIDEINFO;
    }
    inbuf = unsafe { inbuf.add(si_bytes as usize) };
    *bytes_left -= fh_bytes as i32 + si_bytes;

    /* if free mode... */
    if m_mp3_decoder.m_MP3DecInfo.bitrate == 0 || m_mp3_decoder.m_MP3DecInfo.freeBitrateFlag != 0 {
        if m_mp3_decoder.m_MP3DecInfo.freeBitrateFlag == 0 {
            m_mp3_decoder.m_MP3DecInfo.freeBitrateFlag = 1;
            m_mp3_decoder.m_MP3DecInfo.freeBitrateSlots = unsafe {
                MP3FindFreeSync(
                    inbuf,
                    inbuf.offset(-(fh_bytes as isize) - (si_bytes as isize)),
                    *bytes_left,
                )
            };
            if m_mp3_decoder.m_MP3DecInfo.freeBitrateSlots < 0 {
                MP3ClearBadFrame(outbuf);
                m_mp3_decoder.m_MP3DecInfo.freeBitrateFlag = 0;
                return ERR_MP3_FREE_BITRATE_SYNC;
            }
            free_frame_bytes =
                m_mp3_decoder.m_MP3DecInfo.freeBitrateSlots + fh_bytes as i32 + si_bytes;
            m_mp3_decoder.m_MP3DecInfo.bitrate =
                (free_frame_bytes * m_mp3_decoder.m_MP3DecInfo.samprate * 8)
                    / (m_mp3_decoder.m_MP3DecInfo.nGrans as i32
                        * m_mp3_decoder.m_MP3DecInfo.nGranSamps);
        }
        m_mp3_decoder.m_MP3DecInfo.nSlots = m_mp3_decoder.m_MP3DecInfo.freeBitrateSlots
            + m_mp3_decoder.m_FrameHeader.check_pad_bit();
    }

    /* Bit Reservoir Management */
    if use_size != 0 {
        m_mp3_decoder.m_MP3DecInfo.nSlots = *bytes_left;
        if m_mp3_decoder.m_MP3DecInfo.mainDataBegin != 0 || m_mp3_decoder.m_MP3DecInfo.nSlots <= 0 {
            MP3ClearBadFrame(outbuf);
            return ERR_MP3_INVALID_FRAMEHEADER;
        }
        m_mp3_decoder.m_MP3DecInfo.mainDataBytes = m_mp3_decoder.m_MP3DecInfo.nSlots;
        main_ptr = inbuf;
        inbuf = inbuf.add(m_mp3_decoder.m_MP3DecInfo.nSlots as usize);
        *bytes_left -= m_mp3_decoder.m_MP3DecInfo.nSlots;
    } else {
        if m_mp3_decoder.m_MP3DecInfo.nSlots > *bytes_left {
            MP3ClearBadFrame(outbuf);
            return ERR_MP3_INDATA_UNDERFLOW;
        }

        if m_mp3_decoder.m_MP3DecInfo.mainDataBytes >= m_mp3_decoder.m_MP3DecInfo.mainDataBegin {
            // memmove(mainBuf, mainBuf + mainDataBytes - mainDataBegin, mainDataBegin)
            core::ptr::copy(
                m_mp3_decoder.m_MP3DecInfo.mainBuf.as_ptr().add(
                    (m_mp3_decoder.m_MP3DecInfo.mainDataBytes
                        - m_mp3_decoder.m_MP3DecInfo.mainDataBegin) as usize,
                ),
                m_mp3_decoder.m_MP3DecInfo.mainBuf.as_mut_ptr(),
                m_mp3_decoder.m_MP3DecInfo.mainDataBegin as usize,
            );
            // memcpy(mainBuf + mainDataBegin, inbuf, nSlots)
            core::ptr::copy_nonoverlapping(
                inbuf,
                m_mp3_decoder
                    .m_MP3DecInfo
                    .mainBuf
                    .as_mut_ptr()
                    .add(m_mp3_decoder.m_MP3DecInfo.mainDataBegin as usize),
                m_mp3_decoder.m_MP3DecInfo.nSlots as usize,
            );

            m_mp3_decoder.m_MP3DecInfo.mainDataBytes =
                m_mp3_decoder.m_MP3DecInfo.mainDataBegin + m_mp3_decoder.m_MP3DecInfo.nSlots;
            inbuf = inbuf.add(m_mp3_decoder.m_MP3DecInfo.nSlots as usize);
            *bytes_left -= m_mp3_decoder.m_MP3DecInfo.nSlots;
            main_ptr = m_mp3_decoder.m_MP3DecInfo.mainBuf.as_mut_ptr();
        } else {
            // memcpy(mainBuf + mainDataBytes, inbuf, nSlots)
            core::ptr::copy_nonoverlapping(
                inbuf,
                m_mp3_decoder
                    .m_MP3DecInfo
                    .mainBuf
                    .as_mut_ptr()
                    .add(m_mp3_decoder.m_MP3DecInfo.mainDataBytes as usize),
                m_mp3_decoder.m_MP3DecInfo.nSlots as usize,
            );
            m_mp3_decoder.m_MP3DecInfo.mainDataBytes += m_mp3_decoder.m_MP3DecInfo.nSlots;
            inbuf = inbuf.add(m_mp3_decoder.m_MP3DecInfo.nSlots as usize);
            *bytes_left -= m_mp3_decoder.m_MP3DecInfo.nSlots;
            MP3ClearBadFrame(outbuf);
            return ERR_MP3_MAINDATA_UNDERFLOW;
        }
    }

    bit_offset = 0;
    main_bits = m_mp3_decoder.m_MP3DecInfo.mainDataBytes * 8;
    /* decode one complete frame */
    for gr in m_mp3_decoder.m_MP3DecInfo.nGrans.granules() {
        for ch in m_mp3_decoder.m_MP3DecInfo.nChans.channels() {
            prev_bit_offset = bit_offset;
            offset = unpack_scale_factors(
                main_ptr,
                &mut bit_offset,
                main_bits,
                *gr,
                *ch,
                &mut m_mp3_decoder.m_SideInfoSub, // 1. Oczekiwany: *mut [[SideInfoSub; 2]; 2]
                &mut m_mp3_decoder.m_ScaleFactorInfoSub, // 2. Oczekiwany: *mut [[ScaleFactorInfoSub; 2]; 2]
                &mut m_mp3_decoder.m_MP3DecInfo,         // 3. Oczekiwany: *mut MP3DecInfo
                &mut m_mp3_decoder.m_SideInfo,
                &mut m_mp3_decoder.m_FrameHeader, // 5. Oczekiwany: *mut FrameHeader
                &mut m_mp3_decoder.m_ScaleFactorJS, // 6. Oczekiwany: *mut ScaleFactorJS
                m_mp3_decoder.m_MPEGVersion,      // 7. Oczekiwany: i32
            );

            sf_block_bits = 8 * offset - prev_bit_offset + bit_offset;
            huff_block_bits =
                m_mp3_decoder.m_MP3DecInfo.part23Length[*gr as usize][*ch as usize] - sf_block_bits;
            main_ptr = main_ptr.add(offset as usize);
            main_bits -= sf_block_bits;

            if offset < 0 || main_bits < huff_block_bits {
                MP3ClearBadFrame(outbuf);
                return ERR_MP3_INVALID_SCALEFACT;
            }

            prev_bit_offset = bit_offset;
            offset = DecodeHuffman(
                main_ptr,
                &mut bit_offset,
                huff_block_bits,
                *gr,
                *ch,
                &mut m_mp3_decoder.m_HuffmanInfo,
                &mut m_mp3_decoder.m_SFBandTable,
                &mut m_mp3_decoder.m_SideInfoSub,
                m_mp3_decoder.m_MPEGVersion,
            );
            if offset < 0 {
                MP3ClearBadFrame(outbuf);
                return ERR_MP3_INVALID_HUFFCODES;
            }
            main_ptr = main_ptr.add(offset as usize);
            main_bits -= (8 * offset - prev_bit_offset + bit_offset);
        }

        if mp3_dequantize(*gr, m_mp3_decoder) < 0 {
            MP3ClearBadFrame(outbuf);
            return ERR_MP3_INVALID_DEQUANTIZE;
        }

        for ch in m_mp3_decoder.m_MP3DecInfo.nChans.channels() {
            if imdct(
                *gr,
                *ch,
                &mut m_mp3_decoder.m_SFBandTable,
                m_mp3_decoder.m_MPEGVersion,
                &mut m_mp3_decoder.m_SideInfoSub,
                &mut m_mp3_decoder.m_HuffmanInfo,
                &mut m_mp3_decoder.m_IMDCTInfo,
            ) < 0
            {
                MP3ClearBadFrame(outbuf);
                return ERR_MP3_INVALID_IMDCT;
            }
        }

        let pcm_offset = (*gr as i32
            * m_mp3_decoder.m_MP3DecInfo.nGranSamps
            * m_mp3_decoder.m_MP3DecInfo.nChans as i32) as usize;
        if m_mp3_decoder.subband(&mut outbuf[pcm_offset..]) < 0 {
            MP3ClearBadFrame(outbuf);
            return ERR_MP3_INVALID_SUBBAND; // ERR_MP3_INVALID_SUBBAND
        }
    }

    m_mp3_decoder.mp3_get_last_frame_info();

    ERR_MP3_NONE
}
