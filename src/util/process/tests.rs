use std::collections::VecDeque;
use std::io;

use super::*;

#[test]
fn remove_prefix() {
    let mut vec = vec!['a', 'b', 'c'];
    let removed: Vec<_> = vec.remove_prefix(2);
    assert_eq!(removed, vec!['a', 'b']);
    assert_eq!(vec, vec!['c']);

    let mut vec = vec!['a', 'b', 'c'];
    let removed = vec.remove_prefix(0);
    assert!(removed.is_empty());
    assert_eq!(vec, vec!['a', 'b', 'c']);

    let mut vec = vec!['a', 'b', 'c'];
    let removed = vec.remove_prefix(3);
    assert_eq!(removed, vec!['a', 'b', 'c']);
    assert!(vec.is_empty());

    let mut vec = vec!['a', 'b', 'c'];
    let removed = vec.remove_prefix(10);
    assert_eq!(removed, vec!['a', 'b', 'c']);
    assert!(vec.is_empty());
}

struct ReadMock(VecDeque<io::Result<Vec<u8>>>);

impl ReadMock {
    fn new(data: impl IntoIterator<Item = io::Result<Vec<u8>>>) -> Self {
        Self(data.into_iter().collect())
    }
}

impl io::Read for ReadMock {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let next = match self.0.front_mut() {
            Some(Ok(next)) => next,
            Some(Err(_)) => return self.0.pop_front().unwrap().map(|_| 0),
            None => return Ok(0),
        };

        let size = buf.len().min(next.len());
        buf[..size].copy_from_slice(&next[..size]);
        if size == next.len() {
            self.0.pop_front();
        } else {
            next.remove_prefix(size);
        }

        Ok(size)
    }
}

#[test]
fn binary_lines_basic() {
    let read = ReadMock::new([
        Ok(vec![0xff; 5]),
        Ok(vec![0x0a]),
        Ok(vec![0xff, 0xff, 0xff, 0x0a]),
        Ok(vec![0xff; 128]),
        Ok(vec![0xff; 128]),
        Ok(vec![0x0a]),
        Ok(vec![0x0a]),
        Ok(vec![0xff]),
    ]);

    let mut lines = BinaryLines::new(read);

    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, vec![0xff, 0xff, 0xff, 0xff, 0xff, 0x0a]);

    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, vec![0xff, 0xff, 0xff, 0x0a]);

    let mut expected = vec![0xff; 256];
    expected.push(0xa);
    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, expected);

    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, vec![0xa]);

    let mut expected = vec![0xff];
    expected.extend_from_slice(LINE_END.as_bytes());
    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, expected);

    lines.next().ok_or(()).expect_err("EOF expected");
    lines.next().ok_or(()).expect_err("EOF expected");
    lines.next().ok_or(()).expect_err("EOF expected");
}

#[test]
fn binary_lines_multiple() {
    let read = ReadMock::new([Ok(vec![0x0a, 0xff, 0x0a, 0xff, 0x0a, 0xff, 0x0a])]);

    let mut lines = BinaryLines::new(read);

    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, vec![0x0a]);

    for _ in 0..3 {
        let line = lines.next().unwrap().unwrap();
        assert_eq!(line, vec![0xff, 0x0a]);
    }

    lines.next().ok_or(()).expect_err("EOF expected");
}

#[test]
fn binary_lines_large_lines() {
    const READ_SIZE: usize = BinaryLines::<ReadMock>::READ_SIZE;

    let read = ReadMock::new([
        Ok(vec![0xff; READ_SIZE]),
        Ok(vec![0xff; READ_SIZE]),
        Ok(vec![0xff; READ_SIZE]),
        Ok(vec![0x0a]),
        Ok(vec![0xff; READ_SIZE]),
        Ok(vec![0xff; READ_SIZE]),
        Ok(vec![0xff; READ_SIZE]),
        Ok(vec![0x0a]),
    ]);

    let mut lines = BinaryLines::new(read);

    for _ in 0..2 {
        let mut expected = vec![0xff; 3 * READ_SIZE];
        expected.push(0x0a);
        let line = lines.next().unwrap().unwrap();
        assert_eq!(line, expected);
    }

    lines.next().ok_or(()).expect_err("EOF expected");
}

#[test]
fn binary_lines_io_error() {
    let read = ReadMock::new([
        Ok(vec![0xff, 0x0a]),
        Ok(vec![0xff]),
        Err(io::Error::new(io::ErrorKind::Other, anyhow!("test"))),
        Ok(vec![0xff, 0x0a]),
    ]);

    let mut lines = BinaryLines::new(read);

    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, vec![0xff, 0x0a]);

    lines.next().unwrap().unwrap_err();

    let line = lines.next().unwrap().unwrap();
    assert_eq!(line, vec![0xff, 0xff, 0x0a]);
}
