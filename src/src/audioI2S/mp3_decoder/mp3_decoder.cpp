/*
 * mp3_decoder.cpp
 * libhelix_HMP3DECODER
 *
 *  Created on: 26.10.2018
 *  Updated on: 27.05.2022
 */

#include "mp3_decoder.h"

const uint8_t  m_SYNCWORDH              =0xff;
const uint8_t  m_SYNCWORDL              =0xf0;
const uint8_t  m_DQ_FRACBITS_OUT        =25;  // number of fraction bits in output of dequant
const uint8_t  m_CSHIFT                 =12;  // coefficients have 12 leading sign bits for early-terminating mulitplies
const uint8_t  m_SIBYTES_MPEG1_MONO     =17;
const uint8_t  m_SIBYTES_MPEG1_STEREO   =32;
const uint8_t  m_SIBYTES_MPEG2_MONO     =9;
const uint8_t  m_SIBYTES_MPEG2_STEREO   =17;
const uint8_t  m_IMDCT_SCALE            =2;   // additional scaling (by sqrt(2)) for fast IMDCT36
const uint8_t  m_NGRANS_MPEG1           =2;
const uint8_t  m_NGRANS_MPEG2           =1;
const uint32_t m_SQRTHALF               =0x5a82799a;  // sqrt(0.5) in Q31 format


MP3FrameInfo_t *m_MP3FrameInfo;
SFBandTable_t m_SFBandTable;
StereoMode_t m_sMode;  /* mono/stereo mode */
MPEGVersion_t m_MPEGVersion;  /* version ID */
FrameHeader_t *m_FrameHeader;
SideInfoSub_t m_SideInfoSub[m_MAX_NGRAN][m_MAX_NCHAN];
SideInfo_t *m_SideInfo;
CriticalBandInfo_t m_CriticalBandInfo[m_MAX_NCHAN];  /* filled in dequantizer, used in joint stereo reconstruction */
DequantInfo_t *m_DequantInfo;
HuffmanInfo_t *m_HuffmanInfo;
IMDCTInfo_t *m_IMDCTInfo;
ScaleFactorInfoSub_t m_ScaleFactorInfoSub[m_MAX_NGRAN][m_MAX_NCHAN];
ScaleFactorJS_t *m_ScaleFactorJS;
SubbandInfo_t *m_SubbandInfo;
MP3DecInfo_t *m_MP3DecInfo;

const uint32_t polyCoef[264] PROGMEM = {
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
};

/* format = Q30, range = [0.0981, 1.9976]
 *
 * n = 16;
 * k = 0;
 * for(i=0; i<5; i++, n=n/2) {
 *   for(p=0; p<n; p++, k++) {
 *     t = (PI / (4*n)) * (2*p + 1);
 *     coef32[k] = 2.0 * cos(t);
 *   }
 * }
 * coef32[30] *= 0.5;   / *** for initial back butterfly (i.e. two-point DCT) *** /
 */
const int coef32[31] PROGMEM = {
    0x7fd8878d, 0x7e9d55fc, 0x7c29fbee, 0x78848413, 0x73b5ebd0, 0x6dca0d14, 0x66cf811f, 0x5ed77c89,
    0x55f5a4d2, 0x4c3fdff3, 0x41ce1e64, 0x36ba2013, 0x2b1f34eb, 0x1f19f97b, 0x12c8106e, 0x0647d97c,
    0x7f62368f, 0x7a7d055b, 0x70e2cbc6, 0x62f201ac, 0x5133cc94, 0x3c56ba70, 0x25280c5d, 0x0c8bd35e,
    0x7d8a5f3f, 0x6a6d98a4, 0x471cece6, 0x18f8b83c, 0x7641af3c, 0x30fbc54d, 0x2d413ccc,
};

/* let c(j) = cos(M_PI/36 * ((j)+0.5)), s(j) = sin(M_PI/36 * ((j)+0.5))
 * then fastWin[2*j+0] = c(j)*(s(j) + c(j)), j = [0, 8]
 *      fastWin[2*j+1] = c(j)*(s(j) - c(j))
 * format = Q30
 */
const uint32_t fastWin36[18] PROGMEM = {
        0x42aace8b, 0xc2e92724, 0x47311c28, 0xc95f619a, 0x4a868feb, 0xd0859d8c,
        0x4c913b51, 0xd8243ea0, 0x4d413ccc, 0xe0000000, 0x4c913b51, 0xe7dbc161,
        0x4a868feb, 0xef7a6275, 0x47311c28, 0xf6a09e67, 0x42aace8b, 0xfd16d8dd
};

const uint32_t imdctWin[4][36] PROGMEM = {
    {
    0x02aace8b, 0x07311c28, 0x0a868fec, 0x0c913b52, 0x0d413ccd, 0x0c913b52, 0x0a868fec, 0x07311c28,
    0x02aace8b, 0xfd16d8dd, 0xf6a09e66, 0xef7a6275, 0xe7dbc161, 0xe0000000, 0xd8243e9f, 0xd0859d8b,
    0xc95f619a, 0xc2e92723, 0xbd553175, 0xb8cee3d8, 0xb5797014, 0xb36ec4ae, 0xb2bec333, 0xb36ec4ae,
    0xb5797014, 0xb8cee3d8, 0xbd553175, 0xc2e92723, 0xc95f619a, 0xd0859d8b, 0xd8243e9f, 0xe0000000,
    0xe7dbc161, 0xef7a6275, 0xf6a09e66, 0xfd16d8dd  },
    {
    0x02aace8b, 0x07311c28, 0x0a868fec, 0x0c913b52, 0x0d413ccd, 0x0c913b52, 0x0a868fec, 0x07311c28,
    0x02aace8b, 0xfd16d8dd, 0xf6a09e66, 0xef7a6275, 0xe7dbc161, 0xe0000000, 0xd8243e9f, 0xd0859d8b,
    0xc95f619a, 0xc2e92723, 0xbd44ef14, 0xb831a052, 0xb3aa3837, 0xafb789a4, 0xac6145bb, 0xa9adecdc,
    0xa864491f, 0xad1868f0, 0xb8431f49, 0xc8f42236, 0xdda8e6b1, 0xf47755dc, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000  },
    {
    0x07311c28, 0x0d413ccd, 0x07311c28, 0xf6a09e66, 0xe0000000, 0xc95f619a, 0xb8cee3d8, 0xb2bec333,
    0xb8cee3d8, 0xc95f619a, 0xe0000000, 0xf6a09e66, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000,
    0x00000000, 0x00000000, 0x00000000, 0x00000000  },
    {
    0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x00000000, 0x028e9709, 0x04855ec0,
    0x026743a1, 0xfcde2c10, 0xf515dc82, 0xec93e53b, 0xe4c880f8, 0xdd5d0b08, 0xd63510b7, 0xcf5e834a,
    0xc8e6b562, 0xc2da4105, 0xbd553175, 0xb8cee3d8, 0xb5797014, 0xb36ec4ae, 0xb2bec333, 0xb36ec4ae,
    0xb5797014, 0xb8cee3d8, 0xbd553175, 0xc2e92723, 0xc95f619a, 0xd0859d8b, 0xd8243e9f, 0xe0000000,
    0xe7dbc161, 0xef7a6275, 0xf6a09e66, 0xfd16d8dd  },
};

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
int UnpackScaleFactors( unsigned char *buf, int *bitOffset, int bitsAvail, int gr, int ch){
    int bitsUsed;
    unsigned char *startBuf;
    BitStreamInfo_t bitStreamInfo, *bsi;

    /* init GetBits reader */
    startBuf = buf;
    bsi = &bitStreamInfo;
    SetBitstreamPointer(bsi, (bitsAvail + *bitOffset + 7) / 8, buf);
    if (*bitOffset)
        GetBits(bsi, *bitOffset);

    if (m_MPEGVersion == MPEG1)
        UnpackSFMPEG1(bsi, &m_SideInfoSub[gr][ch], &m_ScaleFactorInfoSub[gr][ch],
                      m_SideInfo->scfsi[ch], gr, &m_ScaleFactorInfoSub[0][ch]);
    else
        UnpackSFMPEG2(bsi, &m_SideInfoSub[gr][ch], &m_ScaleFactorInfoSub[gr][ch],
                      gr, ch, m_FrameHeader->modeExt, m_ScaleFactorJS);

    m_MP3DecInfo->part23Length[gr][ch] = m_SideInfoSub[gr][ch].part23Length;

    bitsUsed = CalcBitsUsed(bsi, buf, *bitOffset);
    buf += (bitsUsed + *bitOffset) >> 3;
    *bitOffset = (bitsUsed + *bitOffset) & 0x07;

    return (buf - startBuf);
}
/***********************************************************************************************************************
 * M P 3 D E C
 **********************************************************************************************************************/

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
void MP3GetLastFrameInfo() {
    if (m_MP3DecInfo->layer != 3){
        m_MP3FrameInfo->bitrate=0;
        m_MP3FrameInfo->nChans=0;
        m_MP3FrameInfo->samprate=0;
        m_MP3FrameInfo->bitsPerSample=0;
        m_MP3FrameInfo->outputSamps=0;
        m_MP3FrameInfo->layer=0;
        m_MP3FrameInfo->version=0;
    }
    else{
        m_MP3FrameInfo->bitrate=m_MP3DecInfo->bitrate;
        m_MP3FrameInfo->nChans=m_MP3DecInfo->nChans;
        m_MP3FrameInfo->samprate=m_MP3DecInfo->samprate;
        m_MP3FrameInfo->bitsPerSample=16;
        m_MP3FrameInfo->outputSamps=m_MP3DecInfo->nChans
                * (int) samplesPerFrameTab[m_MPEGVersion][m_MP3DecInfo->layer-1];
        m_MP3FrameInfo->layer=m_MP3DecInfo->layer;
        m_MP3FrameInfo->version=m_MPEGVersion;
    }
}
int MP3GetSampRate(){return m_MP3FrameInfo->samprate;}
int MP3GetChannels(){return m_MP3FrameInfo->nChans;}
int MP3GetBitsPerSample(){return m_MP3FrameInfo->bitsPerSample;}
int MP3GetBitrate(){return m_MP3FrameInfo->bitrate;}
int MP3GetOutputSamps(){return m_MP3FrameInfo->outputSamps;}

/***********************************************************************************************************************
 * Function:    MP3Decode
 *
 * Description: decode one frame of MP3 data
 *
 * Inputs:      number of valid bytes remaining in inbuf
 *              pointer to outbuf, big enough to hold one frame of decoded PCM samples
 *              flag indicating whether MP3 data is normal MPEG format (useSize = 0)
 *              or reformatted as "self-contained" frames (useSize = 1)
 *
 * Outputs:     PCM data in outbuf, interleaved LRLRLR... if stereo
 *              number of output samples = nGrans * nGranSamps * nChans
 *              updated inbuf pointer, updated bytesLeft
 *
 * Return:      error code, defined in mp3dec.h (0 means no error, < 0 means error)
 *
 * Notes:       switching useSize on and off between frames in the same stream
 *                is not supported (bit reservoir is not maintained if useSize on)
 **********************************************************************************************************************/
int MP3Decode( unsigned char *inbuf, size_t inbuf_len, int *bytesLeft, short *outbuf, int useSize){
    int offset, bitOffset, mainBits, gr, ch, fhBytes, siBytes, freeFrameBytes;
    int prevBitOffset, sfBlockBits, huffBlockBits;
    unsigned char *mainPtr;

    /* unpack frame header */
    fhBytes = UnpackFrameHeader(inbuf, inbuf_len, m_FrameHeader, m_MP3DecInfo, &m_MPEGVersion, &m_sMode, &m_SFBandTable);
    if (fhBytes < 0)
        return ERR_MP3_INVALID_FRAMEHEADER; /* don't clear outbuf since we don't know size (failed to parse header) */
    inbuf += fhBytes;
    /* unpack side info */
    siBytes = UnpackSideInfo( inbuf,
        m_SideInfo,
        &m_SideInfoSub,
        m_MP3DecInfo,
        m_MPEGVersion,     // 1 = MPEG1, 0 = MPEG2/2.5
        m_sMode
);
    if (siBytes < 0) {
        MP3ClearBadFrame(m_MP3DecInfo, outbuf);
        return ERR_MP3_INVALID_SIDEINFO;
    }
    inbuf += siBytes;
    *bytesLeft -= (fhBytes + siBytes);

    /* if free mode, need to calculate bitrate and nSlots manually, based on frame size */
    if (m_MP3DecInfo->bitrate == 0 || m_MP3DecInfo->freeBitrateFlag) {
        if(!m_MP3DecInfo->freeBitrateFlag){
            /* first time through, need to scan for next sync word and figure out frame size */
            m_MP3DecInfo->freeBitrateFlag=1;
            m_MP3DecInfo->freeBitrateSlots=MP3FindFreeSync(inbuf, inbuf - fhBytes - siBytes, *bytesLeft);
            if(m_MP3DecInfo->freeBitrateSlots < 0){
                MP3ClearBadFrame(m_MP3DecInfo, outbuf);
                m_MP3DecInfo->freeBitrateFlag = 0;
                return ERR_MP3_FREE_BITRATE_SYNC;
            }
            freeFrameBytes=m_MP3DecInfo->freeBitrateSlots + fhBytes + siBytes;
            m_MP3DecInfo->bitrate=(freeFrameBytes * m_MP3DecInfo->samprate * 8)
                    / (m_MP3DecInfo->nGrans * m_MP3DecInfo->nGranSamps);
        }
        m_MP3DecInfo->nSlots = m_MP3DecInfo->freeBitrateSlots + CheckPadBit(); /* add pad byte, if required */
    }

    /* useSize != 0 means we're getting reformatted (RTP) packets (see RFC 3119)
     *  - calling function assembles "self-contained" MP3 frames by shifting any main_data
     *      from the bit reservoir (in previous frames) to AFTER the sync word and side info
     *  - calling function should set mainDataBegin to 0, and tell us exactly how large this
     *      frame is (in bytesLeft)
     */
    if (useSize) {
        m_MP3DecInfo->nSlots = *bytesLeft;
        if (m_MP3DecInfo->mainDataBegin != 0 || m_MP3DecInfo->nSlots <= 0) {
            /* error - non self-contained frame, or missing frame (size <= 0), could do loss concealment here */
            MP3ClearBadFrame(m_MP3DecInfo, outbuf);
            return ERR_MP3_INVALID_FRAMEHEADER;
        }

        /* can operate in-place on reformatted frames */
        m_MP3DecInfo->mainDataBytes = m_MP3DecInfo->nSlots;
        mainPtr = inbuf;
        inbuf += m_MP3DecInfo->nSlots;
        *bytesLeft -= (m_MP3DecInfo->nSlots);
    } else {
        /* out of data - assume last or truncated frame */
        if (m_MP3DecInfo->nSlots > *bytesLeft) {
            MP3ClearBadFrame(m_MP3DecInfo, outbuf);
            return ERR_MP3_INDATA_UNDERFLOW;
        }
        /* fill main data buffer with enough new data for this frame */
        if (m_MP3DecInfo->mainDataBytes >= m_MP3DecInfo->mainDataBegin) {
            /* adequate "old" main data available (i.e. bit reservoir) */
            memmove(m_MP3DecInfo->mainBuf,
                    m_MP3DecInfo->mainBuf + m_MP3DecInfo->mainDataBytes - m_MP3DecInfo->mainDataBegin,
                    m_MP3DecInfo->mainDataBegin);
            memcpy (m_MP3DecInfo->mainBuf + m_MP3DecInfo->mainDataBegin, inbuf,
                    m_MP3DecInfo->nSlots);

            m_MP3DecInfo->mainDataBytes = m_MP3DecInfo->mainDataBegin + m_MP3DecInfo->nSlots;
            inbuf += m_MP3DecInfo->nSlots;
            *bytesLeft -= (m_MP3DecInfo->nSlots);
            mainPtr = m_MP3DecInfo->mainBuf;
        } else {
            /* not enough data in bit reservoir from previous frames (perhaps starting in middle of file) */
            memcpy(m_MP3DecInfo->mainBuf + m_MP3DecInfo->mainDataBytes, inbuf, m_MP3DecInfo->nSlots);
            m_MP3DecInfo->mainDataBytes += m_MP3DecInfo->nSlots;
            inbuf += m_MP3DecInfo->nSlots;
            *bytesLeft -= (m_MP3DecInfo->nSlots);
            MP3ClearBadFrame(m_MP3DecInfo, outbuf);
            return ERR_MP3_MAINDATA_UNDERFLOW;
        }
    }
    bitOffset = 0;
    mainBits = m_MP3DecInfo->mainDataBytes * 8;

    /* decode one complete frame */
    for (gr = 0; gr < m_MP3DecInfo->nGrans; gr++) {
        for (ch = 0; ch < m_MP3DecInfo->nChans; ch++) {
            /* unpack scale factors and compute size of scale factor block */
            prevBitOffset = bitOffset;
            offset = UnpackScaleFactors( mainPtr, &bitOffset,
                    mainBits, gr, ch);
            sfBlockBits = 8 * offset - prevBitOffset + bitOffset;
            huffBlockBits = m_MP3DecInfo->part23Length[gr][ch] - sfBlockBits;
            mainPtr += offset;
            mainBits -= sfBlockBits;

            if (offset < 0 || mainBits < huffBlockBits) {
                MP3ClearBadFrame(m_MP3DecInfo, outbuf);
                return ERR_MP3_INVALID_SCALEFACT;
            }
            /* decode Huffman code words */
            prevBitOffset = bitOffset;
            offset = DecodeHuffman(
                mainPtr, &bitOffset, huffBlockBits, gr, ch,
                m_HuffmanInfo,
                &m_SFBandTable,
                &m_SideInfoSub,
                &m_MPEGVersion
            );
            if (offset < 0) {
                MP3ClearBadFrame(m_MP3DecInfo, outbuf);
                return ERR_MP3_INVALID_HUFFCODES;
            }
            mainPtr += offset;
            mainBits -= (8 * offset - prevBitOffset + bitOffset);
        }
        /* dequantize coefficients, decode stereo, reorder short blocks */
        if (MP3Dequantize( gr) < 0) {
            MP3ClearBadFrame(m_MP3DecInfo, outbuf);
            return ERR_MP3_INVALID_DEQUANTIZE;
        }

        /* alias reduction, inverse MDCT, overlap-add, frequency inversion */
        for (ch = 0; ch < m_MP3DecInfo->nChans; ch++) {
            if (IMDCT( gr, ch,
                &m_SFBandTable,
        m_MPEGVersion,
        &m_SideInfoSub,
        m_HuffmanInfo,
        m_IMDCTInfo
            ) < 0) {
                MP3ClearBadFrame(m_MP3DecInfo, outbuf);
                return ERR_MP3_INVALID_IMDCT;
            }
        }
        /* subband transform - if stereo, interleaves pcm LRLRLR */
        if (Subband(
                outbuf + gr * m_MP3DecInfo->nGranSamps * m_MP3DecInfo->nChans)
                < 0) {
            MP3ClearBadFrame(m_MP3DecInfo, outbuf);
            return ERR_MP3_INVALID_SUBBAND;
        }
    }
    MP3GetLastFrameInfo();
    return ERR_MP3_NONE;
}

/***********************************************************************************************************************
 * Function:    MP3Decoder_ClearBuffer
 *
 * Description: clear all the memory needed for the MP3 decoder
 *
 * Inputs:      none
 *
 * Outputs:     none
 *
 * Return:      none
 *
 **********************************************************************************************************************/
void MP3Decoder_ClearBuffer(void) {

    /* important to do this - DSP primitives assume a bunch of state variables are 0 on first use */
    memset( m_MP3DecInfo,         0, sizeof(MP3DecInfo_t));                                    //Clear MP3DecInfo
    memset(&m_ScaleFactorInfoSub, 0, sizeof(ScaleFactorInfoSub_t)*(m_MAX_NGRAN *m_MAX_NCHAN)); //Clear ScaleFactorInfo
    memset( m_SideInfo,           0, sizeof(SideInfo_t));                                      //Clear SideInfo
    memset( m_FrameHeader,        0, sizeof(FrameHeader_t));                                   //Clear FrameHeader
    memset( m_HuffmanInfo,        0, sizeof(HuffmanInfo_t));                                   //Clear HuffmanInfo
    memset( m_DequantInfo,        0, sizeof(DequantInfo_t));                                   //Clear DequantInfo
    memset( m_IMDCTInfo,          0, sizeof(IMDCTInfo_t));                                     //Clear IMDCTInfo
    memset( m_SubbandInfo,        0, sizeof(SubbandInfo_t));                                   //Clear SubbandInfo
    memset(&m_CriticalBandInfo,   0, sizeof(CriticalBandInfo_t)*m_MAX_NCHAN);                  //Clear CriticalBandInfo
    memset( m_ScaleFactorJS,      0, sizeof(ScaleFactorJS_t));                                 //Clear ScaleFactorJS
    memset(&m_SideInfoSub,        0, sizeof(SideInfoSub_t)*(m_MAX_NGRAN *m_MAX_NCHAN));        //Clear SideInfoSub
    memset(&m_SFBandTable,        0, sizeof(SFBandTable_t));                                   //Clear SFBandTable
    memset( m_MP3FrameInfo,       0, sizeof(MP3FrameInfo_t));                                  //Clear MP3FrameInfo

    return;

}
/***********************************************************************************************************************
 * Function:    MP3Decoder_AllocateBuffers
 *
 * Description: allocate all the memory needed for the MP3 decoder
 *
 * Inputs:      none
 *
 * Outputs:     none
 *
 * Return:      pointer to MP3DecInfo structure (initialized with pointers to all
 *                the internal buffers needed for decoding)
 *
 * Notes:       if one or more mallocs fail, function frees any buffers already
 *                allocated before returning
 *
 **********************************************************************************************************************/

#ifdef CONFIG_IDF_TARGET_ESP32S3
    // ESP32-S3: If there is PSRAM, prefer it
    #define __malloc_heap_psram(size) \
        heap_caps_malloc_prefer(size, 2, MALLOC_CAP_DEFAULT|MALLOC_CAP_SPIRAM, MALLOC_CAP_DEFAULT|MALLOC_CAP_INTERNAL)
#else
    // ESP32, PSRAM is too slow, prefer SRAM
    #define __malloc_heap_psram(size) \
        heap_caps_malloc_prefer(size, 2, MALLOC_CAP_DEFAULT|MALLOC_CAP_INTERNAL, MALLOC_CAP_DEFAULT|MALLOC_CAP_SPIRAM)
#endif

bool MP3Decoder_AllocateBuffers(void) {
    if(!m_MP3DecInfo)       {m_MP3DecInfo    = (MP3DecInfo_t*)    __malloc_heap_psram(sizeof(MP3DecInfo_t)   );}
    if(!m_FrameHeader)      {m_FrameHeader   = (FrameHeader_t*)   __malloc_heap_psram(sizeof(FrameHeader_t)  );}
    if(!m_SideInfo)         {m_SideInfo      = (SideInfo_t*)      __malloc_heap_psram(sizeof(SideInfo_t)     );}
    if(!m_ScaleFactorJS)    {m_ScaleFactorJS = (ScaleFactorJS_t*) __malloc_heap_psram(sizeof(ScaleFactorJS_t));}
    if(!m_HuffmanInfo)      {m_HuffmanInfo   = (HuffmanInfo_t*)   __malloc_heap_psram(sizeof(HuffmanInfo_t)  );}
    if(!m_DequantInfo)      {m_DequantInfo   = (DequantInfo_t*)   __malloc_heap_psram(sizeof(DequantInfo_t)  );}
    if(!m_IMDCTInfo)        {m_IMDCTInfo     = (IMDCTInfo_t*)     __malloc_heap_psram(sizeof(IMDCTInfo_t)    );}
    if(!m_SubbandInfo)      {m_SubbandInfo   = (SubbandInfo_t*)   __malloc_heap_psram(sizeof(SubbandInfo_t)  );}
    if(!m_MP3FrameInfo)     {m_MP3FrameInfo  = (MP3FrameInfo_t*)  __malloc_heap_psram(sizeof(MP3FrameInfo_t) );}

    if(!m_MP3DecInfo || !m_FrameHeader || !m_SideInfo || !m_ScaleFactorJS || !m_HuffmanInfo ||
       !m_DequantInfo || !m_IMDCTInfo || !m_SubbandInfo || !m_MP3FrameInfo) {
        MP3Decoder_FreeBuffers();
        log_e("not enough memory to allocate mp3decoder buffers");
        return false;
    }
    MP3Decoder_ClearBuffer();
    return true;
}
/***********************************************************************************************************************
 * Function:    MP3Decoder_FreeBuffers
 *
 * Description: frees all the memory used by the MP3 decoder
 *
 * Inputs:      pointer to initialized MP3DecInfo structure
 *
 * Outputs:     none
 *
 * Return:      none
 *
 * Notes:       safe to call even if some buffers were not allocated
 **********************************************************************************************************************/
void MP3Decoder_FreeBuffers()
{
//    uint32_t i = ESP.getFreeHeap();

    if(m_MP3DecInfo)        {free(m_MP3DecInfo);      m_MP3DecInfo=NULL;}
    if(m_FrameHeader)       {free(m_FrameHeader);     m_FrameHeader=NULL;}
    if(m_SideInfo)          {free(m_SideInfo);        m_SideInfo=NULL;}
    if(m_ScaleFactorJS )    {free(m_ScaleFactorJS);   m_ScaleFactorJS=NULL;}
    if(m_HuffmanInfo)       {free(m_HuffmanInfo);     m_HuffmanInfo=NULL;}
    if(m_DequantInfo)       {free(m_DequantInfo);     m_DequantInfo=0;}
    if(m_IMDCTInfo)         {free(m_IMDCTInfo);       m_IMDCTInfo=0;}
    if(m_SubbandInfo)       {free(m_SubbandInfo);     m_SubbandInfo=0;}
    if(m_MP3FrameInfo)      {free(m_MP3FrameInfo);    m_MP3FrameInfo=0;}

//    log_i("MP3Decoder: %lu bytes memory was freed", ESP.getFreeHeap() - i);
}

/***********************************************************************************************************************
 * H U F F M A N N
 **********************************************************************************************************************/

/***********************************************************************************************************************
 * D E Q U A N T
 **********************************************************************************************************************/

/***********************************************************************************************************************
 * Function:    MP3Dequantize
 *
 * Description: dequantize coefficients, decode stereo, reorder short blocks
 *                (one granule-worth)
 *
 * Inputs:      index of current granule
 *
 * Outputs:     dequantized and reordered coefficients in hi->huffDecBuf
 *                (one granule-worth, all channels), format = Q26
 *              operates in-place on huffDecBuf but also needs di->workBuf
 *              updated hi->nonZeroBound index for both channels
 *
 * Return:      0 on success, -1 if null input pointers
 *
 * Notes:       In calling output Q(DQ_FRACBITS_OUT), we assume an implicit bias
 *                of 2^15. Some (floating-point) reference implementations factor this
 *                into the 2^(0.25 * gain) scaling explicitly. But to avoid precision
 *                loss, we don't do that. Instead take it into account in the final
 *                round to PCM (>> by 15 less than we otherwise would have).
 *              Equivalently, we can think of the dequantized coefficients as
 *                Q(DQ_FRACBITS_OUT - 15) with no implicit bias.
 **********************************************************************************************************************/
int MP3Dequantize(int gr){
    int i, ch, nSamps, mOut[2];
    CriticalBandInfo_t *cbi;
    cbi = &m_CriticalBandInfo[0];
    mOut[0] = mOut[1] = 0;

    /* dequantize all the samples in each channel */
    for (ch = 0; ch < m_MP3DecInfo->nChans; ch++) {
        m_HuffmanInfo->gb[ch] = DequantChannel(m_HuffmanInfo->huffDecBuf[ch], m_DequantInfo->workBuf,
                &m_HuffmanInfo->nonZeroBound[ch], &m_SideInfoSub[gr][ch], &m_ScaleFactorInfoSub[gr][ch], &cbi[ch],
                m_FrameHeader,
                &m_SFBandTable,
                m_MPEGVersion
            );
    }

    /* joint stereo processing assumes one guard bit in input samples
     * it's extremely rare not to have at least one gb, so if this is the case
     *   just make a pass over the data and clip to [-2^30+1, 2^30-1]
     * in practice this may never happen
     */
    if (m_FrameHeader->modeExt && (m_HuffmanInfo->gb[0] < 1 || m_HuffmanInfo->gb[1] < 1)) {
        for (i = 0; i < m_HuffmanInfo->nonZeroBound[0]; i++) {
            if (m_HuffmanInfo->huffDecBuf[0][i] < -0x3fffffff)  m_HuffmanInfo->huffDecBuf[0][i] = -0x3fffffff;
            if (m_HuffmanInfo->huffDecBuf[0][i] >  0x3fffffff)  m_HuffmanInfo->huffDecBuf[0][i] =  0x3fffffff;
        }
        for (i = 0; i < m_HuffmanInfo->nonZeroBound[1]; i++) {
            if (m_HuffmanInfo->huffDecBuf[1][i] < -0x3fffffff)  m_HuffmanInfo->huffDecBuf[1][i] = -0x3fffffff;
            if (m_HuffmanInfo->huffDecBuf[1][i] >  0x3fffffff)  m_HuffmanInfo->huffDecBuf[1][i] =  0x3fffffff;
        }
    }

    /* do mid-side stereo processing, if enabled */
    if (m_FrameHeader->modeExt >> 1) {
        if (m_FrameHeader->modeExt & 0x01) {
            /* intensity stereo enabled - run mid-side up to start of right zero region */
            if (cbi[1].cbType == 0)
                nSamps = m_SFBandTable.l[cbi[1].cbEndL + 1];
            else
                nSamps = 3 * m_SFBandTable.s[cbi[1].cbEndSMax + 1];
        } else {
            /* intensity stereo disabled - run mid-side on whole spectrum */
            nSamps = (m_HuffmanInfo->nonZeroBound[0] > m_HuffmanInfo->nonZeroBound[1] ?
                                                       m_HuffmanInfo->nonZeroBound[0] : m_HuffmanInfo->nonZeroBound[1]);
        }
        MidSideProc(m_HuffmanInfo->huffDecBuf, nSamps, mOut);
    }

    /* do intensity stereo processing, if enabled */
    if (m_FrameHeader->modeExt & 0x01) {
        nSamps = m_HuffmanInfo->nonZeroBound[0];
        if (m_MPEGVersion == MPEG1) {
            IntensityProcMPEG1(m_HuffmanInfo->huffDecBuf, nSamps, &m_ScaleFactorInfoSub[gr][1], &m_CriticalBandInfo[0],
                    m_FrameHeader->modeExt >> 1, m_SideInfoSub[gr][1].mixedBlock, mOut);
        } else {
            IntensityProcMPEG2(m_HuffmanInfo->huffDecBuf, nSamps, &m_ScaleFactorInfoSub[gr][1], &m_CriticalBandInfo[0],
                    m_ScaleFactorJS, m_FrameHeader->modeExt >> 1, m_SideInfoSub[gr][1].mixedBlock, mOut);
        }
    }

    /* adjust guard bit count and nonZeroBound if we did any stereo processing */
    if (m_FrameHeader->modeExt) {
        m_HuffmanInfo->gb[0] = CLZ(mOut[0]) - 1;
        m_HuffmanInfo->gb[1] = CLZ(mOut[1]) - 1;
        nSamps = (m_HuffmanInfo->nonZeroBound[0] > m_HuffmanInfo->nonZeroBound[1] ?
                                                       m_HuffmanInfo->nonZeroBound[0] : m_HuffmanInfo->nonZeroBound[1]);
        m_HuffmanInfo->nonZeroBound[0] = nSamps;
        m_HuffmanInfo->nonZeroBound[1] = nSamps;
    }

    /* output format Q(DQ_FRACBITS_OUT) */
    return 0;
}

/***********************************************************************************************************************
 * D Q C H A N
 **********************************************************************************************************************/

/***********************************************************************************************************************
 * S T P R O C
 **********************************************************************************************************************/


/***********************************************************************************************************************
 * I M D C T
 **********************************************************************************************************************/

/***********************************************************************************************************************
 * S U B B A N D
 **********************************************************************************************************************/

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
int Subband( short *pcmBuf) {
    int b;
    if (m_MP3DecInfo->nChans == 2) {
        /* stereo */
        for (b = 0; b < m_BLOCK_SIZE; b++) {
            FDCT32(m_IMDCTInfo->outBuf[0][b], m_SubbandInfo->vbuf + 0 * 32, m_SubbandInfo->vindex,
                    (b & 0x01), m_IMDCTInfo->gb[0]);
            FDCT32(m_IMDCTInfo->outBuf[1][b], m_SubbandInfo->vbuf + 1 * 32, m_SubbandInfo->vindex,
                    (b & 0x01), m_IMDCTInfo->gb[1]);
            PolyphaseStereo(pcmBuf,
                    m_SubbandInfo->vbuf + m_SubbandInfo->vindex + m_VBUF_LENGTH * (b & 0x01),
                    polyCoef);
            m_SubbandInfo->vindex = (m_SubbandInfo->vindex - (b & 0x01)) & 7;
            pcmBuf += (2 * m_NBANDS);
        }
    } else {
        /* mono */
        for (b = 0; b < m_BLOCK_SIZE; b++) {
            FDCT32(m_IMDCTInfo->outBuf[0][b], m_SubbandInfo->vbuf + 0 * 32, m_SubbandInfo->vindex,
                    (b & 0x01), m_IMDCTInfo->gb[0]);
            PolyphaseMono(pcmBuf,
                    m_SubbandInfo->vbuf + m_SubbandInfo->vindex + m_VBUF_LENGTH * (b & 0x01),
                    polyCoef);
            m_SubbandInfo->vindex = (m_SubbandInfo->vindex - (b & 0x01)) & 7;
            pcmBuf += m_NBANDS;
        }
    }

    return 0;
}
