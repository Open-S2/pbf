/// All encoding and decoding is done via u64.
/// So all types must implement this trait to be able to be encoded and decoded.
pub trait BitCast: Sized {
    /// Convert the value to a u64.
    fn to_u64(&self) -> u64;
    /// Convert a u64 to the value.
    fn from_u64(value: u64) -> Self;
}
macro_rules! impl_bitcast {
    ($($t:ty),*) => {
        $(
            impl BitCast for $t {
                fn to_u64(&self) -> u64 {
                    *self as u64
                }
                fn from_u64(value: u64) -> Self {
                    value as $t
                }
            }
        )*
    };
}
impl_bitcast!(u8, i8, u16, i16, u32, i32, u64, i64, usize, isize);
impl BitCast for f32 {
    fn to_u64(&self) -> u64 {
        (*self).to_bits() as u64
    }
    fn from_u64(value: u64) -> Self {
        f32::from_bits(value as u32)
    }
}
impl BitCast for f64 {
    fn to_u64(&self) -> u64 {
        (*self).to_bits()
    }
    fn from_u64(value: u64) -> Self {
        f64::from_bits(value)
    }
}
impl BitCast for bool {
    fn to_u64(&self) -> u64 {
        if *self {
            1
        } else {
            0
        }
    }
    fn from_u64(value: u64) -> Self {
        value != 0
    }
}
