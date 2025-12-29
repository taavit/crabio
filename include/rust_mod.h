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

int MP3FindSyncWord(unsigned char *buf, int nBytes);

void RefillBitstreamCache(BitStreamInfo_t *bsi);
unsigned int GetBits(BitStreamInfo_t *bsi, int nBits);

uint64_t rust_add(uint64_t a, uint64_t b);

uint64_t SAR64(uint64_t x, int n);
uint64_t xSAR64(uint64_t x, int n);
int MULSHIFT32(int x, int y);
uint64_t MADD64(uint64_t sum64, int x, int y);

short ClipToShort(int x, int fracBits);

#ifdef __cplusplus
}
#endif