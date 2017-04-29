//! Types which operate over [`Read`] streams, both encoders and decoders for
//! various formats.
//!
//! [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html

use std::io::prelude::*;
use std::io;

#[cfg(feature = "tokio")]
use futures::Poll;
#[cfg(feature = "tokio")]
use tokio_io::{AsyncRead, AsyncWrite};

use bufreader::BufReader;
use bufread;
use gz;
use {Compression, Decompress};

/// A DEFLATE encoder, or compressor.
///
/// This structure implements a [`Read`] interface and will read uncompressed
/// data from an underlying stream and emit a stream of compressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct DeflateEncoder<R> {
    inner: bufread::DeflateEncoder<BufReader<R>>,
}

/// A DEFLATE decoder, or decompressor.
///
/// This structure implements a [`Read`] interface and takes a stream of
/// compressed data as input, providing the decompressed data when read from.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct DeflateDecoder<R> {
    inner: bufread::DeflateDecoder<BufReader<R>>,
}

impl<R: Read> DeflateEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: ::Compression) -> DeflateEncoder<R> {
        DeflateEncoder {
            inner: bufread::DeflateEncoder::new(BufReader::new(r), level),
        }
    }
}

impl<R> DeflateEncoder<R> {
    /// Resets the state of this encoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This function will reset the internal state of this encoder and replace
    /// the input stream with the one provided, returning the previous input
    /// stream. Future data read from this encoder will be the compressed
    /// version of `r`'s data.
    ///
    /// Note that there may be currently buffered data when this function is
    /// called, and in that case the buffered data is discarded.
    pub fn reset(&mut self, r: R) -> R {
        self.inner.data.reset();
        self.inner.obj.reset(r)
    }

    /// Acquires a reference to the underlying reader
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Consumes this encoder, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It's recommended to only call this function after
    /// EOF has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }

    /// Returns the number of bytes that have been read into this compressor.
    ///
    /// Note that not all bytes read from the underlying object may be accounted
    /// for, there may still be some active buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.data.total_in()
    }

    /// Returns the number of bytes that the compressor has produced.
    ///
    /// Note that not all bytes may have been read yet, some may still be
    /// buffered.
    pub fn total_out(&self) -> u64 {
        self.inner.data.total_out()
    }
}

impl<R: Read> Read for DeflateEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for DeflateEncoder<R> {
}

impl<W: Read + Write> Write for DeflateEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead + AsyncWrite> AsyncWrite for DeflateEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}

impl<R: Read> DeflateDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> DeflateDecoder<R> {
        DeflateDecoder::new_with_buf(r, vec![0; 32 * 1024])
    }

    /// Same as `new`, but the intermediate buffer for data is specified.
    ///
    /// Note that the capacity of the intermediate buffer is never increased,
    /// and it is recommended for it to be large.
    pub fn new_with_buf(r: R, buf: Vec<u8>) -> DeflateDecoder<R> {
        DeflateDecoder {
            inner: bufread::DeflateDecoder::new(BufReader::with_buf(buf, r))
        }
    }
}

impl<R> DeflateDecoder<R> {
    /// Resets the state of this decoder entirely, swapping out the input
    /// stream for another.
    ///
    /// This will reset the internal state of this decoder and replace the
    /// input stream with the one provided, returning the previous input
    /// stream. Future data read from this decoder will be the decompressed
    /// version of `r`'s data.
    ///
    /// Note that there may be currently buffered data when this function is
    /// called, and in that case the buffered data is discarded.
    pub fn reset(&mut self, r: R) -> R {
        self.inner.data = Decompress::new(false);
        self.inner.obj.reset(r)
    }

    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Consumes this decoder, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It's recommended to only call this function after
    /// EOF has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }

    /// Returns the number of bytes that the decompressor has consumed.
    ///
    /// Note that this will likely be smaller than what the decompressor
    /// actually read from the underlying stream due to buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.total_in()
    }

    /// Returns the number of bytes that the decompressor has produced.
    pub fn total_out(&self) -> u64 {
        self.inner.total_out()
    }
}

impl<R: Read> Read for DeflateDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for DeflateDecoder<R> {
}

impl<W: Read + Write> Write for DeflateDecoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncWrite + AsyncRead> AsyncWrite for DeflateDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}

/// A gzip streaming encoder
///
/// This structure exposes a [`Read`] interface that will read uncompressed data
/// from the underlying reader and expose the compressed version as a [`Read`]
/// interface.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct GzEncoder<R> {
    pub inner: bufread::GzEncoder<BufReader<R>>,
}

/// A gzip streaming decoder
///
/// This structure exposes a [`Read`] interface that will consume compressed
/// data from the underlying reader and emit uncompressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct GzDecoder<R> {
    inner: bufread::GzDecoder<BufReader<R>>,
}

/// A gzip streaming decoder that decodes all members of a multistream
///
/// A gzip member consists of a header, compressed data and a trailer. The [gzip
/// specification](https://tools.ietf.org/html/rfc1952), however, allows multiple
/// gzip members to be joined in a single stream.  `MultiGzDecoder` will
/// decode all consecutive members while `GzDecoder` will only decompress the
/// first gzip member. The multistream format is commonly used in bioinformatics,
/// for example when using the BGZF compressed data.
///
/// This structure exposes a [`Read`] interface that will consume all gzip members
/// from the underlying reader and emit uncompressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct MultiGzDecoder<R> {
    inner: bufread::MultiGzDecoder<BufReader<R>>,
}

impl<R: Read> GzEncoder<R> {
    /// Creates a new encoder which will use the given compression level.
    ///
    /// The encoder is not configured specially for the emitted header. For
    /// header configuration, see the `gz::Builder` type.
    ///
    /// The data read from the stream `r` will be compressed and available
    /// through the returned reader.
    pub fn new(r: R, level: Compression) -> GzEncoder<R> {
        gz::Builder::new().read(r, level)
    }
}

impl<R> GzEncoder<R> {
    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying reader.
    ///
    /// Note that mutation of the reader may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Returns the underlying stream, consuming this encoder
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}

impl<R: Read> Read for GzEncoder<R> {
    fn read(&mut self, mut into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}

impl<R: Read + Write> Write for GzEncoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

impl<R: Read> GzDecoder<R> {
    /// Creates a new decoder from the given reader, immediately parsing the
    /// gzip header.
    ///
    /// # Errors
    ///
    /// If an error is encountered when parsing the gzip header, an error is
    /// returned.
    pub fn new(r: R) -> io::Result<GzDecoder<R>> {
        bufread::GzDecoder::new(BufReader::new(r)).map(|r| {
            GzDecoder { inner: r }
        })
    }
}

impl<R> GzDecoder<R> {
    /// Returns the header associated with this stream.
    pub fn header(&self) -> &gz::Header {
        self.inner.header()
    }

    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying stream.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}

impl<R: Read> Read for GzDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}

impl<R: Read + Write> Write for GzDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

impl<R: Read> MultiGzDecoder<R> {
    /// Creates a new decoder from the given reader, immediately parsing the
    /// (first) gzip header. If the gzip stream contains multiple members all will
    /// be decoded.
    ///
    /// # Errors
    ///
    /// If an error is encountered when parsing the gzip header, an error is
    /// returned.
    pub fn new(r: R) -> io::Result<MultiGzDecoder<R>> {
        bufread::MultiGzDecoder::new(BufReader::new(r)).map(|r| {
            MultiGzDecoder { inner: r }
        })
    }
}

impl<R> MultiGzDecoder<R> {
    /// Returns the current header associated with this stream.
    pub fn header(&self) -> &gz::Header {
        self.inner.header()
    }

    /// Acquires a reference to the underlying reader.
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying stream.
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Consumes this decoder, returning the underlying reader.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }
}

impl<R: Read> Read for MultiGzDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}

impl<R: Read + Write> Write for MultiGzDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

/// A ZLIB encoder, or compressor.
///
/// This structure implements a [`Read`] interface and will read uncompressed
/// data from an underlying stream and emit a stream of compressed data.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct ZlibEncoder<R> {
    inner: bufread::ZlibEncoder<BufReader<R>>,
}

/// A ZLIB decoder, or decompressor.
///
/// This structure implements a [`Read`] interface and takes a stream of
/// compressed data as input, providing the decompressed data when read from.
///
/// [`Read`]: https://doc.rust-lang.org/std/io/trait.Read.html
#[derive(Debug)]
pub struct ZlibDecoder<R> {
    inner: bufread::ZlibDecoder<BufReader<R>>,
}

impl<R: Read> ZlibEncoder<R> {
    /// Creates a new encoder which will read uncompressed data from the given
    /// stream and emit the compressed stream.
    pub fn new(r: R, level: ::Compression) -> ZlibEncoder<R> {
        ZlibEncoder {
            inner: bufread::ZlibEncoder::new(BufReader::new(r), level),
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
    ///
    /// Note that there may be currently buffered data when this function is
    /// called, and in that case the buffered data is discarded.
    pub fn reset(&mut self, r: R) -> R {
        self.inner.data.reset();
        self.inner.obj.reset(r)
    }

    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Consumes this encoder, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It's recommended to only call this function after
    /// EOF has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }

    /// Returns the number of bytes that have been read into this compressor.
    ///
    /// Note that not all bytes read from the underlying object may be accounted
    /// for, there may still be some active buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.data.total_in()
    }

    /// Returns the number of bytes that the compressor has produced.
    ///
    /// Note that not all bytes may have been read yet, some may still be
    /// buffered.
    pub fn total_out(&self) -> u64 {
        self.inner.data.total_out()
    }
}

impl<R: Read> Read for ZlibEncoder<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for ZlibEncoder<R> {
}

impl<W: Read + Write> Write for ZlibEncoder<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead + AsyncWrite> AsyncWrite for ZlibEncoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}

impl<R: Read> ZlibDecoder<R> {
    /// Creates a new decoder which will decompress data read from the given
    /// stream.
    pub fn new(r: R) -> ZlibDecoder<R> {
        ZlibDecoder::new_with_buf(r, vec![0; 32 * 1024])
    }

    /// Same as `new`, but the intermediate buffer for data is specified.
    ///
    /// Note that the specified buffer will only be used up to its current
    /// length. The buffer's capacity will also not grow over time.
    pub fn new_with_buf(r: R, buf: Vec<u8>) -> ZlibDecoder<R> {
        ZlibDecoder {
            inner: bufread::ZlibDecoder::new(BufReader::with_buf(buf, r)),
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
    ///
    /// Note that there may be currently buffered data when this function is
    /// called, and in that case the buffered data is discarded.
    pub fn reset(&mut self, r: R) -> R {
        self.inner.data = Decompress::new(true);
        self.inner.obj.reset(r)
    }

    /// Acquires a reference to the underlying stream
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying stream
    ///
    /// Note that mutation of the stream may result in surprising results if
    /// this encoder is continued to be used.
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut().get_mut()
    }

    /// Consumes this decoder, returning the underlying reader.
    ///
    /// Note that there may be buffered bytes which are not re-acquired as part
    /// of this transition. It's recommended to only call this function after
    /// EOF has been reached.
    pub fn into_inner(self) -> R {
        self.inner.into_inner().into_inner()
    }

    /// Returns the number of bytes that the decompressor has consumed.
    ///
    /// Note that this will likely be smaller than what the decompressor
    /// actually read from the underlying stream due to buffering.
    pub fn total_in(&self) -> u64 {
        self.inner.total_in()
    }

    /// Returns the number of bytes that the decompressor has produced.
    pub fn total_out(&self) -> u64 {
        self.inner.total_out()
    }
}

impl<R: Read> Read for ZlibDecoder<R> {
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        self.inner.read(into)
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncRead> AsyncRead for ZlibDecoder<R> {
}

impl<R: Read + Write> Write for ZlibDecoder<R> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.get_mut().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.get_mut().flush()
    }
}

#[cfg(feature = "tokio")]
impl<R: AsyncWrite + AsyncRead> AsyncWrite for ZlibDecoder<R> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.get_mut().shutdown()
    }
}
