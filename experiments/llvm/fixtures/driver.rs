use std::{hint::black_box, time::Instant};

extern "C" {
    fn mix(input: u64) -> u64;
    fn small(input: u8) -> u8;
    fn medium(input: u16) -> u16;
    fn signed(input: i32) -> i32;
}

fn reference(input: u64) -> u64 {
    let x = input.wrapping_add(0x1357_9bdf_2468_ace0);
    let y = (x ^ (x >> 17)).wrapping_mul(0xd6e8_feb8_6659_fd93);
    if y & 0xff < 97 {
        y.wrapping_add(0xa5a5_5a5a_1234_5678)
    } else {
        y ^ 0x7f4a_7c15_9e37_79b9
    }
}

fn main() {
    for input in 0..=u8::MAX {
        assert_eq!(
            unsafe { small(input) },
            input.wrapping_mul(37).wrapping_add(119) ^ 0xa5
        );
    }
    for input in 0..=u16::MAX {
        assert_eq!(
            unsafe { medium(input) },
            input.wrapping_mul(257).wrapping_sub(1023) ^ 0xa55a
        );
    }
    for input in [i32::MIN, i32::MAX, -1025, -1024, -1, 0, 1] {
        assert_eq!(unsafe { signed(input) }, signed_reference(input));
    }
    for input in [0, 1, u64::MAX, u64::MAX - 1, 1 << 63, (1 << 63) - 1] {
        assert_eq!(unsafe { mix(input) }, reference(input));
    }
    let mut input = 0x1234_5678_9abc_def0_u64;
    for _ in 0..100_000 {
        input = input.wrapping_mul(6364136223846793005).wrapping_add(1);
        assert_eq!(unsafe { mix(input) }, reference(input));
        assert_eq!(
            unsafe { signed(input as i32) },
            signed_reference(input as i32)
        );
    }

    let start = Instant::now();
    let mut checksum = 0_u64;
    for input in 0..2_000_000_u64 {
        checksum ^= unsafe { mix(black_box(input)) };
    }
    println!("{checksum} {}", start.elapsed().as_nanos());
}

fn signed_reference(input: i32) -> i32 {
    if input < -1024 {
        input.wrapping_sub(8191)
    } else {
        input.wrapping_mul(-37).wrapping_add(13)
    }
}
