#[cfg(test)]
mod tests {
    extern crate alloc;

    use pbf_core::Protobuf;
    use pbf_derive::{BitCast, ProtoRead, ProtoWrite};

    #[test]
    fn test_bit_cast_macro() {
        use pbf_core::BitCast;

        #[derive(Debug, PartialEq, BitCast)]
        enum TestEnum {
            A = 3,
            B = 1,
            C = 2,
        }

        let a = TestEnum::A;
        let a_num: u64 = a.to_u64();
        assert_eq!(a_num, 3);

        let a_back = TestEnum::from_u64(3);
        assert_eq!(a, a_back);

        let b = TestEnum::B;
        let b_num: u64 = b.to_u64();
        assert_eq!(b_num, 1);

        let b_back = TestEnum::from_u64(1);
        assert_eq!(b, b_back);

        let c = TestEnum::C;
        let c_num: u64 = c.to_u64();
        assert_eq!(c_num, 2);

        let c_back = TestEnum::from_u64(2);
        assert_eq!(c, c_back);
    }

    #[test]
    fn test_proto_write_struct_macro() {
        #[derive(Debug, Default, PartialEq, Copy, Clone, BitCast)]
        enum TestEnum {
            #[default]
            A = 3,
            B = 1,
            C = 2,
        }

        #[derive(Debug, Default, PartialEq, ProtoRead, ProtoWrite)]
        struct NestedStruct {
            a: usize,
            b: String,
        }

        #[derive(Debug, Default, PartialEq, ProtoRead, ProtoWrite)]
        struct TestStruct {
            a: String,
            #[pbf(tag = 10, signed)]
            b: i32,
            #[pbf(ignore)]
            c: bool,
            d: Vec<u8>,
            e: TestEnum,
            #[pbf(tag = 20, fixed)]
            f: u32,
            #[pbf(nested)]
            g: NestedStruct,
            h: Option<f64>,
            i: bool,
            #[pbf(nested)]
            j: Option<NestedStruct>,
            k: Option<String>,
            l: Option<Vec<u8>>,
            m: Option<bool>,
            #[pbf(signed)]
            n: Option<Vec<i32>>,
        }

        let a: TestStruct = TestStruct { k: Some("hello".into()), ..Default::default() };
        let mut pb = Protobuf::new();
        pb.write_fields(&a);

        let bytes = pb.take();
        assert_eq!(
            bytes,
            vec![
                2, 0, 80, 0, 90, 0, 96, 3, 165, 1, 0, 0, 0, 0, 170, 1, 4, 0, 0, 10, 0, 184, 1, 0,
                202, 1, 5, 104, 101, 108, 108, 111
            ]
        );

        let mut pb = Protobuf::from_input(bytes);
        let mut b: TestStruct = Default::default();
        pb.read_fields(&mut b, None);
        assert_eq!(a, b);
    }

    #[test]
    fn test_proto_write_enum_macro() {
        #[derive(Debug, Copy, Clone, PartialEq, PartialOrd, BitCast)]
        enum TestEnum {
            A = 3,
            B = 1,
            C = 2,
        }

        #[derive(Debug, Clone, Default, PartialEq, ProtoRead, ProtoWrite)]
        struct NestedStruct {
            a: usize,
            b: String,
        }

        #[derive(Debug, Default, PartialEq, ProtoRead, ProtoWrite)]
        pub enum Value {
            /// String value
            String(String),
            /// Unsigned integer value
            UInt(u64),
            /// Signed integer 64-bit value
            #[pbf(signed)]
            SInt(i64),
            /// 32-bit Floating point value
            Float(f32),
            /// 64-bit Floating point value
            #[pbf(fixed)]
            Double(f64),
            /// Boolean value
            #[pbf(tag = 12)]
            Bool(bool),
            /// Option case
            Option(Option<i64>),
            /// Value case
            Enum(TestEnum),
            /// Nested struct
            #[pbf(nested)]
            Nested(NestedStruct),
            /// Null value
            #[default]
            Null,
        }

        let a = Value::String("test".to_string());
        let mut pb = Protobuf::new();
        pb.write_fields(&a);

        let bytes = pb.take();
        assert_eq!(bytes, vec![2, 4, 116, 101, 115, 116]);

        let mut pb = Protobuf::from_input(bytes);
        let mut b: Value = Default::default();
        pb.read_fields(&mut b, None);
        assert_eq!(a, b);
    }
}
