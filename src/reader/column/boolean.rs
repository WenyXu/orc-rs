use snafu::OptionExt;

use super::GenericIterator;
use crate::error::{InvalidColumnSnafu, Result};
use crate::proto::stream::Kind;
use crate::reader::column::present::new_present_iter;
use crate::reader::column::{Column, NullableIterator};
use crate::reader::decode::boolean_rle::BooleanIter;

pub fn new_boolean_iter(column: &Column) -> Result<GenericIterator<bool>> {
    let present = new_present_iter(column)?.try_collect::<Vec<_>>()?;
    let rows: usize = present.iter().filter(|&p| *p).count();

    let iter = column
        .stream(Kind::Data)
        .transpose()?
        .map(|reader| {
            Box::new(BooleanIter::new(reader, rows)) as Box<dyn Iterator<Item = Result<bool>>>
        })
        .context(InvalidColumnSnafu { name: &column.name })?;

    Ok(NullableIterator {
        present: Box::new(present.into_iter()),
        iter,
    })
}
