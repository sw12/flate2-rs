use std::io::prelude::*;
use std::io;
use std::mem;

#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};

use bufreader::BufReader;
use zio;
use {Compress, Decompress};

use std::io::prelude::*;

use rand::{thread_rng, Rng};

use deflate::{EncoderWriter, EncoderReader, DecoderReader, DecoderWriter};
use Compression::Default;

#[test]
fn roundtrip() {
    let mut real = Vec::new();
    let mut w = EncoderWriter::new(Vec::new(), Default);
    let v = thread_rng().gen_iter::<u8>().take(1024).collect::<Vec<_>>();
    for _ in 0..200 {
        let to_write = &v[..thread_rng().gen_range(0, v.len())];
        real.extend(to_write.iter().map(|x| *x));
        w.write_all(to_write).unwrap();
    }
    let result = w.finish().unwrap();
    let mut r = DecoderReader::new(&result[..]);
    let mut ret = Vec::new();
    r.read_to_end(&mut ret).unwrap();
    assert!(ret == real);
}

#[test]
fn drop_writes() {
    let mut data = Vec::new();
    EncoderWriter::new(&mut data, Default).write_all(b"foo").unwrap();
    let mut r = DecoderReader::new(&data[..]);
    let mut ret = Vec::new();
    r.read_to_end(&mut ret).unwrap();
    assert!(ret == b"foo");
}

#[test]
fn total_in() {
    let mut real = Vec::new();
    let mut w = EncoderWriter::new(Vec::new(), Default);
    let v = thread_rng().gen_iter::<u8>().take(1024).collect::<Vec<_>>();
    for _ in 0..200 {
        let to_write = &v[..thread_rng().gen_range(0, v.len())];
        real.extend(to_write.iter().map(|x| *x));
        w.write_all(to_write).unwrap();
    }
    let mut result = w.finish().unwrap();

    let result_len = result.len();

    for _ in 0..200 {
        result.extend(v.iter().map(|x| *x));
    }

    let mut r = DecoderReader::new(&result[..]);
    let mut ret = Vec::new();
    r.read_to_end(&mut ret).unwrap();
    assert!(ret == real);
    assert_eq!(r.total_in(), result_len as u64);
}

#[test]
fn roundtrip2() {
    let v = thread_rng()
                .gen_iter::<u8>()
                .take(1024 * 1024)
                .collect::<Vec<_>>();
    let mut r = DecoderReader::new(EncoderReader::new(&v[..], Default));
    let mut ret = Vec::new();
    r.read_to_end(&mut ret).unwrap();
    assert_eq!(ret, v);
}

#[test]
fn roundtrip3() {
    let v = thread_rng()
                .gen_iter::<u8>()
                .take(1024 * 1024)
                .collect::<Vec<_>>();
    let mut w = EncoderWriter::new(DecoderWriter::new(Vec::new()), Default);
    w.write_all(&v).unwrap();
    let w = w.finish().unwrap().finish().unwrap();
    assert!(w == v);
}

#[test]
fn reset_writer() {
    let v = thread_rng()
                .gen_iter::<u8>()
                .take(1024 * 1024)
                .collect::<Vec<_>>();
    let mut w = EncoderWriter::new(Vec::new(), Default);
    w.write_all(&v).unwrap();
    let a = w.reset(Vec::new()).unwrap();
    w.write_all(&v).unwrap();
    let b = w.finish().unwrap();

    let mut w = EncoderWriter::new(Vec::new(), Default);
    w.write_all(&v).unwrap();
    let c = w.finish().unwrap();
    assert!(a == b && b == c);
}

#[test]
fn reset_reader() {
    let v = thread_rng()
                .gen_iter::<u8>()
                .take(1024 * 1024)
                .collect::<Vec<_>>();
    let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
    let mut r = EncoderReader::new(&v[..], Default);
    r.read_to_end(&mut a).unwrap();
    r.reset(&v[..]);
    r.read_to_end(&mut b).unwrap();

    let mut r = EncoderReader::new(&v[..], Default);
    r.read_to_end(&mut c).unwrap();
    assert!(a == b && b == c);
}

#[test]
fn reset_decoder() {
    let v = thread_rng()
                .gen_iter::<u8>()
                .take(1024 * 1024)
                .collect::<Vec<_>>();
    let mut w = EncoderWriter::new(Vec::new(), Default);
    w.write_all(&v).unwrap();
    let data = w.finish().unwrap();

    {
        let (mut a, mut b, mut c) = (Vec::new(), Vec::new(), Vec::new());
        let mut r = DecoderReader::new(&data[..]);
        r.read_to_end(&mut a).unwrap();
        r.reset(&data);
        r.read_to_end(&mut b).unwrap();

        let mut r = DecoderReader::new(&data[..]);
        r.read_to_end(&mut c).unwrap();
        assert!(a == b && b == c && c == v);
    }

    {
        let mut w = DecoderWriter::new(Vec::new());
        w.write_all(&data).unwrap();
        let a = w.reset(Vec::new()).unwrap();
        w.write_all(&data).unwrap();
        let b = w.finish().unwrap();

        let mut w = DecoderWriter::new(Vec::new());
        w.write_all(&data).unwrap();
        let c = w.finish().unwrap();
        assert!(a == b && b == c && c == v);
    }
}

#[test]
fn zero_length_read_with_data() {
    let m = vec![3u8; 128 * 1024 + 1];
    let mut c = EncoderReader::new(&m[..], ::Compression::Default);

    let mut result = Vec::new();
    c.read_to_end(&mut result).unwrap();

    let mut d = DecoderReader::new(&result[..]);
    let mut data = Vec::new();
    assert!(d.read(&mut data).unwrap() == 0);
}

#[test]
fn qc_reader() {
    ::quickcheck::quickcheck(test as fn(_) -> _);

    fn test(v: Vec<u8>) -> bool {
        let mut r = DecoderReader::new(EncoderReader::new(&v[..], Default));
        let mut v2 = Vec::new();
        r.read_to_end(&mut v2).unwrap();
        v == v2
    }
}

#[test]
fn qc_writer() {
    ::quickcheck::quickcheck(test as fn(_) -> _);

    fn test(v: Vec<u8>) -> bool {
        let mut w = EncoderWriter::new(DecoderWriter::new(Vec::new()), Default);
        w.write_all(&v).unwrap();
        v == w.finish().unwrap().finish().unwrap()
    }
}
