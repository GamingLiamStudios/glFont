#[derive(Debug)]
pub enum ValidType {
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U24(u32),
    U32(u32),
    I32(i32),

    // 32bit Fixed Point - 16bit fractional
    F16d16(fixed::types::I16F16),

    // 16bit Fixed Point - 14bit fractional
    F2d14(fixed::types::I2F14),

    // Time since 12.00 Jan 1st 1904, UTC
    Ldt(i64),

    Tag([u8; 4]),

    // Packed version - 16 major, 16 minor
    PVer(u32),

    _USize(usize),
}

impl core::fmt::Display for ValidType {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::U8(v) => write!(f, "{v}"),
            Self::I8(v) => write!(f, "{v}"),
            Self::U16(v) => write!(f, "{v}"),
            Self::I16(v) => write!(f, "{v}"),
            Self::U24(v) | Self::U32(v) => write!(f, "{v}"),
            Self::I32(v) => write!(f, "{v}"),
            Self::F16d16(v) => write!(f, "{v}"),
            Self::F2d14(v) => write!(f, "{v}"),
            Self::Ldt(v) => {
                const UNIX_DIFF: i64 = 2_082_888_000; // Difference in Seconds between EPOCH and UNIX_EPOCH
                let datetime =
                    chrono::DateTime::from_timestamp(*v - UNIX_DIFF, 0).expect("Invalid Timestamp");
                write!(f, "{datetime}")
            },
            Self::Tag(v) => {
                for c in v {
                    write!(f, "{}", *c as char)?;
                }
                Ok(())
            },
            Self::PVer(_) => unimplemented!(),
            Self::_USize(v) => write!(f, "{v}"),
        }
    }
}
