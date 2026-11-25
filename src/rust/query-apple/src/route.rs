use crate::{
    Result, bad,
    health::{Metric, Observations, Workout, attr},
};
use jiff::Timestamp;
use quick_xml::{Reader, events::Event};
use std::{collections::BTreeMap, io::BufRead};

pub fn read<R: BufRead>(input: R, w: &Workout, out: &mut Observations) -> Result<()> {
    let mut r = Reader::from_reader(input);
    r.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut point: Option<BTreeMap<Metric, f64>> = None;
    let mut time: Option<Timestamp> = None;
    let mut field = Vec::new();
    let mut closed = false;
    loop {
        match r.read_event_into(&mut buf)? {
            Event::End(e) if e.local_name().as_ref() == b"gpx" => closed = true,
            Event::Empty(e) if e.local_name().as_ref() == b"gpx" => closed = true,
            Event::Start(e) if e.local_name().as_ref() == b"trkpt" => {
                let coord = |key: &str, limit: f64| -> Result<f64> {
                    let v: f64 = attr(&e, key)?
                        .ok_or_else(|| bad("GPX point missing coordinates"))?
                        .parse()?;
                    if !v.is_finite() || v.abs() > limit {
                        return Err(bad("GPX coordinate out of range"));
                    }
                    Ok(v)
                };
                point = Some(BTreeMap::from([
                    (Metric::Lat, coord("lat", 90.)?),
                    (Metric::Lon, coord("lon", 180.)?),
                ]));
                time = None;
            }
            Event::Start(e) if point.is_some() => field = e.local_name().into_inner().to_vec(),
            Event::Text(text) if point.is_some() => {
                let decoded = text.decode()?;
                let s = decoded.trim();
                let p = point.as_mut().unwrap();
                match field.as_slice() {
                    b"time" => time = Some(s.parse()?),
                    b"ele" => {
                        p.insert(Metric::Altitude, s.parse()?);
                    }
                    b"speed" => {
                        let v: f64 = s.parse()?;
                        if v >= 0. {
                            p.insert(Metric::Speed, v);
                        }
                    }
                    _ => (),
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"trkpt" => {
                let t = time.take().ok_or_else(|| bad("GPX point missing time"))?;
                let p = point.take().ok_or_else(|| bad("invalid GPX point"))?;
                if t >= w.start_utc && t <= w.end_utc {
                    out.add(t, p);
                    out.route_points += 1;
                }
                field.clear();
            }
            Event::End(_) => field.clear(),
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    if !closed || point.is_some() {
        return Err(bad("invalid GPX route"));
    }
    Ok(())
}
