// Compare the official SDK's view with encoded native fields before any scaling.
import { Decoder, Encoder, Profile, Stream } from "@garmin/fitsdk";
import { frames, json, validValue } from "./wire.mjs";

function decode(bytes, options) {
  const { errors } = new Decoder(Stream.fromBuffer(bytes)).read(options);
  if (errors.length) throw Error(String(errors[0]));
}
function* messages(bytes) {
  let definitions = new Map();
  for (const f of frames(bytes)) {
    if (f.kind === "header") definitions = new Map();
    if (f.kind === "definition") definitions.set(f.local, f.fields);
    if (f.kind === "data") {
      yield {
        id: f.global_id,
        offset: f.timestamp_offset ?? null,
        fields: definitions.get(f.local),
        values: f.values,
      };
    }
  }
}
function signature(m) {
  return json([
    m.id,
    m.offset,
    m.fields.map((f, i) => [f.developer_index ?? -1, f.id, f.base ?? null, m.values[i]])
      .sort((a, b) => a[0] - b[0] || a[1] - b[1]),
  ]);
}
export function sdkCheck(bytes) {
  const members = [];
  let start;
  for (const frame of frames(bytes)) {
    if (frame.kind === "header") start = frame.offset;
    if (frame.kind !== "crc") continue;
    const member = bytes.subarray(start, frame.offset + 2);
    const wire = [...messages(member)];
    let mismatch = null;
    let n = 0, checked = 0, decoderError = null;
    try {
      decode(member, {
        includeUnknownData: true,
        applyScaleAndOffset: false,
        convertTypesToStrings: false,
        convertDateTimesToDates: false,
        expandSubFields: false,
        expandComponents: false,
        mergeHeartRates: false,
        mesgListener(id, msg) {
          const expected = wire[n++];
          if (expected?.id !== id) {
            mismatch ??= { ordinal: n, message: id };
            return;
          }
          for (const [i, f] of expected.fields.entries()) {
            if (f.base === undefined) continue;
            const key = Profile.messages[id]?.fields[f.id]?.name ?? f.id;
            const val = validValue(expected.values[i], f.base);
            if (val === null) continue;
            const actual = msg[key];
            checked++;
            if (
              json(actual) !== json(val) &&
              !(Array.isArray(actual) && actual.length === 1 && json(actual[0]) === json(val)) && !mismatch
            ) {
              mismatch = { ordinal: n, message: id, field: f.id };
            }
          }
        },
      });
    } catch (e) {
      decoderError = e.message;
    }
    let equal = false, encoderError = null, firstDifference = null;
    let encodingOrdinal = 0, encodingMessage = null;
    try {
      const enc = new Encoder();
      decode(member, {
        includeUnknownData: true,
        fieldDescriptionListener: (...args) => enc.addDeveloperField(...args),
        mesgListener(id, msg) {
          encodingOrdinal++;
          encodingMessage = id;
          enc.onMesg(id, msg);
        },
      });
      const exported = [...messages(Buffer.from(enc.close()))];
      firstDifference = wire.findIndex((m, i) => !exported[i] || signature(m) !== signature(exported[i]));
      equal = firstDifference === -1 && wire.length === exported.length;
      firstDifference = equal ? null : (firstDifference === -1 ? wire.length : firstDifference) + 1;
    } catch (e) {
      encoderError = { ordinal: encodingOrdinal, message: encodingMessage, reason: e.message };
    }
    members.push({
      wire_messages: wire.length,
      sdk_messages: n,
      native_values_compared: checked,
      native_values_equal: !decoderError && !mismatch && n === wire.length,
      first_native_difference: mismatch,
      decode_error: decoderError,
      encode_field_values_equal: equal,
      first_encode_difference: firstDifference,
      encode_error: encoderError,
    });
  }
  return { backend: "@garmin/fitsdk 21.222.0", members };
}
