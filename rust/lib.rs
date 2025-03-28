#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! The `pbf` Rust crate provides functionalities to read and write Protocol Buffers (protobuf) messages.
//! This crate is a 0 dependency package that uses `no_std` and is intended to be used in
//! embedded systems and WASM applications. The crate is designed to be small and efficient,
//! with the cost of some more modern features. It is up to the user to create the necessary
//! data structures and implement the `ProtoRead` and `ProtoWrite` traits in order to use it
//! more effectively beyond base cases.
//!
//! ## Usage
//!
//! ```rust
//! use pbf::{ProtoRead, ProtoWrite, Protobuf, Field, Type};
//!
//! #[derive(Default)]
//! struct TestMessage {
//!     a: i32,
//!     b: String,
//! }
//! impl TestMessage {
//!     fn new(a: i32, b: &str) -> Self {
//!         TestMessage { a, b: b.to_owned() }
//!     }
//! }
//! impl ProtoWrite for TestMessage {
//!     fn write(&self, pb: &mut Protobuf) {
//!         pb.write_varint_field::<u64>(1, self.a as u64);
//!         pb.write_string_field(2, &self.b);
//!     }
//! }
//! impl ProtoRead for TestMessage {
//!     fn read(&mut self, tag: u64, pb: &mut Protobuf) {
//!         println!("tag: {}", tag);
//!         match tag {
//!             1 => self.a = pb.read_varint::<i32>(),
//!             2 => self.b = pb.read_string(),
//!             _ => panic!("Invalid tag"),
//!         }
//!     }
//! }
//!
//! let mut pb = Protobuf::new();
//! let msg = TestMessage::new(1, "hello");
//! pb.write_fields(&msg);
//!
//! let bytes = pb.take();
//! let mut pb = Protobuf::from_input(bytes);
//!
//! let mut msg = TestMessage::default();
//! pb.read_fields(&mut msg, None);
//! assert_eq!(msg.a, 1);
//! assert_eq!(msg.b, "hello");
//! ```
//!
//! If you are using the `derive` feature, you can derive the `ProtoRead` and `ProtoWrite` traits for your struct.
//!
//! ```rust
//! use pbf::{ProtoRead, ProtoWrite, Protobuf, Field, Type};
//!
//! #[derive(ProtoRead, ProtoWrite, Default)]
//! struct TestMessage {
//!     a: i32,
//!     b: String,
//! }
//! ```
//!
//! `ProtoRead` and `ProtoWrite` trait derives support 5 attributes:
//!
//! - `pbf(tag = 1)` -> Set the tag # of the field.
//! - `pbf(fixed)` -> Set the type to `Fixed` (for 32-bit and 64-bit numbers).
//! - `pbf(signed)` -> Set the type to `Signed` (to handle protobuf "sint" values).
//! - `pbf(nested)` -> If a sub structure is present, set the type to `nested` so it can be properly derived.
//! - `pbf(ignore)` -> Ignore the field.
//!
//! Here is a more complex use case showcasing all the ways you can use derives:
//!
//! ```rust
//! use pbf::{self, BitCast, ProtoRead, ProtoWrite};
//!
//! #[derive(Debug, Default, Copy, Clone, PartialEq, BitCast)]
//! enum TestEnum {
//!     #[default]
//!     A = 1,
//!     B = 2, // tag increments from the previous value so it will be 2
//!     C = 3, // again, tag increments to 3
//! }
//!
//! #[derive(Debug, Default, PartialEq, ProtoRead, ProtoWrite)]
//! struct NestedStruct {
//!     a: usize,
//!     b: String,
//! }
//!
//! #[derive(Debug, Default, PartialEq, ProtoRead, ProtoWrite)]
//! struct TestStruct {
//!    #[pbf(tag = 10, signed)]
//!    a: i32,
//!    #[pbf(ignore)]
//!    b: bool,
//!    c: Vec<u8>,
//!    d: TestEnum,
//!    #[pbf(tag = 20, fixed)]
//!    e: u32,
//!    #[pbf(nested)]
//!    f: NestedStruct,
//!    g: Option<f64>,
//!    #[pbf(nested)]
//!    h: Option<NestedStruct>,
//!    #[pbf(signed)]
//!    i: Option<Vec<i32>>,
//! }
//!
//! #[derive(Debug, Default, PartialEq, ProtoRead, ProtoWrite)]
//! pub enum Value {
//!     /// String value
//!     String(String),
//!     /// Unsigned integer value
//!     UInt(u64),
//!     /// Signed integer 64-bit value
//!     #[pbf(signed)]
//!     SInt(i64),
//!     /// 64-bit Floating point value
//!     #[pbf(fixed)]
//!     Double(f64),
//!     /// Boolean value
//!     #[pbf(tag = 12)]
//!     Bool(bool),
//!     /// Option case
//!     Option(Option<i64>),
//!     /// Value case
//!     Enum(TestEnum),
//!     /// Nested struct
//!     #[pbf(nested)]
//!     Nested(NestedStruct),
//!     /// Null value
//!     #[default]
//!     Null,
//! }
//! ```
//!
//! If you are using the `derive` feature, you can also derive the `BitCast` trait for your enum.
//!
//! ```rust
//! use pbf::BitCast;
//!
//! #[derive(BitCast)]
//! enum TestEnum {
//!     A = 1,
//!     B = 2,
//!     C = 3,
//! }
//! ```

extern crate pbf_core;
#[cfg(feature = "derive")]
extern crate pbf_derive;

pub use pbf_core::*;
#[cfg(feature = "derive")]
pub use pbf_derive::*;
