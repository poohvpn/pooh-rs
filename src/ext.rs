use async_trait::async_trait;
use std::future::Future;

pub trait BytesExt {
    fn u16(&self) -> u16;
    fn u32(&self) -> u32;
    fn u64(&self) -> u64;
    fn usize(&self) -> usize;
    fn checksum(&self) -> u16;
}

impl BytesExt for [u8] {
    fn u16(&self) -> u16 {
        self.iter()
            .take(2)
            .fold(0u16, |a, b| (a << 8) + (*b as u16))
    }

    fn u32(&self) -> u32 {
        self.iter()
            .take(4)
            .fold(0u32, |a, b| (a << 8) + (*b as u32))
    }

    fn u64(&self) -> u64 {
        self.iter()
            .take(8)
            .fold(0u64, |a, b| (a << 8) + (*b as u64))
    }

    fn usize(&self) -> usize {
        self.iter()
            .take((usize::BITS / 8) as usize)
            .fold(0usize, |a, b| (a << 8) + (*b as usize))
    }

    fn checksum(&self) -> u16 {
        let length = self.len() - 1;
        let mut csum = 0u32;
        for i in (0..length).step_by(2) {
            csum += ((self[i] as u32) << 8) + (self[i + 1] as u32)
        }
        if length % 2 == 0 {
            csum += (self[length] as u32) << 8
        }
        while csum > 0xffff {
            csum = (csum >> 16) + (csum & 0xffff)
        }
        !csum as u16
    }
}

pub trait AnyExt: Sized {
    fn type_name(&self) -> &'static str;

    fn debug_type(self) -> Self {
        eprintln!("{}", self.type_name());
        self
    }
    fn drop(self) {
        drop(self)
    }
}

impl<T> AnyExt for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}

pub fn type_of<T>(_: &T) -> &'static str {
    std::any::type_name::<T>()
}

pub fn type_of2<T>(v: T) -> (&'static str, T) {
    (std::any::type_name::<T>(), v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u16() {
        for i in 0..=8 {
            assert_eq!(vec![0u8; i].u16(), 0);
        }
        assert_eq!([0, 1].u16(), 1);
        assert_eq!([0, 0, 1].u16(), 0);
        assert_eq!([0xff, 0xff].u16(), 0xffff);
        assert_eq!([0x12, 0x34].u16(), 0x1234);
    }

    #[test]
    fn test_u32() {
        for i in 0..=8 {
            assert_eq!(vec![0u8; i].u32(), 0);
        }
        assert_eq!([0, 0, 0, 1].u32(), 1);
        assert_eq!([0, 0, 0, 0, 1].u32(), 0);
        assert_eq!([0xff, 0xff].u32(), 0xffff);
        assert_eq!([0x12, 0x34].u32(), 0x1234);
    }

    #[test]
    fn test_u64() {
        for i in 0..=8 {
            assert_eq!(vec![0u8; i].u64(), 0);
        }
        assert_eq!([0, 0, 0, 1].u64(), 1);
        assert_eq!([0, 0, 0, 0, 0, 0, 0, 0, 1].u64(), 0);
        assert_eq!([0xff, 0xff].u64(), 0xffff);
        assert_eq!([0x12, 0x34].u64(), 0x1234);
    }

    #[test]
    fn test_usize() {
        for i in 0..=8 {
            assert_eq!(vec![0u8; i].usize(), 0);
        }
        assert_eq!([0, 0, 0, 1].usize(), 1);
        assert_eq!([0xff, 0xff].usize(), 0xffff);
        assert_eq!([0x12, 0x34].usize(), 0x1234);
    }

    #[test]
    fn test_checksum() {
        assert_eq!(
            hex::decode("00000000a91dc7365cc861240a090002ffffff000a0900010808080801010101051408bad5e789aafe821aca0aedc5538d2f3d").unwrap().checksum(),
            0x8b78u16
        );
        assert_eq!(hex::decode("00000000").unwrap().checksum(), 0xffffu16);
        assert_eq!(
            hex::decode("000000001234123412341234").unwrap().checksum(),
            0xb72fu16
        );
    }

    #[test]
    fn test_debug() {
        let a: f32 = 112.3;
        assert!(debug!(a <= 114f32));
    }
}
