use core::mem::transmute;

/// All encoding and decoding is done via u64.
/// So all types must implement this trait to be able to be encoded and decoded.
pub trait BitCast: Sized {
    /// Convert the value to a u64.
    fn to_u64(&self) -> u64;
    /// Convert a u64 to the value.
    fn from_u64(value: u64) -> Self;
}
impl BitCast for u64 {
    fn to_u64(&self) -> u64 {
        *self
    }
    fn from_u64(value: u64) -> Self {
        value
    }
}
impl BitCast for i64 {
    fn to_u64(&self) -> u64 {
        unsafe { transmute::<i64, u64>(*self) }
    }
    fn from_u64(value: u64) -> Self {
        unsafe { transmute::<u64, i64>(value) }
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
impl BitCast for u32 {
    fn to_u64(&self) -> u64 {
        *self as u64
    }
    fn from_u64(value: u64) -> Self {
        value as u32
    }
}
impl BitCast for i32 {
    fn to_u64(&self) -> u64 {
        unsafe { transmute::<i32, u32>(*self) as u64 }
    }
    fn from_u64(value: u64) -> Self {
        unsafe { transmute::<u32, i32>(value as u32) }
    }
}
impl BitCast for f32 {
    fn to_u64(&self) -> u64 {
        (*self).to_bits() as u64
    }
    fn from_u64(value: u64) -> Self {
        f32::from_bits(value as u32)
    }
}
impl BitCast for u16 {
    fn to_u64(&self) -> u64 {
        *self as u64
    }
    fn from_u64(value: u64) -> Self {
        value as u16
    }
}
impl BitCast for i16 {
    fn to_u64(&self) -> u64 {
        unsafe { transmute::<i16, u16>(*self) as u64 }
    }
    fn from_u64(value: u64) -> Self {
        unsafe { transmute::<u16, i16>(value as u16) }
    }
}
impl BitCast for u8 {
    fn to_u64(&self) -> u64 {
        *self as u64
    }
    fn from_u64(value: u64) -> Self {
        value as u8
    }
}
impl BitCast for i8 {
    fn to_u64(&self) -> u64 {
        unsafe { transmute::<i8, u8>(*self) as u64 }
    }
    fn from_u64(value: u64) -> Self {
        unsafe { transmute::<u8, i8>(value as u8) }
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
