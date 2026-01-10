
pub struct BitStreamInfo<'a> {
    pub bytes: &'a [u8],
    pub cache: u32,
    pub cached_bits: i32,
}

impl<'a> BitStreamInfo<'a> {
    pub fn from_slice(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            cache: 0,
            cached_bits: 0,
        }
    }
    pub fn get_bits(&mut self, mut n_bits: u32) -> u32 {
        n_bits = n_bits.min(32);

        // Special case: requesting 0 bits
        if n_bits == 0 {
            return 0;
        }

        // Extract top n_bits from current cache
        let mut data = self.cache.wrapping_shr(32 - n_bits);

        // Consume the bits we just read
        self.cache = self.cache.wrapping_shl(n_bits);
        self.cached_bits -= n_bits as i32;

        // If we went negative → we crossed a 32-bit boundary → need to refill
        if self.cached_bits < 0 {
            let needed = (-self.cached_bits) as u32;  // positive amount needed from new cache

            self.refill_bitstream_cache();

            // OR in up to 'needed' bits from the freshly loaded cache
            let available = self.cached_bits.max(0) as u32;
            let take = needed.min(available);

            if take > 0 {
                data |= self.cache.wrapping_shr(32 - take);

                self.cache = self.cache.wrapping_shl(take);
                self.cached_bits -= take as i32;
            }
            // If no more data (EOF), low bits stay 0 — correct behavior
        }

        data
    }

    pub fn refill_bitstream_cache(&mut self) {
        let len = self.bytes.len();
        if len == 0 {
            self.cache = 0;
            self.cached_bits = 0;
        } else if len >= 4 {
            self.cache = u32::from_be_bytes([self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]]);
            self.cached_bits = 32;
            self.bytes = &self.bytes[4..];
        } else {
            self.cache = 0u32;
            for &byte in self.bytes {
                self.cache = (self.cache << 8) | (byte as u32);
            }
            let shift = 8 * (4-len);
            self.cache = self.cache << shift;
            self.cached_bits = (8 * len) as i32;
            self.bytes = &[];
        }
    }

    pub fn calc_bits_used(
        &self,
        start: &[u8],
        start_offset: usize,
    ) -> i32 {
        let mut bits_used = (start.len() as i32 - self.bytes.len() as i32) * 8;
        bits_used -= self.cached_bits;
        bits_used -= start_offset as i32;
        bits_used
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::bit_stream_cache::BitStreamInfo;

    fn make_bsi(data: &[u8]) -> BitStreamInfo<'_> {
        BitStreamInfo {
            bytes: data,
            cache: 0,
            cached_bits: 0,
        }
    }

    #[test]
    fn test_get_single_bits() {
        let data = [0b1011_0011];
        let mut bsi = make_bsi(&data);

        assert_eq!(bsi.get_bits(1), 1);
        assert_eq!(bsi.get_bits(1), 0);
        assert_eq!(bsi.get_bits(1), 1);
        assert_eq!(bsi.get_bits(1), 1);
        assert_eq!(bsi.get_bits(1), 0);
        assert_eq!(bsi.get_bits(1), 0);
        assert_eq!(bsi.get_bits(1), 1);
        assert_eq!(bsi.get_bits(1), 1);

        assert_eq!(bsi.cached_bits, 0);
        assert_eq!(bsi.bytes.len(), 0); // consumed 1 byte
    }

    #[test]
    fn test_get_various_sizes_from_cache() {
        let data = [0xAB, 0xCD, 0xEF]; // 0xABCDEF
        let mut bsi = make_bsi(&data);

        // First read 12 bits: should get 0xABC (0b1010_1011_1100)
        assert_eq!(bsi.get_bits(12), 0xABC);
        // Now cache has 12 bits left: 0xDEF (shifted up)
        // cache = 0xCDEF0000 >> (32-12) wait, let's verify next

        assert_eq!(bsi.get_bits(8), 0xDE); // next 8 bits
        assert_eq!(bsi.get_bits(4), 0xF);  // next 4 bits (0xE from 0xEF)
        assert_eq!(bsi.get_bits(4), 0x0);  // last 4 bits

        assert_eq!(bsi.cached_bits, 0);
    }

    #[test]
    fn test_split_across_refill() {
        let data = [0xF0, 0x0F, 0xAA];
        let mut bsi = make_bsi(&data);

        // Read 4 bits: 0b1111
        assert_eq!(bsi.get_bits(4), 0xF);
        // Now 4 bits left in cache: 0b0000 (from 0xF0)

        // Read 12 bits: should take remaining 4 (0x0) + next 8 (0x0F) = 0x00F
        assert_eq!(bsi.get_bits(12), 0x00F);

        // Now cache has 0xAA and possibly more, but we read 8 bits from refill
        assert_eq!(bsi.get_bits(8), 0xAA);

        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn test_full_32_bits() {
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut bsi = make_bsi(&data);

        assert_eq!(bsi.get_bits(32), 0x12345678);
        assert_eq!(bsi.cached_bits, 0);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn test_more_than_32_bits_capped() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
        let mut bsi = make_bsi(&data);

        // Should cap at 32 and return top 32 bits
        assert_eq!(bsi.get_bits(40), 0xFFFFFFFF);
        // After reading 32 bits, one byte remains in buffer, cache should be refilled
        bsi.refill_bitstream_cache(); // simulate next refill
        assert_eq!(bsi.cache, 0xFF000000);
        assert_eq!(bsi.cached_bits, 8);
    }

    #[test]
    fn test_small_reads_after_large() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut bsi = make_bsi(&data);

        assert_eq!(bsi.get_bits(16), 0xDEAD);
        assert_eq!(bsi.get_bits(3), 0b101); // first 3 bits of 0xBE = 0b10111110
        assert_eq!(bsi.get_bits(5), 0b11110); // next 5 bits
        assert_eq!(bsi.get_bits(8), 0xEF);
    }

    #[test]
    fn test_empty_stream_returns_zero() {
        let mut bsi = make_bsi(&[]);

        assert_eq!(bsi.get_bits(1), 0);
        assert_eq!(bsi.get_bits(16), 0);
        assert_eq!(bsi.get_bits(32), 0);
        assert_eq!(bsi.cached_bits, 0);
    }

    #[test]
    fn test_partial_final_byte() {
        let data = [0b1010_1010]; // 0b1010_1010
        let mut bsi = make_bsi(&data);

        assert_eq!(bsi.get_bits(3), 0b101);
        assert_eq!(bsi.get_bits(3), 0b010);
        assert_eq!(bsi.get_bits(2), 0b10);
        // Only 8 bits total, no more
        assert_eq!(bsi.get_bits(1), 0);
    }

        #[test]
    fn refill_empty() {
        let data = [];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();

        assert_eq!(bsi.cache, 0);
        assert_eq!(bsi.cached_bits, 0);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_full_4_bytes() {
        let data = [0x12, 0x34, 0x56, 0x78];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();

        assert_eq!(bsi.cache, 0x12345678);
        assert_eq!(bsi.cached_bits, 32);
        assert_eq!(bsi.bytes, &[][..]); // consumed 4 bytes
    }

    #[test]
    fn refill_full_more_than_4_bytes() {
        let data = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();

        assert_eq!(bsi.cache, 0xABCDEF12);
        assert_eq!(bsi.cached_bits, 32);
        assert_eq!(bsi.bytes, &[0x34, 0x56][..]); // left 2 bytes
    }

    #[test]
    fn refill_3_bytes() {
        let data = [0xAB, 0xCD, 0xEF];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();

        assert_eq!(bsi.cache, 0xABCDEF00); // left-justified!
        assert_eq!(bsi.cached_bits, 24);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_2_bytes() {
        let data = [0x12, 0x34];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();

        assert_eq!(bsi.cache, 0x12340000);
        assert_eq!(bsi.cached_bits, 16);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_1_byte() {
        let data = [0xFF];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();

        assert_eq!(bsi.cache, 0xFF000000);
        assert_eq!(bsi.cached_bits, 8);
        assert_eq!(bsi.bytes.len(), 0);
    }

    #[test]
    fn refill_multiple_calls() {
        let data = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let mut bsi = make_bsi(&data);

        bsi.refill_bitstream_cache();
        assert_eq!(bsi.cache, 0xAABBCCDD);
        assert_eq!(bsi.cached_bits, 32);
        assert_eq!(bsi.bytes, &[0xEE][..]);

        bsi.refill_bitstream_cache();
        assert_eq!(bsi.cache, 0xEE000000);
        assert_eq!(bsi.cached_bits, 8);
        assert_eq!(bsi.bytes.len(), 0);

        bsi.refill_bitstream_cache();
        assert_eq!(bsi.cache, 0);
        assert_eq!(bsi.cached_bits, 0);
    }
}