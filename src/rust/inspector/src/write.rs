//! Physical FIT fields and local message definitions, preserving developer identity.
use crate::wire::{self, Field};
use std::io;

pub struct Cell {
    pub def: Field,
    pub data: Vec<u8>,
}
pub fn native(id: u8, base: u8, data: Vec<u8>) -> Cell {
    Cell {
        def: Field {
            id,
            base: Some(base),
            size: data.len() as u8,
            developer: None,
        },
        data,
    }
}
pub fn int_bytes(x: u16, big: bool) -> Vec<u8> {
    if big {
        x.to_be_bytes().to_vec()
    } else {
        x.to_le_bytes().to_vec()
    }
}
pub fn set(cells: &mut Vec<Cell>, new: Cell) -> io::Result<()> {
    if let Some(f) = cells
        .iter_mut()
        .find(|x| x.def.id == new.def.id && x.def.developer == new.def.developer)
    {
        if f.def.base != new.def.base {
            return Err(wire::invalid("unsupported width/type of selected field"));
        }
        *f = new;
    } else if new.def.developer.is_none() {
        let i = cells
            .iter()
            .position(|f| f.def.developer.is_some())
            .unwrap_or(cells.len());
        cells.insert(i, new);
    } else {
        cells.push(new);
    }
    Ok(())
}
pub fn definition(local: usize, n: u16, big: bool, cells: &[Cell]) -> io::Result<Vec<u8>> {
    let k = cells
        .iter()
        .take_while(|x| x.def.developer.is_none())
        .count();
    let d = cells.len() - k;
    let count =
        |v| u8::try_from(v).map_err(|_| wire::invalid("too many fields in a FIT definition"));
    let mut buf = vec![
        0x40 | (if d > 0 { 0x20 } else { 0 }) | local as u8,
        0,
        u8::from(big),
    ];
    buf.extend(int_bytes(n, big));
    buf.push(count(k)?);
    for f in &cells[..k] {
        buf.extend([f.def.id, f.def.size, f.def.base.unwrap()]);
    }
    if d > 0 {
        buf.push(count(d)?);
        for f in &cells[k..] {
            buf.extend([f.def.id, f.def.size, f.def.developer.unwrap()]);
        }
    }
    Ok(buf)
}

/// Publish an exact original, reusing only an identical regular file.
pub fn copy_original(bytes: &[u8], dest: &std::path::Path) -> std::io::Result<&'static str> {
    use std::{
        fs,
        io::{self, ErrorKind, Write},
        path::Path,
    };
    let directory = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(directory)?;
    let existing = || -> io::Result<&'static str> {
        if dest.symlink_metadata()?.is_file() && fs::read(dest)? == bytes {
            Ok("reused")
        } else {
            Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "destination already exists with different contents or file type",
            ))
        }
    };
    match existing() {
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        result => return result,
    }
    // Publish a complete sibling file exactly once, including competing exporters.
    let mut staged = tempfile::NamedTempFile::new_in(directory)?;
    staged.write_all(bytes)?;
    staged.as_file().sync_all()?;
    match staged.persist_noclobber(dest) {
        Ok(_) => Ok("exported"),
        Err(e) if e.error.kind() == ErrorKind::AlreadyExists => existing(),
        Err(e) => Err(e.error),
    }
}
