#![no_std]

pub mod bit_cast;

extern crate alloc;

use alloc::{borrow::ToOwned, string::String, vec::Vec};
use bit_cast::BitCast;
use core::{cell::RefCell, mem::size_of};

const MAX_VARINT_LENGTH: usize = u64::BITS as usize * 8 / 7 + 1;
const BIT_SHIFT: [u64; 10] = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63];

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
pub struct Field {
    tag: u64,
    r#type: Type,
}

pub trait ProtoRead {
    fn read(&mut self, tag: u64, pbf: &mut Protobuf);
}

pub trait ProtoWrite {
    fn write(&self, pbf: &mut Protobuf);
}

#[derive(Default)]
pub struct Protobuf {
    buf: RefCell<Vec<u8>>,
    pos: usize,
}
impl Protobuf {
    pub fn new() -> Protobuf {
        let buf = RefCell::new(Vec::new());
        Protobuf { buf, pos: 0 }
    }

    pub fn from_input(buf: RefCell<Vec<u8>>) -> Protobuf {
        Protobuf { buf, pos: 0 }
    }

    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    // === READING =================================================================

    fn decode_varint(&mut self) -> u64 {
        if self.pos >= self.buf.borrow().len() {
            unreachable!();
        }

        let mut val: u64 = 0;
        let buf = self.buf.borrow();

        for (n, shift) in BIT_SHIFT.iter().enumerate().take(MAX_VARINT_LENGTH) {
            let b = buf[self.pos] as u64;
            self.pos += 1;
            if n == 0 {
                if b & 0x80 == 0 {
                    return b;
                }
                val = b & 0x7f;
            } else {
                val |= (b & 0x7f) << shift;
            }
            if b < 0x80 {
                break;
            }
        }

        val
    }

    pub fn skip(&mut self, t: Type) {
        match t {
            Type::Varint => _ = self.decode_varint(),
            Type::Fixed64 => self.pos += 8,
            Type::Fixed32 => self.pos += 4,
            Type::Bytes => self.pos += self.decode_varint() as usize,
        };
    }

    pub fn read_field(&mut self) -> Field {
        let val = self.decode_varint();
        Field {
            tag: val >> 3,
            r#type: Type::from(val as u8 & 0x7),
        }
    }

    pub fn read_bytes(&mut self) -> Vec<u8> {
        let end = self.decode_varint() as usize + self.pos;
        let buf = self.buf.borrow();
        let bytes = buf[self.pos..end].to_vec();
        self.pos += end - self.pos;

        bytes
    }

    pub fn read_string(&mut self) -> String {
        String::from_utf8(self.read_bytes()).expect("Invalid UTF-8")
    }

    pub fn read_fixed<T>(&mut self) -> T
    where
        T: BitCast,
    {
        let buf = self.buf.borrow();
        let mut val: u64 = 0;
        let size = size_of::<T>();

        let mut n = 0;
        while n < size {
            val |= (buf[self.pos] as u64) << (n << 3);
            self.pos += 1;
            n += 1;
        }

        if cfg!(target_endian = "big") {
            val = val.swap_bytes();
        }

        T::from_u64(val)
    }

    pub fn read_varint<T>(&mut self) -> T
    where
        T: BitCast,
    {
        let val = self.decode_varint();
        T::from_u64(val)
    }

    pub fn read_s_varint<T>(&mut self) -> T
    where
        T: TryFrom<i64>,
    {
        T::try_from(zagzig(self.decode_varint()))
            .unwrap_or_else(|_| panic!("read_s_varint: Invalid conversion"))
    }

    pub fn read_packed<T>(&mut self) -> Vec<T>
    where
        T: BitCast,
    {
        let end = self.decode_varint() as usize + self.pos;
        let mut res: Vec<T> = Vec::new();
        while self.pos < end {
            let val = self.decode_varint();
            res.push(T::from_u64(val));
        }

        res
    }

    pub fn read_s_packed<T>(&mut self) -> Vec<T>
    where
        T: TryFrom<i64>,
    {
        let end = self.decode_varint() as usize + self.pos;
        let mut res: Vec<T> = Vec::new();
        while self.pos < end {
            res.push(self.read_s_varint::<T>());
        }

        res
    }

    /// If the length of the struct or enum is already known
    /// you use this function over "read_message"
    /// Top level parsing uses read_fields, whereas
    /// when you come across a nested message, you use read_message
    pub fn read_fields<T: ProtoRead>(&mut self, t: &mut T) {
        let end = self.buf.borrow().len();

        while self.pos < end {
            let field = self.read_field();
            let start_pos = self.pos;
            t.read(field.tag, self);
            if start_pos == self.pos {
                self.skip(field.r#type);
            }
        }
    }

    pub fn read_message<T: ProtoRead>(&mut self, t: &mut T) {
        let end = self.decode_varint() as usize + self.pos;

        while self.pos < end {
            let field = self.read_field();
            let start_pos = self.pos;
            t.read(field.tag, self);
            if start_pos == self.pos {
                self.skip(field.r#type);
            }
        }
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

    fn write_fixed<T>(&mut self, val: T)
    where
        T: BitCast,
    {
        let size = size_of::<T>();
        let mut val: u64 = val.to_u64();
        let mut buf = self.buf.borrow_mut();

        if cfg!(target_endian = "big") {
            val = val.swap_bytes();
        }

        let mut n = 0;
        while n < size {
            buf.push((val >> (n << 3)) as u8);
            n += 1;
        }
    }

    fn write_field(&mut self, tag: u64, r#type: Type) {
        let b: u64 = (tag << 3) | Into::<u64>::into(r#type);
        self.write_varint(b);
    }

    fn write_length_varint(&mut self, tag: u64, val: usize) {
        self.write_field(tag, Type::Bytes);
        self.write_varint(val as u64);
    }

    pub fn write_varint_field<T>(&mut self, tag: u64, val: T)
    where
        T: BitCast,
    {
        self.write_field(tag, Type::Varint);
        self.write_varint(val.to_u64());
    }

    pub fn write_s_varint_field<T>(&mut self, tag: u64, val: T)
    where
        T: Into<i64>,
    {
        self.write_field(tag, Type::Varint);
        self.write_s_varint(val.into());
    }

    pub fn write_packed_varint<T>(&mut self, tag: u64, val: &[T])
    where
        T: BitCast + Copy,
    {
        let mut bytes: Vec<u8> = Vec::new();
        for &v in val {
            Protobuf::write_varint_to_buffer(&mut bytes, v.to_u64());
        }

        self.write_length_varint(tag, bytes.len());
        let mut buf = self.buf.borrow_mut();
        buf.append(&mut bytes.to_owned());
    }

    pub fn write_packed_s_varint<T>(&mut self, tag: u64, val: &[T])
    where
        T: Into<i64> + Copy,
    {
        let mut bytes: Vec<u8> = Vec::new();
        for &v in val {
            Protobuf::write_varint_to_buffer(&mut bytes, zigzag(v.into()));
        }

        self.write_length_varint(tag, bytes.len());
        let mut buf = self.buf.borrow_mut();
        buf.append(&mut bytes.to_owned());
    }

    /// Supports only 32 and 64 bit types
    pub fn write_fixed_field<T>(&mut self, tag: u64, val: T)
    where
        T: BitCast + Copy,
    {
        let r#type = match size_of::<T>() {
            4 => Type::Fixed32,
            8 => Type::Fixed64,
            _ => panic!("Invalid fixed type"),
        };
        self.write_field(tag, r#type);
        self.write_fixed(val);
    }

    pub fn write_string_field(&mut self, tag: u64, val: &str) {
        self.write_length_varint(tag, val.len());
        let mut buf = self.buf.borrow_mut();
        buf.extend_from_slice(val.as_bytes());
    }

    pub fn write_bytes_field(&mut self, tag: u64, val: &[u8]) {
        self.write_length_varint(tag, val.len());
        let mut buf = self.buf.borrow_mut();
        buf.extend_from_slice(val)
    }

    pub fn write_message<T: ProtoWrite>(&mut self, tag: u64, t: &T) {
        let mut pbf = Protobuf::new();
        t.write(&mut pbf);
        let bytes = pbf.take();
        self.write_length_varint(tag, bytes.len());
        let mut buf = self.buf.borrow_mut();
        buf.extend_from_slice(&bytes);
    }

    /// When done writing to the buffer, call this function to take ownership
    pub fn take(&mut self) -> Vec<u8> {
        self.buf.take()
    }
}

pub fn zigzag(val: i64) -> u64 {
    ((val << 1) ^ (val >> 63)) as u64
}

pub fn zagzig(val: u64) -> i64 {
    (val >> 1) as i64 ^ -((val & 1) as i64)
}

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let buf = vec![];
        let pb = Protobuf::from_input(RefCell::new(buf));
        assert_eq!(pb.pos, 0);
    }

    #[test]
    fn test_zigzag() {
        assert_eq!(zigzag(0), 0);
        assert_eq!(zagzig(0), 0);
        assert_eq!(zagzig(zigzag(0)), 0);
        assert_eq!(zagzig(zigzag(5)), 5);
        assert_eq!(zagzig(zigzag(-5)), -5);

        let max_i64 = i64::MAX;
        let min_i64 = i64::MIN;
        assert_eq!(zagzig(zigzag(max_i64)), max_i64);
        assert_eq!(zagzig(zigzag(min_i64)), min_i64);
    }

    #[test]
    #[should_panic(expected = "Invalid value for Type")]
    fn test_write_field_panic() {
        let _t: Type = Type::from(22);
    }

    #[test]
    fn test_varint() {
        let mut pb = Protobuf::new();
        pb.write_varint(1);
        pb.write_varint(300);
        pb.write_varint(0x7fffffffffffffff);

        let bytes = pb.take();
        assert_eq!(
            bytes,
            &[1, 172, 2, 255, 255, 255, 255, 255, 255, 255, 255, 127]
        );

        let mut pb = Protobuf::from_input(RefCell::new(bytes));
        assert_eq!(pb.read_varint::<u64>(), 1);
        assert_eq!(pb.read_varint::<u64>(), 300);
        assert_eq!(pb.read_varint::<u64>(), 0x7fffffffffffffff);
    }

    #[test]
    fn test_varint_field() {
        let mut pb = Protobuf::new();
        // unsigned
        pb.write_varint_field(0, 5_u8);
        pb.write_varint_field(1, 1_u16);
        pb.write_varint_field(2, 300_u32);
        pb.write_varint_field(3, 0x7fffffffffffffff_u64);
        // signed
        pb.write_varint_field(4, -5_i8);
        pb.write_varint_field(5, -1_i16);
        pb.write_varint_field(6, -300_i32);
        pb.write_varint_field(7, -94949494949_i64);
        // bool
        pb.write_varint_field(8, true);
        pb.write_varint_field(9, false);
        // enum
        #[derive(Debug, PartialEq)]
        enum TestEnum {
            A = 1,
            B = 2,
            C = 3,
        }
        impl BitCast for TestEnum {
            fn from_u64(val: u64) -> Self {
                match val {
                    1 => TestEnum::A,
                    2 => TestEnum::B,
                    3 => TestEnum::C,
                    _ => panic!("Invalid enum value"),
                }
            }
            fn to_u64(&self) -> u64 {
                match self {
                    TestEnum::A => 1,
                    TestEnum::B => 2,
                    TestEnum::C => 3,
                }
            }
        }
        pb.write_varint_field(10, TestEnum::B);
        // float
        pb.write_varint_field(11, std::f32::consts::PI);
        pb.write_varint_field(12, -std::f64::consts::PI);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        // unsigned
        // tag 0
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 0,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<u8>(), 5);
        // tag 1
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<u16>(), 1);
        // tag 2
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<u32>(), 300);
        // tag 3
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 3,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<u64>(), 0x7fffffffffffffff);

        // signed
        // tag 4
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 4,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<i8>(), -5);
        // tag 5
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 5,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<i16>(), -1);
        // tag 6
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 6,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<i32>(), -300);
        // tag 7
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 7,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<i64>(), -94949494949);

        // bool
        // tag 8
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 8,
                r#type: Type::Varint
            }
        );
        assert!(pb.read_varint::<bool>());
        // tag 9
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 9,
                r#type: Type::Varint
            }
        );
        assert!(!pb.read_varint::<bool>());

        // enum
        // tag 10
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 10,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<TestEnum>(), TestEnum::B);

        // float
        // tag 11
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 11,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<f32>(), std::f32::consts::PI);
        // tag 12
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 12,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<f64>(), -std::f64::consts::PI);
    }

    #[test]
    fn test_s_varint_field() {
        let mut pb = Protobuf::new();
        pb.write_s_varint_field(1, 5_i8);
        pb.write_s_varint_field(2, 5_i16);
        pb.write_s_varint_field(3, 5_i32);
        pb.write_s_varint_field(4, 5_i64);
        pb.write_s_varint_field(5, -5_i8);
        pb.write_s_varint_field(6, -5_i16);
        pb.write_s_varint_field(7, -5_i32);
        pb.write_s_varint_field(8, -5_i64);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i8>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i16>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 3,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i32>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 4,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i64>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 5,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i8>(), -5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 6,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i16>(), -5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 7,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i32>(), -5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 8,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_s_varint::<i64>(), -5);
    }

    #[test]
    #[should_panic(expected = "Invalid fixed type")]
    fn test_fixed_panic() {
        let mut pb = Protobuf::new();
        pb.write_fixed_field(1, 1_u8);
    }

    #[test]
    fn test_fixed() {
        let mut pb = Protobuf::new();
        pb.write_fixed_field(1, 5_u32);
        pb.write_fixed_field(2, -5_i32);
        pb.write_fixed_field(3, 5.5_f32);
        pb.write_fixed_field(4, 5_u64);
        pb.write_fixed_field(5, -5_i64);
        pb.write_fixed_field(6, 5.5_f64);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Fixed32
            }
        );
        assert_eq!(pb.read_fixed::<u32>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Fixed32
            }
        );
        assert_eq!(pb.read_fixed::<i32>(), -5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 3,
                r#type: Type::Fixed32
            }
        );
        assert_eq!(pb.read_fixed::<f32>(), 5.5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 4,
                r#type: Type::Fixed64
            }
        );
        assert_eq!(pb.read_fixed::<u64>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 5,
                r#type: Type::Fixed64
            }
        );
        assert_eq!(pb.read_fixed::<i64>(), -5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 6,
                r#type: Type::Fixed64
            }
        );
        assert_eq!(pb.read_fixed::<f64>(), 5.5);
    }

    #[test]
    fn test_string() {
        let mut pb = Protobuf::new();
        pb.write_string_field(1, "hello");
        pb.write_string_field(2, "world");

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_string(), "hello");
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_string(), "world");
    }

    #[test]
    fn test_bytes() {
        let mut pb = Protobuf::new();
        pb.write_bytes_field(1, &[1, 2, 3]);
        pb.write_bytes_field(2, &[4, 5, 6]);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_bytes(), &[1, 2, 3]);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_bytes(), &[4, 5, 6]);
    }

    #[test]
    fn test_set_pos() {
        let mut pb = Protobuf::new();
        pb.write_varint_field(1, 5);
        pb.write_varint_field(2, 5);
        pb.write_varint_field(3, 5);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        pb.set_pos(2);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<u8>(), 5);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 3,
                r#type: Type::Varint
            }
        );
        assert_eq!(pb.read_varint::<u8>(), 5);
    }

    #[test]
    fn test_skip() {
        let mut pb = Protobuf::new();
        pb.write_varint_field(1, 5_u8);
        pb.write_fixed_field(2, -5_i32);
        pb.write_fixed_field(3, 5.5_f64);
        pb.write_packed_varint::<u16>(4, &[1, 2, 3, 4, 5]);
        pb.write_varint_field(5, false);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        let mut field = pb.read_field();
        pb.skip(field.r#type); // skip 1 Type::Varint
        field = pb.read_field();
        pb.skip(field.r#type); // skip 2 Type::Fixed32
        field = pb.read_field();
        pb.skip(field.r#type); // skip 3 Type::Fixed64
        field = pb.read_field();
        pb.skip(field.r#type); // skip 4 Type::Bytes
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 5,
                r#type: Type::Varint
            }
        );
        assert!(!pb.read_varint::<bool>());
    }

    #[test]
    fn test_packed_and_s_packed() {
        let mut pb = Protobuf::new();
        pb.write_packed_varint::<u16>(1, &[1, 2, 3]);
        pb.write_packed_varint::<f32>(2, &[4.4, 5.5, 6.6]);
        pb.write_packed_s_varint(3, &[-1, -2, -3]);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_packed::<u16>(), vec![1, 2, 3]);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_packed::<f32>(), vec![4.4, 5.5, 6.6]);
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 3,
                r#type: Type::Bytes
            }
        );
        assert_eq!(pb.read_s_packed::<i32>(), vec![-1, -2, -3]);
    }

    #[test]
    fn test_message() {
        #[derive(Default)]
        struct TestMessage {
            a: i32,
            b: String,
        }
        impl TestMessage {
            fn new(a: i32, b: &str) -> Self {
                TestMessage { a, b: b.to_owned() }
            }
        }
        impl ProtoWrite for TestMessage {
            fn write(&self, pb: &mut Protobuf) {
                pb.write_varint_field::<u64>(1, self.a as u64);
                pb.write_string_field(2, &self.b);
            }
        }
        impl ProtoRead for TestMessage {
            fn read(&mut self, tag: u64, pb: &mut Protobuf) {
                println!("tag: {}", tag);
                match tag {
                    1 => self.a = pb.read_varint::<i32>(),
                    2 => self.b = pb.read_string(),
                    _ => panic!("Invalid tag"),
                }
            }
        }

        let mut pb = Protobuf::new();
        let msg = TestMessage::new(1, "hello");
        pb.write_message(1, &msg);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        // first read in the field for the message
        let field = pb.read_field();
        assert_eq!(
            field,
            Field {
                tag: 1,
                r#type: Type::Bytes
            }
        );

        let mut msg = TestMessage::default();
        pb.read_message(&mut msg);
        assert_eq!(msg.a, 1);
        assert_eq!(msg.b, "hello");
    }
}
