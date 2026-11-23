//! Actual SDK observations for the inventory. Physical widths and offsets are
//! supplied by wire definitions, since RustyFIT returns logical field values.
use crate::wire::{Layout, float, invalid};
use rustyfit::{
    Decoder,
    proto::{Message, Value as FitValue},
};
use serde_json::{Value, json};
use std::io::Result;

pub fn messages(member: &[u8]) -> Result<std::vec::IntoIter<Message>> {
    let mut decoder = Decoder::builder().expand_components(false).build();
    let fit = decoder
        .decode(member)
        .map_err(|e| invalid(format!("RustyFIT decode: {e:?}")))?
        .ok_or_else(|| invalid("RustyFIT found no member"))?;
    Ok(fit.messages.into_iter())
}

fn value(v: &FitValue) -> Value {
    use FitValue::*;
    match v {
        Invalid => Value::Null,
        Int8(n) => json!(n),
        Uint8(n) => json!(n),
        Int16(n) => json!(n),
        Uint16(n) => json!(n),
        Int32(n) => json!(n),
        Uint32(n) => json!(n),
        Int64(n) => json!(n),
        Uint64(n) => json!(n),
        String(s) => json!(s),
        Float32(n) => float(*n as f64),
        Float64(n) => float(*n),
        ArrayInt8(a) => json!(a),
        ArrayUint8(a) => json!(a),
        ArrayInt16(a) => json!(a),
        ArrayUint16(a) => json!(a),
        ArrayInt32(a) => json!(a),
        ArrayUint32(a) => json!(a),
        ArrayInt64(a) => json!(a),
        ArrayUint64(a) => json!(a),
        ArrayString(a) => json!(a),
        ArrayFloat32(a) => Value::Array(a.iter().map(|&n| float(n as f64)).collect()),
        ArrayFloat64(a) => Value::Array(a.iter().copied().map(float).collect()),
    }
}

pub fn observations(
    message: &Message,
    layout: &Layout,
    compressed: bool,
) -> Result<(Option<i64>, Vec<Value>)> {
    if message.num.0 != layout.number {
        return Err(invalid("SDK message number differs from wire definition"));
    }
    let mut native = message.fields.iter();
    let mut dev = message.developer_fields.iter();
    let mut timestamp = None;
    if compressed {
        let f = native
            .next()
            .filter(|f| f.num == 253)
            .ok_or_else(|| invalid("SDK compressed timestamp missing"))?;
        timestamp = value(&f.value).as_i64();
    }
    let mut values = Vec::with_capacity(layout.fields.len());
    for field in &layout.fields {
        let v = if let Some(index) = field.developer {
            dev.next()
                .filter(|f| f.num == field.id && f.developer_data_index == index)
                .map(|f| &f.value)
        } else {
            native
                .next()
                .filter(|f| f.num == field.id && Some(f.base_type.0) == field.base)
                .map(|f| &f.value)
        }
        .ok_or_else(|| {
            invalid(format!(
                "RustyFIT field missing or differently typed: message {}, field {}, developer {:?}",
                layout.number, field.id, field.developer
            ))
        })?;
        let v = value(v);
        if !compressed && field.developer.is_none() && field.id == 253 {
            timestamp = v.as_i64();
        }
        values.push(v);
    }
    if native.next().is_some() || dev.next().is_some() {
        return Err(invalid("SDK has additional fields"));
    }
    Ok((timestamp, values))
}
