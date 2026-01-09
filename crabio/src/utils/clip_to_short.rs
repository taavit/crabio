#[inline(always)]
pub fn clip_to_short(mut x: i32, frac_bits: i32) -> i16 {
    x >>= frac_bits;

    #[cfg(target_arch = "xtensa")]
    unsafe {
        core::arch::asm!(
            "clamps {0}, {0}, 15",
            inout(reg) x,
            options(nomem, nostack, preserves_flags),
        );
    }

    #[cfg(not(target_arch = "xtensa"))]
    {
        x = x.clamp(i16::MIN as i32, i16::MAX as i32);
    }

    x as i16
}

#[cfg(test)]
mod tests {
    use crate::utils::clip_to_short::clip_to_short;

    #[test]
    fn test_no_shift_no_clipping() {
        // frac_bits = 0 → no shift, just clipping
        assert_eq!(clip_to_short(0, 0), 0);
        assert_eq!(clip_to_short(100, 0), 100);
        assert_eq!(clip_to_short(-5000, 0), -5000);
        assert_eq!(clip_to_short(32767, 0), 32767);
        assert_eq!(clip_to_short(-32768, 0), -32768);
    }

    #[test]
    fn test_positive_clipping() {
        // Values that exceed i16::MAX after shift should be clamped to 32767
        assert_eq!(clip_to_short(32767 << 1, 1), 32767); // 65534 >> 1 = 32767 → no clip
        assert_eq!(clip_to_short(32768 << 1, 1), 32767); // 65536 >> 1 = 32768 → clipped
        assert_eq!(clip_to_short(1_000_000, 0), 32767);
        assert_eq!(clip_to_short(2_147_483_647, 0), 32767); // i32::MAX
        assert_eq!(clip_to_short(40000, 1), 20000); // 40000 >> 1 = 20000 → no clip?
        assert_eq!(clip_to_short(100_000, 1), 32767); // 100000 >> 1 = 50000 → still > 32767 → clipped
    }

    #[test]
    fn test_negative_clipping() {
        // Values that go below i16::MIN after shift should be clamped to -32768
        assert_eq!(clip_to_short(-32768 << 1, 1), -32768); // -65536 >> 1 = -32768 → no clip
        assert_eq!(clip_to_short((-32768 << 1) - 2, 1), -32768); // -65538 >> 1 = -32769 → clipped
        assert_eq!(clip_to_short(-100_000, 0), -32768);
        assert_eq!(clip_to_short(-2_147_483_648, 0), -32768); // i32::MIN
    }

    #[test]
    fn test_exact_edges() {
        assert_eq!(clip_to_short(32767, 0), 32767);
        assert_eq!(clip_to_short(32768, 0), 32767);
        assert_eq!(clip_to_short(-32768, 0), -32768);
        assert_eq!(clip_to_short(-32769, 0), -32768);

        // After shifting
        assert_eq!(clip_to_short(32767 << 5, 5), 32767);
        assert_eq!(clip_to_short((32767 << 5) + 1, 5), 32767);
        assert_eq!(clip_to_short(-32768 << 5, 5), -32768);
        assert_eq!(clip_to_short((-32768 << 5) - 1, 5), -32768);
    }

    #[test]
    fn test_large_frac_bits() {
        // Shifting by many bits → most values become 0 or -1 (for negative)
        assert_eq!(clip_to_short(12345, 20), 0);
        assert_eq!(clip_to_short(-12345, 20), -1); // arithmetic right shift in Rust for signed
        assert_eq!(clip_to_short(1 << 30, 30), 1);
        assert_eq!(clip_to_short(-1 << 30, 30), -1);
        assert_eq!(clip_to_short(1_000_000, 31), 0);
        assert_eq!(clip_to_short(-1_000_000, 31), -1);
    }

    #[test]
    fn test_frac_bits_zero() {
        // No shift, pure clipping
        for i in -40000..=40000 {
            let expected = i.clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            assert_eq!(clip_to_short(i, 0), expected);
        }
    }

    #[test]
    fn test_small_shifts() {
        assert_eq!(clip_to_short(1, 1), 0);
        assert_eq!(clip_to_short(3, 1), 1);
        assert_eq!(clip_to_short(4, 1), 2);
        assert_eq!(clip_to_short(-3, 1), -2);
        // But let's verify actual behavior:
        assert_eq!(clip_to_short(-5, 1), -3);
        // Better to test known values
        assert_eq!(clip_to_short(-1, 0), -1);
        assert_eq!(clip_to_short(-1 << 10, 10), -1);
    }
}

