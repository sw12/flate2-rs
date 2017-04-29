//! ZLIB compression and decompression of streams

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

/// A ZLIB encoder, or compressor.
///
/// This structure implements a [`BufRead`] interface and will read uncompressed
/// data from an underlying stream and emit a stream of compressed data.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
#[derive(Debug)]
pub struct ZlibEncoder<R> {
    pub obj: R,
    pub data: Compress,
}

/// A ZLIB decoder, or decompressor.
///
/// This structure implements a [`BufRead`] interface and takes a stream of
/// compressed data as input, providing the decompressed data when read from.
///
/// [`BufRead`]: https://doc.rust-lang.org/std/io/trait.BufRead.html
#[derive(Debug)]
pub struct ZlibDecoder<R> {
    pub obj: R,
    pub data: Decompress,
}

impl<R: BufRead> ZlibEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: ::Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            obj: r,
            data: Compress::new(level, true),
        }
    }
}

impl<R> ZlibEncoder<R> {
    /// Resets the state of this encoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This function will reset the internal state of this encoder and replace
    /// the input stream with the one provided, returning the previous input
    /// stream. Future data read from this encoder will be the compressed
    /// version of `r`'s data.
    pub fn reset(&mut self, r: R) -> R {
        self.data.reset();
        mem::replace(&mut self.obj, r)
    }

    /// Acquires a reference to the underlying reader
    pub fn get_ref(&self) -> &R {
        &self.obj
    }

    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.obj
    }

    /// Consumes this encoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.obj
    }

    /// Returns the number of bytes that have been read into this compressor.
    ///
    /// Note that not all bytes read from the underlying object may be accounted
    /// for, there may still be some active buffering.
    pub fn total_in(&self) -> u64 {
        self.data.total_in()
    }

    /// Returns the number of bytes that the compressor has produced.
    ///
    /// Note that not all bytes may have been read yet, some may still be
    /// buffered.
    pub fn total_out(&self) -> u64 {
        self.data.total_out()
    }
}

impl<R: BufRead> Read for ZlibEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        zio::read(&mut self.obj, &mut self.data, buf)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for ZlibEncoder<R> {
}

impl<R: BufRead + Write> Write for ZlibEncoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for ZlibEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}

impl<R: BufRead> ZlibDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> ZlibDecoder<R> {
        ZlibDecoder {
            obj: r,
            data: Decompress::new(true),
        }
    }
}

impl<R> ZlibDecoder<R> {
    /// Resets the state of this decoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This will reset the internal state of this decoder and replace the
    /// input stream with the one provided, returning the previous input
    /// stream. Future data read from this decoder will be the decompressed
    /// version of `r`'s data.
    pub fn reset(&mut self, r: R) -> R {
        self.data = Decompress::new(true);
        mem::replace(&mut self.obj, r)
    }

    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        &self.obj
    }

    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.obj
    }

    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.obj
    }

    /// Returns the number of bytes that the decompressor has consumed.
    ///
    /// Note that this will likely be smaller than what the decompressor
    /// actually read from the underlying stream due to buffering.
    pub fn total_in(&self) -> u64 {
        self.data.total_in()
    }

    /// Returns the number of bytes that the decompressor has produced.
    pub fn total_out(&self) -> u64 {
        self.data.total_out()
    }
}

impl<R: BufRead> Read for ZlibDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        zio::read(&mut self.obj, &mut self.data, into)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead + BufRead> AsyncRead for ZlibDecoder<R> {
}

impl<R: BufRead + Write> Write for ZlibDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncWrite + BufRead> AsyncWrite for ZlibDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
