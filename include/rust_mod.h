#pragma once

#ifdef __cplusplus
extern "C" {
#endif

typedef struct BitStreamInfo {
    unsigned char *bytePtr;
    unsigned int iCache;
    int cachedBits;
    int nBytes;
} BitStreamInfo_t;

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

void PolyphaseStereo(short *pcm, int *vbuf, const uint32_t *coefBase);
void PolyphaseMono(short *pcm, int *vbuf, const uint32_t *coefBase);

#ifdef __cplusplus
}
#endif