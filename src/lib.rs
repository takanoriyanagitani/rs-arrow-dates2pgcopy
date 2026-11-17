use std::io;

use io::Write;

use arrow_array::Array;
use arrow_array::ArrayRef;
use arrow_array::cast::AsArray;

use arrow_array::Date32Array;
use arrow_array::Int32Array;

pub const NULL_BYTES: [u8; 4] = u32::to_be_bytes(0xffff_ffff);

pub trait DateLike: Array {
    fn to_dword(&self, trusted_index: usize) -> [u8; 4];

    fn to_writer<W>(&self, trusted_index: usize, wtr: &mut W) -> Result<(), io::Error>
    where
        W: Write,
    {
        if self.is_null(trusted_index) {
            wtr.write_all(&NULL_BYTES)?;
            return Ok(());
        }

        let bytes: [u8; 4] = self.to_dword(trusted_index);

        // Buffer layout: 3 bytes padding/header, 1 byte for length (4), 4 bytes for value
        let mut buf: [u8; 8] = [0; 8];
        buf[3] = 4;
        buf[4..8].copy_from_slice(&bytes);

        wtr.write_all(&buf)
    }
}

/// pg date epoch - unix epoch(2000-01-01 - 1970-01-01)
pub const PG_DATE_OFFSET_FOR_UNIX_EPOCH: u16 = 10957;

impl DateLike for &Date32Array {
    fn to_dword(&self, trusted_index: usize) -> [u8; 4] {
        let days_since_unix_epoch: i32 = self.value(trusted_index);
        let pg_day: i32 =
            days_since_unix_epoch.saturating_sub(PG_DATE_OFFSET_FOR_UNIX_EPOCH.into());
        pg_day.to_be_bytes()
    }
}

impl DateLike for &Int32Array {
    fn to_dword(&self, trusted_index: usize) -> [u8; 4] {
        let days_since_unix_epoch: i32 = self.value(trusted_index);
        let pg_day: i32 =
            days_since_unix_epoch.saturating_sub(PG_DATE_OFFSET_FOR_UNIX_EPOCH.into());
        pg_day.to_be_bytes()
    }
}

pub fn ref2date32(r: &ArrayRef) -> Option<&Date32Array> {
    r.as_primitive_opt()
}

pub fn ref2i32(r: &ArrayRef) -> Option<&Int32Array> {
    r.as_primitive_opt()
}
