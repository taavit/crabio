#pragma once

#ifdef __cplusplus
extern "C" {
#endif

static const uint8_t  m_HUFF_PAIRTABS          =32;
static const uint8_t  m_BLOCK_SIZE             =18;
static const uint8_t  m_NBANDS                 =32;
static const uint8_t  m_MAX_REORDER_SAMPS      =(192-126)*3;      // largest critical band for short blocks (see sfBandTable)
static const uint16_t m_VBUF_LENGTH            =17*2* m_NBANDS;    // for double-sized vbuf FIFO
static const uint8_t  m_MAX_SCFBD              =4;     // max scalefactor bands per channel
static const uint16_t m_MAINBUF_SIZE           =1940;
static const uint8_t  m_MAX_NGRAN              =2;     // max granules
static const uint8_t  m_MAX_NCHAN              =2;     // max channels
static const uint16_t m_MAX_NSAMP              =576;   // max samples per channel, per granule

typedef struct BitStreamInfo {
    unsigned char *bytePtr;
    unsigned int iCache;
    int cachedBits;
    int nBytes;
} BitStreamInfo_t;

typedef struct FrameHeader {
    int layer;              /* layer index (1, 2, or 3) */
    int crc;                /* CRC flag: 0 = disabled, 1 = enabled */
    int brIdx;              /* bitrate index (0 - 15) */
    int srIdx;              /* sample rate index (0 - 2) */
    int paddingBit;         /* padding flag: 0 = no padding, 1 = single pad byte */
    int privateBit;         /* unused */
    int modeExt;            /* used to decipher joint stereo mode */
    int copyFlag;           /* copyright flag: 0 = no, 1 = yes */
    int origFlag;           /* original flag: 0 = copy, 1 = original */
    int emphasis;           /* deemphasis mode */
    int CRCWord;            /* CRC word (16 bits, 0 if crc not enabled) */
} FrameHeader_t;

typedef struct MP3DecInfo {
    /* buffer which must be large enough to hold largest possible main_data section */
    unsigned char mainBuf[m_MAINBUF_SIZE];
    /* special info for "free" bitrate files */
    int freeBitrateFlag;
    int freeBitrateSlots;
    /* user-accessible info */
    int bitrate;
    int nChans;
    int samprate;
    int nGrans;             /* granules per frame */
    int nGranSamps;         /* samples per granule */
    int nSlots;
    int layer;

    int mainDataBegin;
    int mainDataBytes;
    int part23Length[m_MAX_NGRAN][m_MAX_NCHAN];
} MP3DecInfo_t;

/* indexing = [version][samplerate index]
 * sample rate of frame (Hz)
 */
const int samplerateTab[3][3] = {
        { 44100, 48000, 32000 }, /* MPEG-1 */
        { 22050, 24000, 16000 }, /* MPEG-2 */
        { 11025, 12000, 8000  }, /* MPEG-2.5 */
};

/* indexing = [version][mono/stereo]
 * number of bytes in side info section of bitstream
 */
const int/*short*/sideBytesTab[3][2] = { { 17, 32 }, /* MPEG-1:   mono, stereo */
{ 9, 17 }, /* MPEG-2:   mono, stereo */
{ 9, 17 }, /* MPEG-2.5: mono, stereo */
};

typedef struct SFBandTable {
    int/*short*/ l[23];
    int/*short*/ s[14];
} SFBandTable_t;

/* indexing = [version][sampleRate][long (.l) or short (.s) block]
 *   sfBandTable[v][s].l[cb] = index of first bin in critical band cb (long blocks)
 *   sfBandTable[v][s].s[cb] = index of first bin in critical band cb (short blocks)
 */
const SFBandTable_t sfBandTable[3][3] = {
    { /* MPEG-1 (44, 48, 32 kHz) */
        {   {0, 4, 8, 12, 16, 20, 24, 30, 36, 44, 52,  62,  74,  90, 110, 134, 162, 196, 238, 288, 342, 418, 576 },
            {0, 4, 8, 12, 16, 22, 30, 40, 52, 66, 84, 106, 136, 192}    },
        {   {0, 4, 8, 12, 16, 20, 24, 30, 36, 42, 50,  60,  72,  88, 106, 128, 156, 190, 230, 276, 330, 384, 576 },
            {0, 4, 8, 12, 16, 22, 28, 38, 50, 64, 80, 100, 126, 192}    },
        {   {0, 4, 8, 12, 16, 20, 24, 30, 36, 44,  54,  66,  82, 102, 126, 156, 194, 240, 296, 364, 448, 550, 576 },
            {0, 4, 8, 12, 16, 22, 30, 42, 58, 78, 104, 138, 180, 192}   }   },
    { /* MPEG-2 (22, 24, 16 kHz) */
        {   {0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 },
            {0, 4,  8, 12, 18, 24, 32, 42, 56, 74, 100, 132, 174, 192}  },
        {   {0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 114, 136, 162, 194, 232, 278, 332, 394, 464, 540, 576 },
            {0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 136, 180, 192}  },
        {   {0, 6, 12, 18, 24, 30, 36, 44, 54, 66, 80, 96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 },
            {0, 4, 8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192}   },  },
    { /* MPEG-2.5 (11, 12, 8 kHz) */
        {   {0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 },
            {0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192 }  },
        {   {0, 6, 12, 18, 24, 30, 36, 44, 54, 66,  80,  96, 116, 140, 168, 200, 238, 284, 336, 396, 464, 522, 576 },
            {0, 4,  8, 12, 18, 26, 36, 48, 62, 80, 104, 134, 174, 192 }  },
        {   {0, 12, 24, 36, 48, 60, 72, 88, 108, 132, 160, 192, 232, 280, 336, 400, 476, 566, 568, 570, 572, 574, 576 },
            {0,  8, 16, 24, 36, 52, 72, 96, 124, 160, 162, 164, 166, 192 }   },   },
};

typedef enum {          /* map to 0,1,2 to make table indexing easier */
    MPEG1 =  0,
    MPEG2 =  1,
    MPEG25 = 2
} MPEGVersion_t;

typedef struct MP3FrameInfo {
    int bitrate;
    int nChans;
    int samprate;
    int bitsPerSample;
    int outputSamps;
    int layer;
    int version;
} MP3FrameInfo_t;

typedef enum {          /* map these to the corresponding 2-bit values in the frame header */
    Stereo = 0x00,      /* two independent channels, but L and R frames might have different # of bits */
    Joint = 0x01,       /* coupled channels - layer III: mix of M-S and intensity, Layers I/II: intensity and direct coding only */
    Dual = 0x02,        /* two independent channels, L and R always have exactly 1/2 the total bitrate */
    Mono = 0x03         /* one channel */
} StereoMode_t;


int CLIP_2N(int y, uint32_t n);

int MP3FindSyncWord(unsigned char *buf, int nBytes);
int MP3FindFreeSync(unsigned char *buf, unsigned char firstFH[4], int nBytes);

void RefillBitstreamCache(BitStreamInfo_t *bsi);
unsigned int GetBits(BitStreamInfo_t *bsi, int nBits);

uint64_t rust_add(uint64_t a, uint64_t b);

void imdct12(int *x, int *out);
void idct9(int *x);

uint64_t SAR64(uint64_t x, int n);
uint64_t xSAR64(uint64_t x, int n);
int MULSHIFT32(int x, int y);
uint64_t MADD64(uint64_t sum64, int x, int y);


void FDCT32(int *buf, int *dest, int offset, int oddBlock, int gb);
short ClipToShort(int x, int fracBits);
int FreqInvertRescale(int *y, int *xPrev, int blockIdx, int es);

void PolyphaseStereo(short *pcm, int *vbuf, const uint32_t *coefBase);
void PolyphaseMono(short *pcm, int *vbuf, const uint32_t *coefBase);
void WinPrevious(int *xPrev, int *xPrevWin, int btPrev);

#ifdef __cplusplus
}
#endif