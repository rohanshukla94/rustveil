#![no_std]

#[no_mangle]
pub extern "C" fn mix(input: u64) -> u64 {
    let x = input.wrapping_add(0x1357_9bdf_2468_ace0);
    let y = (x ^ (x >> 17)).wrapping_mul(0xd6e8_feb8_6659_fd93);
    if y & 0xff < 97 {
        y.wrapping_add(0xa5a5_5a5a_1234_5678)
    } else {
        y ^ 0x7f4a_7c15_9e37_79b9
    }
}

#[no_mangle]
pub extern "C" fn small(input: u8) -> u8 {
    input.wrapping_mul(37).wrapping_add(119) ^ 0xa5
}

#[no_mangle]
pub extern "C" fn medium(input: u16) -> u16 {
    input.wrapping_mul(257).wrapping_sub(1023) ^ 0xa55a
}

#[no_mangle]
pub extern "C" fn signed(input: i32) -> i32 {
    if input < -1024 {
        input.wrapping_sub(8191)
    } else {
        input.wrapping_mul(-37).wrapping_add(13)
    }
}
