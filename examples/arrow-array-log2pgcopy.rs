use std::io;
use std::process::ExitCode;
use std::sync::Arc;

use io::Write;

use arrow_array::RecordBatch;

use arrow_array::ArrayRef;

use arrow_array::Date32Array;
use arrow_array::Int32Array;

use arrow_schema::DataType;
use arrow_schema::Field;
use arrow_schema::Schema;

use rs_arrow_dates2pgcopy::ref2date32;
use rs_arrow_dates2pgcopy::ref2i32;

use rs_arrow_dates2pgcopy::DateLike;

fn io_record_batch() -> impl FnMut() -> Result<RecordBatch, io::Error> {
    || {
        RecordBatch::try_new(
            Schema::new(vec![
                Field::new("id", DataType::Int32, false),
                Field::new("date", DataType::Date32, false),
            ])
            .into(),
            vec![
                Arc::new(Int32Array::from(vec![10957, 10958, 10959, 10956])),
                Arc::new(Date32Array::from(vec![1, 3, 5, 7])),
            ],
        )
        .map_err(io::Error::other)
    }
}

#[derive(Debug, PartialEq)]
enum DateArray<'a> {
    Ignore,
    Int32s(&'a Int32Array),
    Dates(&'a Date32Array),
}

impl<'a> DateArray<'a> {
    fn to_writer<W>(&self, trusted_index: usize, wtr: &mut W) -> Result<(), io::Error>
    where
        W: Write,
    {
        match self {
            Self::Int32s(l) => l.to_writer(trusted_index, wtr),
            Self::Dates(l) => l.to_writer(trusted_index, wtr),
            _ => Ok(()),
        }
    }
}

fn io_bat2stdout(rbat: RecordBatch) -> impl FnOnce() -> Result<(), io::Error> {
    move || {
        let rowcnt: usize = rbat.num_rows();
        let cols: &[ArrayRef] = rbat.columns();

        let mapd: Vec<DateArray> = cols
            .iter()
            .map(|aref| {
                if let Some(i) = ref2i32(aref) {
                    return DateArray::Int32s(i);
                }

                if let Some(i) = ref2date32(aref) {
                    return DateArray::Dates(i);
                }

                DateArray::Ignore
            })
            .collect();

        let o = io::stdout();
        let mut ol = o.lock();

        let mut colsize: [u8; 2] = [0; 2];
        colsize[1] = mapd.len() as u8;

        for i in 0..rowcnt {
            ol.write_all(&colsize)?;
            for col in mapd.iter() {
                col.to_writer(i, &mut ol)?;
            }
        }
        Ok(())
    }
}

const PGCOPY_SIGN: &[u8] = b"PGCOPY\n\xff\r\n\0";
const PGCOPY_FLAGS: [u8; 4] = [0; 4];
const PGCOPY_EXT: [u8; 4] = [0; 4];

const PGCOPY_TRAILER: [u8; 2] = u16::to_be_bytes(0xffff);

fn io_header2stdout() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        let o = io::stdout();
        let mut ol = o.lock();
        let mut buf: [u8; 19] = [0; 19];
        let src = PGCOPY_SIGN.iter().chain(&PGCOPY_FLAGS).chain(&PGCOPY_EXT);
        let z = buf.iter_mut().zip(src);
        for pair in z {
            let (dst, src) = pair;
            *dst = *src;
        }
        ol.write_all(&buf)?;
        ol.flush()
    }
}

fn io_trailer2stdout() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        let o = io::stdout();
        let mut ol = o.lock();
        ol.write_all(&PGCOPY_TRAILER)?;
        ol.flush()
    }
}

fn io_main() -> impl FnOnce() -> Result<(), io::Error> {
    || {
        io_header2stdout()()?;
        let rbat: RecordBatch = io_record_batch()()?;
        io_bat2stdout(rbat)()?;
        io_trailer2stdout()()?;
        Ok(())
    }
}

fn sub() -> Result<(), io::Error> {
    io_main()()
}

fn main() -> ExitCode {
    sub().map(|_| ExitCode::SUCCESS).unwrap_or_else(|e| {
        eprintln!("{e}");
        ExitCode::FAILURE
    })
}
