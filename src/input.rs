use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

#[cfg(feature = "gunzip")]
use flate2::bufread::MultiGzDecoder;

#[cfg(feature = "unxz")]
use xz2::read::XzDecoder;

#[cfg(feature = "unzstd")]
use zstd::stream::read::Decoder as ZstdDecoder;

pub struct Input<'a> {
    pub label: String,
    inner: Box<dyn BufRead + 'a>,
}


//const BZIP2_MAGIC: [u8; 3] = *b"BZh";
//const LZ4_MAGIC:   [u8; 4] = [0x04, 0x22, 0x4d, 0x18];
//const LZO_MAGIC:   [u8; 9] = *b"\x89LZO\0\r\n\x1a\n";

#[cfg(feature = "gunzip")]
const GZIP_MAGIC:  [u8; 3] = [0x1f, 0x8b, 0x08];

#[cfg(feature = "unxz")]
const XZ_MAGIC:    [u8; 6] = *b"\xfd7zXZ\0";

#[cfg(feature = "unzstd")]
const ZSTD_MAGIC:  [u8; 4] = [0x28, 0xb5, 0x2f, 0xfd];

impl<'a> Input<'a> {
    // stdin -> reader
    #[allow(dead_code)]
    pub fn stdin() -> io::Result<Input<'a>> {
        Input::reader(io::stdin().lock(), "STDIN")
    }

    // path -> file -> reader
    pub fn path(path: impl AsRef<Path>) -> io::Result<Input<'a>> {
        let path: &Path = path.as_ref();
        let label = path.as_os_str().to_string_lossy();
        File::open(path).map(|file| Input::file(file, &label).unwrap())
    }

    // file -> reader
    pub fn file(file: File, label: &str) -> io::Result<Input<'a>> {
        Input::reader(BufReader::new(file), label)
    }

    // reader (always gets called)
    #[allow(unused_mut)]
    pub fn reader(mut reader: impl BufRead + 'a, label: &str) -> io::Result<Input<'a>> {
        #[cfg(any(feature = "gunzip", feature = "unxz", feature = "unzstd"))]
        let buf = reader.fill_buf()?;

        #[cfg(feature = "gunzip")]
        if buf.len() >= 3 && &buf[0..=2] == GZIP_MAGIC {
            let reader = MultiGzDecoder::new(reader);
            let reader = BufReader::new(reader);
            return Ok(Input {
                label: label.to_string(),
                inner: Box::new(reader)
            });
        }

        #[cfg(feature = "unxz")]
        if buf.len() >= 6 && &buf[0..=5] == XZ_MAGIC {
            let reader = XzDecoder::new(reader);
            let reader = BufReader::new(reader);
            return Ok(Input {
                label: label.to_string(),
                inner: Box::new(reader)
            });
        }


        #[cfg(feature = "unzstd")]
        if buf.len() >= 4 && &buf[0..=3] == ZSTD_MAGIC {
            let reader = ZstdDecoder::with_buffer(reader)?;
            let reader = BufReader::new(reader);
            return Ok(Input {
                label: label.to_string(),
                inner: Box::new(reader)
            });
        }

        Ok(Input { label: label.to_string(), inner: Box::new(reader) })
    }

    #[allow(dead_code)]
    pub fn as_label(&self) -> String {
        self.label.clone()
    }
}

impl<'a> Read for Input<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<'a> BufRead for Input<'a> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.inner.fill_buf()
    }

    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt);
    }
}
