use memchr;
use std::{io, string};

// Skips r to right after [[ and returns the position at which this tag is found
fn skip_to_opening_tag<R: io::BufRead + io::Seek>(r: &mut R) -> io::Result<u64> {
    let mut stage_two = false;
    loop {
        let (on_chr, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            match memchr::memchr(b'[', available) {
                Some(i) => (true, i + 1),
                None if available.len() == 0 => {
                    return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
                }
                None => (false, available.len()),
            }
        };
        r.consume(used);
        if stage_two && used == 1 {
            return r.stream_position().map(|p| p - 2);
        }
        stage_two = on_chr;
    }
}

// Skips r to right after ]] and returns the bytes read upto this tag
pub fn read_to_closing_tag<R: io::BufRead>(r: &mut R) -> io::Result<Vec<u8>> {
    let mut buf = vec![];
    r.read_until(b']', &mut buf)?;
    loop {
        let read = r.read_until(b']', &mut buf)?;
        if read == 1 {
            break;
        } else if read == 0 {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof));
        }
    }
    buf.pop();
    buf.pop();
    Ok(buf)
}

fn extract_link_target(mut buf: Vec<u8>) -> Result<String, string::FromUtf8Error> {
    if let Some(idx) = memchr::memchr2(b'|', b'#', &buf) {
        buf.truncate(idx)
    }
    String::from_utf8(buf.trim_ascii().to_vec())
}

pub fn next_wikilink<R: io::BufRead + io::Seek>(r: &mut R) -> Option<(u64, String)> {
    // Keep going until we find a link which is not internal, or an error occurs
    loop {
        let idx = skip_to_opening_tag(r).ok()?;
        let buf = read_to_closing_tag(r).ok()?;
        let tgt = extract_link_target(buf).ok()?;
        if !tgt.is_empty() {
            // Not an internal link, done
            return Some((idx, tgt));
        }
    }
}

#[derive(Debug)]
struct WikilinksIter<B> {
    buf: B,
}

impl<B: io::BufRead + io::Seek> Iterator for WikilinksIter<B> {
    type Item = (u64, String);

    fn next(&mut self) -> Option<Self::Item> {
        next_wikilink(&mut self.buf)
    }
}

pub fn read_wikilinks<R: io::Read + io::Seek>(reader: R) -> impl Iterator<Item = (u64, String)> {
    let buf = io::BufReader::with_capacity(64 * 1024, reader);
    WikilinksIter { buf }
}
