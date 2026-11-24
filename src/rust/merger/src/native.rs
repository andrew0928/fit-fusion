//! Native FIT field identity and lossless record values used for same-field selection.
use crate::{Error, Result};
use fit_inspector::wire::{self, Record};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

pub(crate) static NAMES: LazyLock<BTreeMap<(u16, u8), String>> = LazyLock::new(|| {
    let mut names = BTreeMap::new();
    let mut message_name = "";
    for line in include_str!("../../../python/fit_fusion/profile.tsv").lines() {
        let mut columns = line.split('\t');
        let Some(message) = columns.next().and_then(|v| v.parse::<u16>().ok()) else {
            continue;
        };
        let Some(name) = columns.next() else { continue };
        if !name.is_empty() {
            message_name = name;
        }
        if let (Some(field), Some(name)) = (
            columns.next().and_then(|v| v.parse::<u8>().ok()),
            columns.next(),
        ) {
            names.insert((message, field), format!("{message_name}.{name}"));
        }
    }
    names
});

pub(crate) fn record_id(name: &str) -> Option<u8> {
    NAMES
        .range((20, 0)..=(20, 255))
        .find_map(|(&(_, id), n)| (n == name).then_some(id))
}

/// Canonical little-endian elements. Keep width, array elements, padding and floating point bits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeValue {
    pub base: u8,
    pub bytes: Vec<u8>,
}
impl NativeValue {
    fn convert_order(bytes: &mut [u8], base: u8, big: bool) {
        if big && base & 128 != 0 {
            for part in bytes.chunks_exact_mut(wire::width(base).unwrap()) {
                part.reverse();
            }
        }
    }
    pub fn read(base: u8, bytes: &[u8], big: bool) -> std::io::Result<Self> {
        let width =
            wire::width(base).ok_or_else(|| wire::invalid("unknown selected FIT base type"))?;
        if bytes.is_empty() || !bytes.len().is_multiple_of(width) {
            return Err(wire::invalid("invalid selected FIT field width"));
        }
        let mut bytes = bytes.to_vec();
        Self::convert_order(&mut bytes, base, big);
        Ok(Self { base, bytes })
    }
    pub fn encoded(&self, big: bool) -> Vec<u8> {
        let mut bytes = self.bytes.clone();
        Self::convert_order(&mut bytes, self.base, big);
        bytes
    }
    fn invalid_element(&self) -> u64 {
        let width = wire::width(self.base).unwrap();
        let mask = u64::MAX >> (64 - 8 * width);
        match self.base & 31 {
            1 | 3 | 5 | 14 => mask >> 1,
            7 | 10 | 11 | 12 | 16 => 0,
            _ => mask,
        }
    }
    pub fn valid(&self) -> bool {
        if self.base & 31 == 7 {
            return self.bytes[0] != 0;
        }
        let invalid = self.invalid_element();
        self.bytes
            .chunks_exact(wire::width(self.base).unwrap())
            .any(|element| wire::unsigned(element, false) != invalid)
    }
    pub fn invalidate(&mut self) {
        let width = wire::width(self.base).unwrap();
        let invalid = self.invalid_element().to_le_bytes();
        for part in self.bytes.chunks_exact_mut(width) {
            part.copy_from_slice(&invalid[..width]);
        }
    }
    pub fn decoded(&self) -> Value {
        wire::decoded(&self.bytes, Some(self.base), false)
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_values_preserve_zeros_array_elements_and_endianness() {
        let array = NativeValue::read(0x84, &[0xff, 0xff, 0, 0, 1, 0], true).unwrap();
        assert!(array.valid());
        assert_eq!(array.decoded(), serde_json::json!([65535, 0, 256]));
        assert_eq!(array.encoded(false), [255, 255, 0, 0, 0, 1]);
        for (base, bytes) in [
            (1, vec![127]),
            (0x90, vec![0; 8]),
            (0x88, vec![255; 4]),
            (7, vec![0, 65]),
        ] {
            assert!(!NativeValue::read(base, &bytes, false).unwrap().valid());
        }
        let mut float = NativeValue::read(0x88, &12.5_f32.to_be_bytes(), true).unwrap();
        assert_eq!(float.decoded(), serde_json::json!(12.5));
        float.invalidate();
        assert!(!float.valid());
    }
}
