use std::sync::Arc;

use chrono::NaiveDateTime;
use snafu::OptionExt;

use super::present::new_present_iter;
use super::{Column, GenericIterator, NullableIterator};
use crate::error::{self, Result};
use crate::proto::stream::Kind;
use crate::reader::decode::rle_v2::{SignedRleV2Iter, UnsignedRleV2Iter};

// TIMESTAMP_BASE is 1 January 2015, the base value for all timestamp values.
const TIMESTAMP_BASE: i64 = 1420070400;

pub struct TimestampIterator {
    data: Box<dyn Iterator<Item = Result<i64>>>,
    secondary: Box<dyn Iterator<Item = Result<u64>>>,
}

impl Iterator for TimestampIterator {
    type Item = Result<NaiveDateTime>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.data.next() {
            Some(Ok(data)) => match self.secondary.next() {
                Some(Ok(mut nanos)) => {
                    let zeros = nanos & 0x7;
                    nanos >>= 3;
                    if zeros != 0 {
                        for _ in 0..=zeros {
                            nanos *= 10;
                        }
                    }
                    let timestamp =
                        NaiveDateTime::from_timestamp_opt(data + TIMESTAMP_BASE, nanos as u32)
                            .context(error::InvalidTimestampSnafu);

                    Some(timestamp)
                }
                Some(Err(err)) => Some(Err(err)),
                None => None,
            },
            Some(Err(err)) => Some(Err(err)),
            None => None,
        }
    }
}

pub fn new_timestamp_iter(column: &Column) -> Result<GenericIterator<NaiveDateTime>> {
    let present = new_present_iter(column)?.try_collect::<Vec<_>>()?;
    let rows: usize = present.iter().filter(|&p| *p).count();

    let data = column
        .stream(Kind::Data)
        .transpose()?
        .map(|reader| Box::new(SignedRleV2Iter::new(reader, rows, vec![])))
        .context(error::InvalidColumnSnafu { name: &column.name })?;

    let secondary = column
        .stream(Kind::Secondary)
        .transpose()?
        .map(|reader| Box::new(UnsignedRleV2Iter::new(reader, rows, vec![])))
        .context(error::InvalidColumnSnafu { name: &column.name })?;

    Ok(NullableIterator {
        present: Box::new(present.into_iter()),
        iter: Box::new(TimestampIterator { data, secondary }),
    })
}
