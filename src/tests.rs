use super::*;

#[test]
fn test_encode_idx() {
    assert_eq!(encode_idx(0), [0, 0, 0]);
    assert_eq!(encode_idx(10), [0, 0, 10]);
    assert_eq!(encode_idx(256), [0, 1, 0]);
    assert_eq!(encode_idx(0xF3_A7_09), [0xF3u8, 0xA7u8, 0x09u8]);
    assert_eq!(encode_idx(0x76_00_11), [0x76u8, 0x00u8, 0x11u8]);
    assert_eq!(encode_idx(0xFFFFFF), [0xFFu8, 0xFFu8, 0xFFu8]);
}

#[test]
fn test_reserve_idx() {
    let vec: &mut Vec<u8> = &mut vec![1, 5, 8];
    assert_eq!(reserve_idx(vec), 3);
    vec.push(13);
    assert_eq!(vec, &vec![1, 5, 8, RESERVED_BYTE, RESERVED_BYTE, RESERVED_BYTE, 13]);
}

#[test]
fn test_write_idx() {
    let vec: &mut Vec<u8> = &mut vec![1, 5, 8];
    let pos = reserve_idx(vec);
    vec.push(13);
    vec.push(16);
    write_idx(vec, pos, 42);
    assert_eq!(vec, &vec![1, 5, 8, 0, 0, 42, 13, 16]);
}

#[test]
fn test_decode_idx() {
    assert_eq!(decode_idx(&[0, 0, 0]), 0);
    assert_eq!(decode_idx(&[0, 0, 10]), 10);
    assert_eq!(decode_idx(&[0, 1, 0]), 256);
    assert_eq!(decode_idx(&[0xF3, 0xA7, 0x09]), 0xF3_A7_09);
    assert_eq!(decode_idx(&[0x76, 0x00, 0x11]), 0x76_00_11);
    assert_eq!(decode_idx(&[0xFF, 0xFF, 0xFF]), 0xFFFFFF);
}
