use std::{io::Write, mem::replace, str::from_utf8};

use ngram_iter::WORD_JOINER;

pub struct LatexWrite<T: Write> {
    inner: T,
    last_chars: [char; 3],
}

impl<T: Write> LatexWrite<T> {
    fn process_trigram(&mut self, trigram: [char; 3]) -> std::io::Result<usize> {
        let res = match trigram {
            [WORD_JOINER, _, _] | [' ', ' ', _] => Ok(1),
            [' ', '\n', _] => {
                self.inner.write(b"\n")?;
                Ok(2)
            }
            ['$', _, _] => {
                self.inner.write(b"\\$")?;
                Ok(1)
            }
            ['%', _, _] => {
                self.inner.write(b"\\%")?;
                Ok(1)
            }
            [' ', '–' | '-' | '—', ' ' | ','] => {
                self.inner.write(b" ---")?;
                Ok(2)
            }
            [',', '–' | '-' | '—', ' '] => {
                self.inner.write(b",---")?;
                Ok(2)
            }
            [d @ '0'..='9', '–' | '-' | '—', '0'..='9'] => {
                let mut buf = [0];
                self.inner.write(d.encode_utf8(&mut buf).as_bytes())?;
                self.inner.write(b"--")?;
                Ok(2)
            }
            [c, _, _] => {
                let mut buf = [0; 4];
                self.inner.write(c.encode_utf8(&mut buf).as_bytes())?;
                Ok(1)
            }
        };

        self.inner.flush()?;

        return res;
    }
}

impl<T: Write> Write for LatexWrite<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let chars = match from_utf8(buf) {
            Ok(s) => s.chars(),
            Err(err) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, err)),
        };

        let chars = self
            .last_chars
            .clone()
            .into_iter()
            .filter(|c| *c != WORD_JOINER)
            .chain(chars);

        let mut chars = ngram_iter::Iter::from(chars);

        loop {
            match chars.next() {
                None => break,
                Some(trigram) => match trigram {
                    [_, _, WORD_JOINER] => {
                        self.last_chars = trigram;
                        break;
                    }
                    trigram @ _ => match self.process_trigram(trigram) {
                        Ok(s) => {
                            for _ in 1..s {
                                chars.next();
                            }
                        }
                        Err(err) => return Err(err),
                    },
                },
            }
        }

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let last_chars = replace(
            &mut self.last_chars,
            [WORD_JOINER, WORD_JOINER, WORD_JOINER],
        )
        .into_iter()
        .filter(|c| *c != WORD_JOINER);

        let mut chars = ngram_iter::Iter::from(last_chars);

        loop {
            match chars.next() {
                None => break,
                Some(trigram) => match self.process_trigram(trigram) {
                    Ok(s) => {
                        for _ in 1..s {
                            chars.next();
                        }
                    }
                    Err(err) => return Err(err),
                },
            }
        }
        Ok(())
    }
}

impl<T> From<T> for LatexWrite<T>
where
    T: Write,
{
    #[inline]
    fn from(value: T) -> Self {
        LatexWrite {
            inner: value,
            last_chars: [WORD_JOINER, WORD_JOINER, WORD_JOINER],
        }
    }
}

impl<T: Write> Drop for LatexWrite<T> {
    fn drop(&mut self) {
        self.flush().ok();
    }
}

#[cfg(test)]
mod test {
    use std::str::from_utf8;

    use super::LatexWrite;

    #[test]
    fn test_simple_text() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "123456").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "123456");
    }

    #[test]
    fn test_escape_dollar() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "R$ 10,00").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "R\\$ 10,00");
    }

    #[test]
    fn test_escape_percent() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "5%").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "5\\%");
    }

    #[test]
    fn test_escape_em_dash() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "foo - bar").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "foo --- bar");
    }

    #[test]
    fn test_escape_em_dash_2() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "foo — bar").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "foo --- bar");
    }

    #[test]
    fn test_escape_em_dash_3() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "foo –, bar").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "foo ---, bar");
    }

    #[test]
    fn test_escape_en_dash() {
        use std::io::Write;
        let mut vec = Vec::new();
        {
            let mut write = LatexWrite::from(&mut vec);
            write!(&mut write, "10-90").unwrap();
        }
        assert_eq!(from_utf8(&vec).unwrap(), "10--90");
    }
}
