// our own imports
mod input;
use input::Input;

// standard library
use std::{
    env,
    io::{self, BufRead, Write, BufWriter},
    thread,
    time::{Duration, Instant},
};

// crates
use clap::Parser;
use format_bytes::write_bytes;

const DROP_THRESHOLD: Duration = Duration::from_millis(100);
const FAIL_THRESHOLD: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Parser)]
#[command(version, name = env!("CARGO_PKG_NAME"), long_about = None)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    input: String,
}

/// Finds the next start of frame marker and parses the timecode
fn read_timecode(r: &mut Input, buf: &mut Vec<u8>) -> io::Result<u64> {
    buf.truncate(0);
    match r.read_until(0x16, buf) {
        Ok(size) => {
            if size == 0 { return Ok(0); }
            buf.truncate(0);
            r.read_until(0x74/*t*/, buf).unwrap();
            Ok(String::from_utf8_lossy(&buf)
                      .trim_end_matches('t')
                      .to_string()
                      .parse::<u64>()
                      .unwrap())
        },
        Err(e) => Err(e),
    }
}

fn main() {
    let cli = Cli::parse();

    let mut r = Input::path(&cli.input).expect("open failed: {:?}");
    let mut w = BufWriter::with_capacity(1<<17, std::io::stdout());
    let mut buf = Vec::<u8>::with_capacity(1<<20); // 1MiB buffer

    let mut frame_count = 0usize;
    let mut dropped = 0usize;

    let start = Instant::now();

    loop {
        let timecode = read_timecode(&mut r, &mut buf).unwrap();
        // probably not a valid file...
        if frame_count == 0 && timecode != 0 { panic!(); }

        let skip = timecode > 0 && {
            let then = start + Duration::from_millis(timecode);
            let now = Instant::now();
            if then > now {
                // wait for next frame
                thread::sleep(then - now);
                false
            } else if then < now - DROP_THRESHOLD {
                dropped += 1;
                true
            } else if then < now - FAIL_THRESHOLD {
                w.write_all(b"\x1b[KYour connection is too slow to play this, sorry.\n").unwrap();
                break;
            } else {
                false
            }
        };

        buf.truncate(0);
        let size = match r.read_until(0x17, &mut buf) {
            Ok(size) => {
                if size == 0 {
                    break;
                }
                let size = size - 1; // trim ETB
                buf.truncate(if skip { 0 } else { size });
                size
            },
            Err(e) => {
                eprintln!("Error: {:?}", e);
                break;
            }
        };

        // timestamp components
        let t_hh = timecode / (3600 * 1000);
        let t_mm = (timecode / (60 * 1000)) % 60;
        let t_ss = (timecode / 1000) % 60;
        let t_ms = timecode % 1000;

        // status line
        let status = format!(
            "  [{:6}][{:6}b][{:02}:{:02}:{:02}.{:03}]",
            frame_count, size,
            t_hh, t_mm, t_ss, t_ms,
        );

        // add status line to buffer
        buf.write_all(status.as_bytes()).unwrap();
        if dropped > 0 {
            write_bytes!(&mut buf, b" Dropped frames: {}", dropped).unwrap();
        }
        buf.write_all(b"\x1b[G").unwrap();

        // output buffer
        w.write_all(&buf).unwrap();

        // flush
        let _ = w.flush();

        // increment frame count
        frame_count += 1;
    }

    w.write_all(b"\n").unwrap();
}
