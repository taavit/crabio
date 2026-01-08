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

/***********************************************************************************************************************
 * M P 3 D E C
 **********************************************************************************************************************/


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
    return MP3DecodeHelper(
        inbuf,
        inbuf_len,
        bytesLeft,
        outbuf,
        useSize,

        m_FrameHeader,
        m_MP3DecInfo,
        &m_MPEGVersion,
        &m_sMode,
        &m_SFBandTable,
        m_SideInfo,
        &m_SideInfoSub,
        m_HuffmanInfo,
        m_DequantInfo,
        &m_ScaleFactorInfoSub,
        &m_CriticalBandInfo,
        m_ScaleFactorJS,
        m_IMDCTInfo,
        m_SubbandInfo,
        m_MP3FrameInfo
    );
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

