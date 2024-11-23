use memchr;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Result, Seek};

// Skips r to right after [[ and returns the position at which this tag is found
fn skip_to_opening_tag<R: BufRead + Seek>(r: &mut R) -> Result<u64> {
    let mut stage_two = false;
    loop {
        let (on_chr, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            match memchr::memchr(b'[', available) {
                Some(i) => (true, i + 1),
                None if available.len() == 0 => {
                    return Err(Error::from(ErrorKind::UnexpectedEof));
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

pub fn next_wikilink<R: BufRead + Seek>(r: &mut R) -> Option<(u64, String)> {
    let idx = skip_to_opening_tag(r).ok()?;
    let mut buf = vec![];
    loop {
        r.read_until(b']', &mut buf).ok()?;
        let read = r.read_until(b']', &mut buf).ok()?;
        if read == 1 {
            break;
        }
    }
    buf.pop();
    buf.pop();
    let mut link = String::from_utf8(buf).ok()?;
    if let Some(idx) = link.find(|c| c == '|' || c == '#') {
        link.truncate(idx)
    }
    Some((idx, link.trim_ascii().to_string()))
}

#[derive(Debug)]
struct WikilinksIter<B> {
    buf: B,
}

impl<B: BufRead + Seek> Iterator for WikilinksIter<B> {
    type Item = (u64, String);

    fn next(&mut self) -> Option<Self::Item> {
        next_wikilink(&mut self.buf)
    }
}

pub fn read_wikilinks<R: Read + Seek>(reader: R) -> impl Iterator<Item = (u64, String)> {
    let buf = BufReader::with_capacity(64 * 1024, reader);
    WikilinksIter { buf }
}
