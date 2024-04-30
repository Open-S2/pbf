#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::{cell::RefCell, mem};
use num_traits::{FromPrimitive, PrimInt};

const MAX_VARINT_LENGTH: usize = mem::size_of::<u64>() * 8 / 7 + 1;
const BIT_SHIFT: [u64; 10] = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63];

const BIG_ENDIAN: bool = cfg!(targem_endian = "big");

pub enum Type {
    Varint = 0,  // varint: int32, int64, uint32, uint64, sint32, sint64, bool, enum
    Fixed64 = 1, // 64-bit: double, fixed64, sfixed64
    Bytes = 2,   // len-delimited: string, bytes, embedded messages, packed repeated fields
    Fixed32 = 5, // 32-bit: float, fixed32, sfixed32
}
impl From<u8> for Type {
    fn from(val: u8) -> Self {
        match val & 0x7 {
            0 => Type::Varint,
            1 => Type::Fixed64,
            2 => Type::Bytes,
            5 => Type::Fixed32,
            _ => panic!("Invalid value for Type"),
        }
    }
}
impl From<Type> for u8 {
    fn from(t: Type) -> Self {
        match t {
            Type::Varint => 0,
            Type::Fixed64 => 1,
            Type::Bytes => 2,
            Type::Fixed32 => 5,
        }
    }
}
impl From<Type> for u64 {
    fn from(t: Type) -> Self {
        match t {
            Type::Varint => 0,
            Type::Fixed64 => 1,
            Type::Bytes => 2,
            Type::Fixed32 => 5,
        }
    }
}

// impl From<i8> for u64 {
//     fn from(val: i8) -> Self {
//         val as u64
//     }
// }
// impl From<i16> for u64 {
//     fn from(val: i16) -> Self {
//         val as u64
//     }
// }
// impl From<i32> for u64 {
//     fn from(val: i32) -> Self {
//         val as u64
//     }
// }
// impl From<i64> for u64 {
//     fn from(val: i64) -> Self {
//         val as u64
//     }
// }

/// sadf
pub struct Field {
    tag: u64,
    r#type: Type,
}

pub struct Protobuf {
    buf: RefCell<Vec<u8>>,
    pos: usize,
    len: usize,
}
impl Protobuf {
    pub fn new() -> Protobuf {
        let buf = RefCell::new(Vec::new());
        Protobuf {
            buf,
            pos: 0,
            len: 0,
        }
    }
    pub fn from_input(buf: RefCell<Vec<u8>>) -> Protobuf {
        let len = buf.borrow().len();
        Protobuf { buf, pos: 0, len }
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    // === READING =================================================================

    fn decode_varint(&mut self) -> u64 {
        let mut val: u64 = 0;
        let buf = self.buf.borrow();

        for n in 0..MAX_VARINT_LENGTH {
            let b = buf[self.pos] as u64;
            self.pos += 1;
            if n == 0 {
                if b & 0x80 == 0 {
                    return b;
                }
                val = b & 0x7f;
            } else {
                val |= (b & 0x7f) << (BIT_SHIFT[n]);
            }
            if b < 0x80 {
                break;
            }
        }

        val
    }

    pub fn read_value(&mut self) -> u64 {
        if self.pos >= self.len {
            unreachable!();
        }

        self.decode_varint()
    }

    pub fn skip(&mut self, t: Type) {
        match t {
            Type::Varint => _ = self.read_value(),
            Type::Fixed64 => self.pos += 8,
            Type::Fixed32 => self.pos += 4,
            Type::Bytes => self.pos = self.read_value() as usize + self.pos,
        };
    }

    pub fn read_field(&mut self) -> Field {
        let val = self.read_value();
        return Field {
            tag: val >> 3,
            r#type: Type::from(val as u8 & 0x7),
        };
    }

    pub fn read_bytes(&mut self) -> Vec<u8> {
        let end = self.read_value() as usize + self.pos;
        let buf = self.buf.borrow();
        let bytes = buf[self.pos..end].to_vec();
        self.pos += end - self.pos;

        bytes
    }

    pub fn read_string(&mut self) -> String {
        String::from_utf8(self.read_bytes()).expect("Invalid UTF-8")
    }

    pub fn read_fixed<T: Default>(&mut self) -> T
    where
        T: Default + PrimInt + FromPrimitive,
    {
        let buf = self.buf.borrow();
        let mut val: T = T::default();
        let size = core::mem::size_of::<T>();

        let mut n = 0;
        while n < size {
            val = val | (T::from_u8(buf[self.pos]).unwrap() << (n << 3));
            self.pos += 1;
            n += 1;
        }

        if BIG_ENDIAN {
            val = val.swap_bytes();
        }

        val
    }

    pub fn read_varint<T>(&mut self) -> T
    where
        T: From<u64>,
    {
        T::from(self.read_value())
    }

    pub fn read_bool(&mut self) -> bool {
        self.read_value() != 0
    }

    pub fn read_s_varint<T>(&mut self) -> T
    where
        T: From<i64>,
    {
        T::from(zagzig(self.read_value()))
    }

    pub fn read_packed<T>(&mut self) -> Vec<T>
    where
        T: From<u64>,
    {
        let end = self.read_value() as usize + self.pos;
        let mut res: Vec<T> = Vec::new();
        while self.pos < end {
            res.push(self.read_varint::<T>());
        }

        res
    }

    pub fn read_s_packed<T>(&mut self) -> Vec<T>
    where
        T: From<i64>,
    {
        let end = self.read_value() as usize + self.pos;
        let mut res: Vec<T> = Vec::new();
        while self.pos < end {
            res.push(self.read_s_varint::<T>());
        }

        res
    }

    // === WRITING =================================================================

    fn write_varint(&mut self, val: u64) {
        let mut buf = self.buf.borrow_mut();
        let mut val = val;

        while val > 0x80 {
            buf.push((val & 0x7f) as u8 | 0x80);
            val >>= 7;
        }
        buf.push(val as u8);
    }

    fn write_varint_to_buffer(buf: &mut Vec<u8>, val: u64) {
        let mut val = val;

        while val > 0x80 {
            buf.push((val & 0x7f) as u8 | 0x80);
            val >>= 7;
        }
        buf.push(val as u8);
    }

    fn write_s_varint(&mut self, val: i64) {
        self.write_varint(zigzag(val));
    }

    fn write_value<T>(&mut self, val: T)
    where
        T: Into<u64>,
    {
        let val = Into::<u64>::into(val);
        self.write_varint(val);
    }

    fn write_fixed<T>(&mut self, val: T)
    where
        T: Into<u64>,
    {
        let size = mem::size_of::<T>();
        let mut val = Into::<u64>::into(val);
        let mut buf = self.buf.borrow_mut();

        if BIG_ENDIAN {
            val = val.swap_bytes();
        }

        let mut n = 0;
        while n < size {
            buf.push((val >> (n << 3)) as u8);
            n += 1;
        }
    }
}

pub fn zigzag(val: i64) -> u64 {
    ((val << 1) ^ (val >> 63)) as u64
}

pub fn zagzig(val: u64) -> i64 {
    (val >> 1) as i64 ^ -((val & 1) as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let buf = Vec::new();
        let pb = Protobuf::from_input(RefCell::new(buf));
        assert_eq!(pb.pos, 0);
        assert_eq!(pb.len, 0);
    }

    #[test]
    fn test_zigzag() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zagzig(0), 0);
        assert_eq!(zagzig(zigzag(0)), 0);
        assert_eq!(zagzig(zigzag(5)), 5);
        assert_eq!(zagzig(zigzag(-5)), -5);

        let max_i64 = i64::MAX;
        assert_eq!(zagzig(zigzag(max_i64)), max_i64);
    }
}
