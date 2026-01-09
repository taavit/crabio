/*
 * mp3_decoder.cpp
 * libhelix_HMP3DECODER
 *
 *  Created on: 26.10.2018
 *  Updated on: 27.05.2022
 */

#include "mp3_decoder.h"

MP3Decoder_t *m_MP3Decoder;

/***********************************************************************************************************************
 * M P 3 D E C
 **********************************************************************************************************************/


int MP3GetSampRate(){return m_MP3Decoder->m_MP3FrameInfo.samprate;}
int MP3GetChannels(){return m_MP3Decoder->m_MP3FrameInfo.nChans;}
int MP3GetBitsPerSample(){return m_MP3Decoder->m_MP3FrameInfo.bitsPerSample;}
int MP3GetBitrate(){return m_MP3Decoder->m_MP3FrameInfo.bitrate;}
int MP3GetOutputSamps(){return m_MP3Decoder->m_MP3FrameInfo.outputSamps;}

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

        m_MP3Decoder
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
    memset( m_MP3Decoder,         0, sizeof(MP3Decoder_t));                                    //Clear MP3DecInfo

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
    if(!m_MP3Decoder)       {m_MP3Decoder    = (MP3Decoder_t*)    __malloc_heap_psram(sizeof(MP3Decoder_t)   );}
    if(!m_MP3Decoder) {
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

    if(m_MP3Decoder)        {free(m_MP3Decoder);      m_MP3Decoder=NULL;}

//    log_i("MP3Decoder: %lu bytes memory was freed", ESP.getFreeHeap() - i);
}
