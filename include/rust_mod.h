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

typedef struct HuffmanInfo {
    int huffDecBuf[m_MAX_NCHAN][m_MAX_NSAMP];       /* used both for decoded Huffman values and dequantized coefficients */
    int nonZeroBound[m_MAX_NCHAN];                /* number of coeffs in huffDecBuf[ch] which can be > 0 */
    int gb[m_MAX_NCHAN];                          /* minimum number of guard bits in huffDecBuf[ch] */
} HuffmanInfo_t;

typedef struct ScaleFactorJS { /* used in MPEG 2, 2.5 intensity (joint) stereo only */
    int intensityScale;
    int slen[4];
    int nr[4];
} ScaleFactorJS_t;

typedef struct ScaleFactorInfoSub {    /* max bits in scalefactors = 5, so use char's to save space */
    char l[23];            /* [band] */
    char s[13][3];         /* [band][window] */
} ScaleFactorInfoSub_t;

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

/* indexing = [version][layer][bitrate index]
 * bitrate (kbps) of frame
 *   - bitrate index == 0 is "free" mode (bitrate determined on the fly by
 *       counting bits between successive sync words)
 */
const short bitrateTab[3][3][15] PROGMEM = { {
/* MPEG-1 */
{ 0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448 }, /* Layer 1 */
{ 0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384 }, /* Layer 2 */
{ 0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320 }, /* Layer 3 */
}, {
/* MPEG-2 */
{ 0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256 }, /* Layer 1 */
{ 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160 }, /* Layer 2 */
{ 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160 }, /* Layer 3 */
}, {
/* MPEG-2.5 */
{ 0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256 }, /* Layer 1 */
{ 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160 }, /* Layer 2 */
{ 0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160 }, /* Layer 3 */
}, };

/* indexing = [version][sampleRate][bitRate]
 * for layer3, nSlots = floor(samps/frame * bitRate / sampleRate / 8)
 *   - add one pad slot if necessary
 */
const short slotTab[3][3][15] PROGMEM = {
    { /* MPEG-1 */
        { 0, 104, 130, 156, 182, 208, 261, 313, 365, 417, 522, 626, 731, 835, 1044 }, /* 44 kHz */
        { 0, 96, 120, 144, 168, 192, 240, 288, 336, 384, 480, 576, 672, 768, 960 }, /* 48 kHz */
        { 0, 144, 180, 216, 252, 288, 360, 432, 504, 576, 720, 864, 1008, 1152, 1440 }, /* 32 kHz */
    },
    { /* MPEG-2 */
        { 0, 26, 52, 78, 104, 130, 156, 182, 208, 261, 313, 365, 417, 470, 522 }, /* 22 kHz */
        { 0, 24, 48, 72, 96, 120, 144, 168, 192, 240, 288, 336, 384, 432, 480 }, /* 24 kHz */
        { 0, 36, 72, 108, 144, 180, 216, 252, 288, 360, 432, 504, 576, 648, 720 }, /* 16 kHz */
    },
    { /* MPEG-2.5 */
        { 0, 52, 104, 156, 208, 261, 313, 365, 417, 522, 626, 731, 835, 940, 1044 }, /* 11 kHz */
        { 0, 48, 96, 144, 192, 240, 288, 336, 384, 480, 576, 672, 768, 864, 960 }, /* 12 kHz */
        { 0, 72, 144, 216, 288, 360, 432, 504, 576, 720, 864, 1008, 1152, 1296, 1440 }, /*  8 kHz */
    },
};



/* indexing = [version][layer]
 * number of samples in one frame (per channel)
 */
const int/*short*/samplesPerFrameTab[3][3] PROGMEM = { { 384, 1152, 1152 }, /* MPEG1 */
{ 384, 1152, 576 }, /* MPEG2 */
{ 384, 1152, 576 }, /* MPEG2.5 */
};

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
int UnpackFrameHeader(
    unsigned char *buf,
    size_t inbuf_len,
    FrameHeader_t *m_FrameHeader,
    MP3DecInfo *m_MP3DecInfo,
    MPEGVersion_t *m_MPEGVersion,
    StereoMode_t *m_sMode,
    SFBandTable *m_SFBandTable
);

typedef struct SideInfoSub {
    int part23Length;       /* number of bits in main data */
    int nBigvals;           /* 2x this = first set of Huffman cw's (maximum amplitude can be > 1) */
    int globalGain;         /* overall gain for dequantizer */
    int sfCompress;         /* unpacked to figure out number of bits in scale factors */
    int winSwitchFlag;      /* window switching flag */
    int blockType;          /* block type */
    int mixedBlock;         /* 0 = regular block (all short or long), 1 = mixed block */
    int tableSelect[3];     /* index of Huffman tables for the big values regions */
    int subBlockGain[3];    /* subblock gain offset, relative to global gain */
    int region0Count;       /* 1+region0Count = num scale factor bands in first region of bigvals */
    int region1Count;       /* 1+region1Count = num scale factor bands in second region of bigvals */
    int preFlag;            /* for optional high frequency boost */
    int sfactScale;         /* scaling of the scalefactors */
    int count1TableSelect;  /* index of Huffman table for quad codewords */
} SideInfoSub_t;

typedef struct SideInfo {
    int mainDataBegin;
    int privateBits;
    int scfsi[m_MAX_NCHAN][m_MAX_SCFBD];                /* 4 scalefactor bands per channel */
} SideInfo_t;


void RefillBitstreamCache(BitStreamInfo_t *bsi);
unsigned int GetBits(BitStreamInfo_t *bsi, int nBits);
void SetBitstreamPointer(BitStreamInfo_t *bsi, int nBytes, unsigned char *buf);
int CheckPadBit();
int CalcBitsUsed(BitStreamInfo_t *bsi, unsigned char *startBuf, int startOffset);

uint64_t rust_add(uint64_t a, uint64_t b);

void imdct12(int *x, int *out);
void idct9(int *x);

uint64_t SAR64(uint64_t x, int n);
uint64_t xSAR64(uint64_t x, int n);
int MULSHIFT32(int x, int y);
uint64_t MADD64(uint64_t sum64, int x, int y);


void FDCT32(int *buf, int *dest, int offset, int oddBlock, int gb);
int IMDCT36(int *xCurr, int *xPrev, int *y, int btCurr, int btPrev, int blockIdx, int gb);
short ClipToShort(int x, int fracBits);
int FreqInvertRescale(int *y, int *xPrev, int blockIdx, int es);

void PolyphaseStereo(short *pcm, int *vbuf, const uint32_t *coefBase);
void PolyphaseMono(short *pcm, int *vbuf, const uint32_t *coefBase);
void WinPrevious(int *xPrev, int *xPrevWin, int btPrev);
void MP3ClearBadFrame(MP3DecInfo_t *m_MP3DecInfo, short *outbuf);
void UnpackSFMPEG1(BitStreamInfo_t *bsi, SideInfoSub_t *sis, ScaleFactorInfoSub_t *sfis, int *scfsi, int gr, ScaleFactorInfoSub_t *sfisGr0);
void UnpackSFMPEG2(BitStreamInfo_t *bsi, SideInfoSub_t *sis, ScaleFactorInfoSub_t *sfis, int gr, int ch, int modeExt, ScaleFactorJS_t *sfjs);

int UnpackSideInfo(
    unsigned char *buf,
    SideInfo_t *m_SideInfo,
    SideInfoSub_t (*m_SideInfoSub)[m_MAX_NCHAN][m_MAX_NGRAN],
    MP3DecInfo_t *m_MP3DecInfo,
    int m_MPEGVersion,     // 1 = MPEG1, 0 = MPEG2/2.5
    int m_sMode
);

int DecodeHuffmanPairs(int *xy, int nVals, int tabIdx, int bitsLeft, unsigned char *buf, int bitOffset);
int DecodeHuffmanQuads(int *vwxy, int nVals, int tabIdx, int bitsLeft, unsigned char *buf, int bitOffset);

int DecodeHuffman(
    unsigned char *buf,
    int *bitOffset,
    int huffBlockBits,
    int gr,
    int ch,
    HuffmanInfo_t *m_HuffmanInfo,
    SFBandTable_t *m_SFBandTable,
    SideInfoSub_t (*m_SideInfoSub)[2][2],
    MPEGVersion_t *m_MPEGVersion
);

#ifdef __cplusplus
}
#endif