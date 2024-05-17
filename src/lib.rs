#![no_std]
#![deny(missing_docs)]
//! The `pbf` Rust crate provides functionalities to read and write Protocol Buffers (protobuf) messages.
//! This crate is a 0 dependency package that uses `no_std` and is intended to be used in
//! embedded systems and WASM applications. The crate is designed to be small and efficient,
//! with the cost of some features and flexibility. It is up to the user to create the necessary
//! data structures and implement the `ProtoRead` and `ProtoWrite` traits in order to use it effectively.

/// All encoding and decoding is done via u64.
/// So all types must implement this trait to be able to be encoded and decoded.
pub mod bit_cast;

extern crate alloc;

use alloc::{borrow::ToOwned, string::String, vec::Vec};
use bit_cast::BitCast;
use core::{cell::RefCell, mem::size_of};

const MAX_VARINT_LENGTH: usize = u64::BITS as usize * 8 / 7 + 1;
const BIT_SHIFT: [u64; 10] = [0, 7, 14, 21, 28, 35, 42, 49, 56, 63];

/// The `Type` enum represents the different types that a field can have in a protobuf message.
/// The `Type` enum is used to determine how to encode and decode the field.
#[derive(Debug, PartialEq)]
pub enum Type {
    /// Varint may be: int32, int64, uint32, uint64, sint32, sint64, bool, enum
    Varint = 0,
    /// Fixed 64-bit numbers will take up exactly 64 bits of space
    /// They may be u64, i64, or f64
    Fixed64 = 1,
    /// This includes any len-delimited tyles:
    /// string, bytes, embedded messages, packed repeated fields
    Bytes = 2,
    /// Fixed 32-bit numbers will take up exactly 64 bits of space
    /// They may be an u32, i32, or f32
    Fixed32 = 5,
    /// This is a null type
    None = 7,
}
impl From<u8> for Type {
    /// Convert a u8 to a Type
    /// # Panics
    /// If the value is not a valid Type
    fn from(val: u8) -> Self {
        match val & 0x7 {
            0 => Type::Varint,
            1 => Type::Fixed64,
            2 => Type::Bytes,
            5 => Type::Fixed32,
            7 => Type::None,
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
            Type::None => 7,
        }
    }
}

/// The `Field` struct contains a tag and a type.
/// The tag is used to track the data type in the message for decoding.
/// The type is used to determine how to encode and decode the field.
#[derive(Debug, PartialEq)]
pub struct Field {
    /// The tag is used to track the data type in the message for decoding.
    pub tag: u64,
    /// The type is used to determine how to encode and decode the field.
    pub r#type: Type,
}

/// The `ProtoRead` trait is used to read a protobuf **message**.
/// This crate forces the user to implement this trait in order to read a protobuf message.
///
/// # Example
/// Using OSM File Format [BlobHeader](https://github.com/openstreetmap/OSM-binary/blob/65e7e976f5c8e47f057a0d921639ea8e6309ef06/osmpbf/fileformat.proto#L63) as an example:
/// ```proto
/// message BlobHeader {
///     required string type = 1;
///     optional bytes indexdata = 2;
///     required int32 datasize = 3;
/// }
/// ```
/// The user would implement the `ProtoRead` trait for the `BlobHeader` struct.
/// ```
/// use pbf::{ProtoRead, Protobuf};
///
/// struct BlobHeader {
///     r#type: String,
///     indexdata: Vec<u8>,
///     datasize: i32,
/// }
/// impl ProtoRead for BlobHeader {
///    fn read(&mut self, tag: u64, pbf: &mut Protobuf) {
///       match tag {
///          1 => self.r#type = pbf.read_string(),
///          2 => self.indexdata = pbf.read_bytes(),
///          3 => self.datasize = pbf.read_varint::<i32>(),
///          _ => unreachable!(),
///       }
///    }
/// }
/// ```
pub trait ProtoRead {
    /// The `read` method is used to read a field from a protobuf message.
    /// The `tag` parameter is used to determine which field to read into the struct.
    /// The `pbf` parameter is used to read the data in the appropriate format.
    ///
    /// # Example
    /// Using OSM File Format [BlobHeader](https://github.com/openstreetmap/OSM-binary/blob/65e7e976f5c8e47f057a0d921639ea8e6309ef06/osmpbf/fileformat.proto#L63) as an example:
    /// ```proto
    /// message BlobHeader {
    ///     required string type = 1;
    ///     optional bytes indexdata = 2;
    ///     required int32 datasize = 3;
    /// }
    /// ```
    /// An example **read** implementation for a `BlobHeader` struct:
    /// ```
    /// use pbf::{ProtoRead, Protobuf};
    ///
    /// struct BlobHeader {
    ///     r#type: String,
    ///     indexdata: Vec<u8>,
    ///     datasize: i32,
    /// }
    /// impl ProtoRead for BlobHeader {
    ///    fn read(&mut self, tag: u64, pbf: &mut Protobuf) {
    ///       match tag {
    ///          1 => self.r#type = pbf.read_string(),
    ///          2 => self.indexdata = pbf.read_bytes(),
    ///          3 => self.datasize = pbf.read_varint::<i32>(),
    ///          _ => unreachable!(),
    ///       }
    ///    }
    /// }
    /// ```
    fn read(&mut self, tag: u64, pbf: &mut Protobuf);
}

/// The `ProtoWrite` trait is used to write a protobuf **message**.
/// This crate forces the user to implement this trait in order to write a protobuf message.
///
/// # Example
/// Using OSM File Format [BlobHeader](https://github.com/openstreetmap/OSM-binary/blob/65e7e976f5c8e47f057a0d921639ea8e6309ef06/osmpbf/fileformat.proto#L63) as an example:
/// ```proto
/// message BlobHeader {
///     required string type = 1;
///     optional bytes indexdata = 2;
///     required int32 datasize = 3;
/// }
/// ```
/// The user would implement the `ProtoWrite` trait for the `BlobHeader` struct.
/// ```
/// use pbf::{ProtoWrite, Protobuf};
///
/// struct BlobHeader {
///    r#type: String,
///   indexdata: Vec<u8>,
///   datasize: i32,
/// }
/// impl ProtoWrite for BlobHeader {
///     fn write(&self, pbf: &mut Protobuf) {
///         pbf.write_string_field(1, &self.r#type);
///         pbf.write_bytes_field(2, &self.indexdata);
///         pbf.write_varint_field(3, self.datasize);
///     }
/// }
/// ```
pub trait ProtoWrite {
    /// The `write` method is used to write a field to a protobuf message.
    /// The `pbf` parameter is used to write the data in the appropriate format.
    ///
    /// # Example
    /// Using OSM File Format [BlobHeader](https://github.com/openstreetmap/OSM-binary/blob/65e7e976f5c8e47f057a0d921639ea8e6309ef06/osmpbf/fileformat.proto#L63) as an example:
    /// ```proto
    /// message BlobHeader {
    ///     required string type = 1;
    ///     optional bytes indexdata = 2;
    ///     required int32 datasize = 3;
    /// }
    /// ```
    /// An example **write** implementation for a `BlobHeader` struct:
    /// ```
    /// use pbf::{ProtoWrite, Protobuf};
    ///
    /// struct BlobHeader {
    ///    r#type: String,
    ///   indexdata: Vec<u8>,
    ///   datasize: i32,
    /// }
    /// impl ProtoWrite for BlobHeader {
    ///     fn write(&self, pbf: &mut Protobuf) {
    ///         pbf.write_string_field(1, &self.r#type);
    ///         pbf.write_bytes_field(2, &self.indexdata);
    ///         pbf.write_varint_field(3, self.datasize);
    ///     }
    /// }
    /// ```
    fn write(&self, pbf: &mut Protobuf);
}

/// The `Protobuf` struct is used to read and write protobuf messages.
///
/// # Example
/// Create a new Protobuf instance:
/// ```
/// use pbf::Protobuf;
///
/// let mut pbf = Protobuf::new();
/// ```
/// Create a Protobuf instance from a byte buffer:
/// ```
/// use pbf::Protobuf;
/// use std::cell::RefCell; // or use core::cell::RefCell; if sticking with no_std
///
/// let mut buf = vec![0x0A, 0x03, 0x74, 0x65, 0x73, 0x74];
/// let mut pbf = Protobuf::from_input(RefCell::new(buf));
/// ```
#[derive(Default)]
pub struct Protobuf {
    buf: RefCell<Vec<u8>>,
    pos: usize,
}
impl Protobuf {
    /// Create a new Protobuf instance.
    pub fn new() -> Protobuf {
        let buf = RefCell::new(Vec::new());
        Protobuf { buf, pos: 0 }
    }

    /// Create a Protobuf instance from a byte buffer.
    pub fn from_input(buf: RefCell<Vec<u8>>) -> Protobuf {
        Protobuf { buf, pos: 0 }
    }

    /// Set the position to read from the buffer next.
    pub fn set_pos(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// get the length of the bufer
    pub fn len(&self) -> usize {
        self.buf.borrow().len()
    }

    /// check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // === READING =================================================================

    /// Decode a varint from the buffer at the current position.
    pub fn decode_varint(&mut self) -> u64 {
        if self.pos >= self.len() {
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

    /// AFter reading a field, you can choose to skip it's value
    /// in the buffer if it is not needed.
    pub fn skip(&mut self, t: Type) {
        match t {
            Type::Varint => _ = self.decode_varint(),
            Type::Fixed64 => self.pos += 8,
            Type::Fixed32 => self.pos += 4,
            Type::Bytes => self.pos += self.decode_varint() as usize,
            Type::None => { /* Do nothing */ }
        };
    }

    /// Read a field from the buffer.
    pub fn read_field(&mut self) -> Field {
        let val = self.decode_varint();
        Field {
            tag: val >> 3,
            r#type: Type::from((val & 0x7) as u8),
        }
    }

    /// Read in bytes from the buffer.
    pub fn read_bytes(&mut self) -> Vec<u8> {
        let end = self.decode_varint() as usize + self.pos;
        let buf = self.buf.borrow();
        let bytes = buf[self.pos..end].to_vec();
        self.pos += end - self.pos;

        bytes
    }

    /// Read in a string from the buffer.
    pub fn read_string(&mut self) -> String {
        String::from_utf8(self.read_bytes()).expect("Invalid UTF-8")
    }

    /// Read in a fixed size value from the buffer.
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

    /// Read in a variable size value from the buffer.
    pub fn read_varint<T>(&mut self) -> T
    where
        T: BitCast,
    {
        let val = self.decode_varint();
        T::from_u64(val)
    }

    /// Read in a signed variable size value from the buffer.
    ///
    /// # Panics
    /// Panics if the conversion from `i64` to `T` fails.
    pub fn read_s_varint<T>(&mut self) -> T
    where
        T: TryFrom<i64>,
    {
        T::try_from(zagzig(self.decode_varint()))
            .unwrap_or_else(|_| panic!("read_s_varint: Invalid conversion"))
    }

    /// Read in a packed value from the buffer.
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

    /// Read in a signed packed value from the buffer.
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

    /// Read a message from the buffer. This is the alternative to `read_message`
    /// which does the same thing but you may already know the size of the message.
    /// The other case is top level data may have fields but no message length.
    pub fn read_fields<T: ProtoRead>(&mut self, t: &mut T, end: Option<usize>) {
        let end = end.unwrap_or(self.len());

        while self.pos < end {
            let field = self.read_field();
            let start_pos = self.pos;
            t.read(field.tag, self);
            if start_pos == self.pos {
                self.skip(field.r#type);
            }
        }
    }

    /// Read in an entire message from the buffer.
    /// This is usually used to read in a struct or enum.
    pub fn read_message<T: ProtoRead>(&mut self, t: &mut T) {
        let end = self.decode_varint() as usize + self.pos;

        self.read_fields(t, Some(end));
    }

    // === WRITING =================================================================

    /// Write a u64 to the buffer.
    pub fn write_varint(&mut self, val: u64) {
        let mut buf = self.buf.borrow_mut();
        let mut val = val;

        while val > 0x80 {
            buf.push((val & 0x7f) as u8 | 0x80);
            val >>= 7;
        }
        buf.push(val as u8);
    }

    /// Use this if you want to use the varint encoding in your own buffer.
    pub fn write_varint_to_buffer(buf: &mut Vec<u8>, val: u64) {
        let mut val = val;

        while val > 0x80 {
            buf.push((val & 0x7f) as u8 | 0x80);
            val >>= 7;
        }
        buf.push(val as u8);
    }

    /// Write an i64 to the buffer.
    pub fn write_s_varint(&mut self, val: i64) {
        self.write_varint(zigzag(val));
    }

    /// Write a fixed size value to the buffer. This will not compress the value.
    pub fn write_fixed<T>(&mut self, val: T)
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

    /// write a field of "tag" and "type" to the buffer.
    pub fn write_field(&mut self, tag: u64, r#type: Type) {
        let b: u64 = (tag << 3) | Into::<u64>::into(r#type);
        self.write_varint(b);
    }

    /// write a tag with the size of the buffer to be appended to the internal buffer.
    pub fn write_length_varint(&mut self, tag: u64, val: usize) {
        self.write_field(tag, Type::Bytes);
        self.write_varint(val as u64);
    }

    /// write a variable sized number, bool, or enum into to the buffer.
    pub fn write_varint_field<T>(&mut self, tag: u64, val: T)
    where
        T: BitCast,
    {
        self.write_field(tag, Type::Varint);
        self.write_varint(val.to_u64());
    }

    /// write a signed variable sized number into to the buffer.
    pub fn write_s_varint_field<T>(&mut self, tag: u64, val: T)
    where
        T: Into<i64>,
    {
        self.write_field(tag, Type::Varint);
        self.write_s_varint(val.into());
    }

    /// write a vector packed variable sized number, bool, or enum into to the buffer.
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

    /// write a vector packed signed variable sized number into to the buffer.
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

    /// write a fixed sized number into to the buffer. No compression is done.
    /// Supports 32 and 64 bit numbers.
    ///
    /// # Panics
    /// Panics if the size of the type is not 32 or 64 bits.
    pub fn write_fixed_field<T>(&mut self, tag: u64, val: T)
    where
        T: BitCast + Copy,
    {
        let type_ = match size_of::<T>() {
            4 => Type::Fixed32,
            8 => Type::Fixed64,
            _ => panic!("Invalid fixed type"),
        };

        self.write_field(tag, type_);
        self.write_fixed(val);
    }

    /// write only the string to the buffer
    pub fn write_string(&mut self, val: &str) {
        self.write_varint(val.len() as u64);
        let mut buf = self.buf.borrow_mut();
        buf.extend_from_slice(val.as_bytes());
    }

    /// write a string into to the buffer.
    pub fn write_string_field(&mut self, tag: u64, val: &str) {
        self.write_length_varint(tag, val.len());
        let mut buf = self.buf.borrow_mut();
        buf.extend_from_slice(val.as_bytes());
    }

    /// write a byte array into to the buffer.
    pub fn write_bytes_field(&mut self, tag: u64, val: &[u8]) {
        self.write_length_varint(tag, val.len());
        let mut buf = self.buf.borrow_mut();
        buf.extend_from_slice(val)
    }

    /// write a message into to the buffer.
    /// The message must implement the ProtoWrite trait.
    /// This is usually reserved for structs and enums.
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

/// convert a signed integer to an unsigned integer using zigzag encoding.
pub fn zigzag(val: i64) -> u64 {
    ((val << 1) ^ (val >> 63)) as u64
}

/// convert an unsigned integer to a signed integer using zigzag decoding.
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
    #[should_panic(expected = "Invalid fixed type")]
    fn test_fixed_panic() {
        let mut pb = Protobuf::new();
        pb.write_fixed_field(1, 1_u8);
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
    fn test_write_field() {
        let mut pb = Protobuf::new();
        pb.write_field(1, Type::Varint);
        pb.write_field(2, Type::None);

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(
            pb.read_field(),
            Field {
                tag: 1,
                r#type: Type::Varint,
            }
        );
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 2,
                r#type: Type::None,
            }
        );
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
        pb.write_field(5, Type::None);
        pb.write_varint_field(6, false);

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
        field = pb.read_field();
        pb.skip(field.r#type); // skip 5 Type::None
        assert_eq!(
            pb.read_field(),
            Field {
                tag: 6,
                r#type: Type::Varint
            }
        );
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
        #[derive(Debug, PartialEq, Default)]
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

    #[test]
    fn test_message_with_skip() {
        #[derive(Debug, PartialEq, Default)]
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
                if tag == 2 { self.b = pb.read_string() }
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
        assert_eq!(msg.a, 0);
        assert_eq!(msg.b, "hello");
    }

    #[test]
    fn unicode_string() {
        let mut pb = Protobuf::new();
        pb.write_string("你好");

        let bytes = pb.take();
        let mut pb = Protobuf::from_input(RefCell::new(bytes));

        assert_eq!(pb.read_string(), "你好");
    }
}
