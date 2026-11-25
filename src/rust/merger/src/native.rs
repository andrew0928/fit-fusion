//! Read the selected native values at each activity timestamp.
use crate::{Error, Result};
pub(crate) use fit_inspector::native::{NAMES, NativeValue, record_id};
use fit_inspector::wire::{self, Record};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Records = BTreeMap<u32, BTreeMap<u8, NativeValue>>;

pub(crate) fn read_records<'a>(
    bytes: &[u8],
    mut times: impl Iterator<Item = &'a u32>,
    fields: &BTreeSet<u8>,
) -> Result<Records> {
    let mut records = Records::new();
    wire::walk(bytes, |frame| {
        if let Record::Data { layout, values, .. } = frame.record
            && layout.number == 20
        {
            let time = *times
                .next()
                .ok_or_else(|| wire::invalid("unexpected wire record"))?;
            let mut values_at_time = BTreeMap::new();
            for (f, raw) in layout.fields.iter().zip(values) {
                if let Some(base) = f.base
                    && f.developer.is_none()
                    && fields.contains(&f.id)
                {
                    let value = NativeValue::read(base, raw, layout.big)?;
                    if value.valid() {
                        values_at_time.insert(f.id, value);
                    }
                }
            }
            records.insert(time, values_at_time);
        }
        Ok(())
    })
    .map_err(|e| Error::data(e.to_string()))?;
    if times.next().is_some() {
        return Err(Error::data("missing wire records"));
    }
    Ok(records)
}
