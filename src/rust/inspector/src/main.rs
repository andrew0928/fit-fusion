mod inventory;
mod roundtrip;
mod sdk;
use fit_inspector::wire;

use std::{
    env, fs,
    io::{self, Write},
    path::Path,
    process::ExitCode,
};
const HELP: &str = "Inspect a FIT recording offline. Run from a terminal:
  fit-inspector inspect FILE.fit [--decoder rustyfit|wire]   Show fields, coverage and timing (JSON)
  fit-inspector raw FILE.fit                                Show wire frames (JSON Lines)
  fit-inspector roundtrip FILE.fit --out NEW.fit            Re-encode and verify a new FIT
  fit-inspector --skill <index|inspect|raw|roundtrip>        Show agent usage and retry guidance";

fn run(args: &[String]) -> io::Result<i32> {
    let args: Vec<_> = args.iter().map(String::as_str).collect();
    let (cmd, file, out) = match args.as_slice() {
        ["--help" | "-h"] | ["inspect" | "raw" | "roundtrip", "--help"] => {
            println!("{HELP}");
            return Ok(0);
        }
        ["--skill", topic] => {
            let guide = match *topic {
                "index" => HELP,
                "inspect" => {
                    "fit-inspector inspect FILE.fit — read one FIT offline. stdout is JSON with members, messages, field coverage and timing. On a nonzero exit discard the report and inspect stderr."
                }
                "raw" => {
                    "fit-inspector raw FILE.fit — each stdout line is a JSON frame with offset and hex. Concatenate hex only after exit 0; a corrupt FIT can produce partial stdout."
                }
                "roundtrip" => {
                    "fit-inspector roundtrip FILE.fit --out NEW.fit — requires a new path in an existing directory. Decode and encode with RustyFIT, then verify messages and CRC. Exit 0 returns a JSON receipt; compare semantic_equal, byte_equal and changed_string_widths. On failure inspect stderr and choose a fresh path for retry."
                }
                _ => {
                    eprintln!("{HELP}");
                    return Ok(2);
                }
            };
            println!("{guide}");
            return Ok(0);
        }
        [cmd @ ("inspect" | "raw"), file] => (*cmd, *file, None),
        ["inspect", file, "--decoder", "rustyfit"] => ("inspect", *file, None),
        ["inspect", file, "--decoder", "wire"] => ("inspect-wire", *file, None),
        ["roundtrip", file, "--out", out] => ("roundtrip", *file, Some(*out)),
        _ => {
            eprintln!("{HELP}");
            return Ok(2);
        }
    };
    let execute = || -> io::Result<()> {
        let data = fs::read(file)?;
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        match cmd {
            "raw" => wire::walk(&data, |frame| {
                serde_json::to_writer(&mut stdout, &frame.raw())?;
                writeln!(stdout)
            })?,
            "inspect" | "inspect-wire" => {
                serde_json::to_writer(
                    &mut stdout,
                    &inventory::inspect(&data, file, cmd == "inspect")?,
                )?;
                writeln!(stdout)?;
            }
            _ => {
                serde_json::to_writer(
                    &mut stdout,
                    &roundtrip::export(&data, Path::new(out.unwrap()))?,
                )?;
                writeln!(stdout)?;
            }
        }
        stdout.flush()
    };
    execute().map_err(|e| io::Error::new(e.kind(), format!("{file}: {e}")))?;
    Ok(0)
}
fn main() -> ExitCode {
    match run(&env::args().skip(1).collect::<Vec<_>>()) {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("fit-inspector: {e}");
            ExitCode::FAILURE
        }
    }
}
