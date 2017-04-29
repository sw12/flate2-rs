use std::cmp;
use std::env;
use std::ffi::CString;
use std::io::prelude::*;
use std::io;
use std::mem;
use std::time;

#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};

use {Compression, Compress};
use bufreader::BufReader;
use crc::{CrcReader, Crc};
use deflate;
use zio;

use std::io::prelude::*;

use super::{EncoderWriter, EncoderReader, DecoderReader, Builder};
use Compression::Default;
use rand::{thread_rng, Rng};

#[test]
fn roundtrip() {
    let mut e = EncoderWriter::new(Vec::new(), Default);
    e.write_all(b"foo bar baz").unwrap();
    let inner = e.finish().unwrap();
    let mut d = DecoderReader::new(&inner[..]).unwrap();
    let mut s = String::new();
    d.read_to_string(&mut s).unwrap();
    assert_eq!(s, "foo bar baz");
}

#[test]
fn roundtrip_zero() {
    let e = EncoderWriter::new(Vec::new(), Default);
    let inner = e.finish().unwrap();
    let mut d = DecoderReader::new(&inner[..]).unwrap();
    let mut s = String::new();
    d.read_to_string(&mut s).unwrap();
    assert_eq!(s, "");
}

#[test]
fn roundtrip_big() {
    let mut real = Vec::new();
    let mut w = EncoderWriter::new(Vec::new(), Default);
    let v = thread_rng().gen_iter::<u8>().take(1024).collect::<Vec<_>>();
    for _ in 0..200 {
        let to_write = &v[..thread_rng().gen_range(0, v.len())];
        real.extend(to_write.iter().map(|x| *x));
        w.write_all(to_write).unwrap();
    }
    let result = w.finish().unwrap();
    let mut r = DecoderReader::new(&result[..]).unwrap();
    let mut v = Vec::new();
    r.read_to_end(&mut v).unwrap();
    assert!(v == real);
}

#[test]
fn roundtrip_big2() {
    let v = thread_rng()
                .gen_iter::<u8>()
                .take(1024 * 1024)
                .collect::<Vec<_>>();
    let mut r = DecoderReader::new(EncoderReader::new(&v[..], Default))
                    .unwrap();
    let mut res = Vec::new();
    r.read_to_end(&mut res).unwrap();
    assert!(res == v);
}

#[test]
fn fields() {
    let r = vec![0, 2, 4, 6];
    let e = Builder::new()
                .filename("foo.rs")
                .comment("bar")
                .extra(vec![0, 1, 2, 3])
                .read(&r[..], Default);
    let mut d = DecoderReader::new(e).unwrap();
    assert_eq!(d.header().filename(), Some(&b"foo.rs"[..]));
    assert_eq!(d.header().comment(), Some(&b"bar"[..]));
    assert_eq!(d.header().extra(), Some(&b"\x00\x01\x02\x03"[..]));
    let mut res = Vec::new();
    d.read_to_end(&mut res).unwrap();
    assert_eq!(res, vec![0, 2, 4, 6]);

}

#[test]
fn keep_reading_after_end() {
    let mut e = EncoderWriter::new(Vec::new(), Default);
    e.write_all(b"foo bar baz").unwrap();
    let inner = e.finish().unwrap();
    let mut d = DecoderReader::new(&inner[..]).unwrap();
    let mut s = String::new();
    d.read_to_string(&mut s).unwrap();
    assert_eq!(s, "foo bar baz");
    d.read_to_string(&mut s).unwrap();
    assert_eq!(s, "foo bar baz");
}

#[test]
fn qc_reader() {
    ::quickcheck::quickcheck(test as fn(_) -> _);

    fn test(v: Vec<u8>) -> bool {
        let r = EncoderReader::new(&v[..], Default);
        let mut r = DecoderReader::new(r).unwrap();
        let mut v2 = Vec::new();
        r.read_to_end(&mut v2).unwrap();
        v == v2
    }
}

#[test]
fn flush_after_write() {
    let mut f = EncoderWriter::new(Vec::new(), Default);
    write!(f, "Hello world").unwrap();
    f.flush().unwrap();
}
