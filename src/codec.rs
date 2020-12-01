//! `Ipld` codecs.
use crate::cid::Cid;
use crate::error::{Result, UnsupportedCodec};
use core::convert::TryFrom;
use std::io::{Cursor, Read, Seek, Write};

/// Codec trait.
pub trait Codec:
    Copy + Unpin + Send + Sync + 'static + Sized + TryFrom<u64, Error = UnsupportedCodec> + Into<u64>
{
    /// Encodes an encodable type.
    fn encode<T: Encode<Self> + ?Sized>(&self, obj: &T) -> Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(u16::MAX as usize);
        obj.encode(*self, &mut buf)?;
        Ok(buf)
    }

    /// Decodes a decodable type.
    fn decode<T: Decode<Self>>(&self, mut bytes: &[u8]) -> Result<T> {
        T::decode(*self, &mut bytes)
    }

    /// Scrapes the references.
    fn references<T: References<Self>, E: Extend<Cid>>(
        &self,
        bytes: &[u8],
        set: &mut E,
    ) -> Result<()> {
        T::references(*self, &mut Cursor::new(bytes), set)
    }
}

/// Encode trait.
///
/// This trait is generic over a codec, so that different codecs can be implemented for the same
/// type.
pub trait Encode<C: Codec> {
    /// Encodes into a `impl Write`.
    ///
    /// It takes a specific codec as parameter, so that the [`Encode`] can be generic over an enum
    /// that contains multiple codecs.
    fn encode<W: Write>(&self, c: C, w: &mut W) -> Result<()>;
}

/// Decode trait.
///
/// This trait is generic over a codec, so that different codecs can be implemented for the same
/// type.
pub trait Decode<C: Codec>: Sized {
    /// Decode from an `impl Read`.
    ///
    /// It takes a specific codec as parameter, so that the [`Decode`] can be generic over an enum
    /// that contains multiple codecs.
    fn decode<R: Read>(c: C, r: &mut R) -> Result<Self>;
}

/// References trait.
///
/// This trait is generic over a codec, so that different codecs can be implemented for the same
/// type.
pub trait References<C: Codec>: Sized {
    /// Scrape the references from an `impl Read`.
    ///
    /// It takes a specific codec as parameter, so that the [`References`] can be generic over an
    /// enum that contains multiple codecs.
    fn references<R: Read + Seek, E: Extend<Cid>>(c: C, r: &mut R, set: &mut E) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipld::Ipld;
    use thiserror::Error;

    #[derive(Debug, Error)]
    #[error("not null")]
    pub struct NotNull;

    #[derive(Clone, Copy, Debug)]
    struct CodecImpl;

    impl Codec for CodecImpl {}

    impl From<CodecImpl> for u64 {
        fn from(_: CodecImpl) -> Self {
            0
        }
    }

    impl TryFrom<u64> for CodecImpl {
        type Error = UnsupportedCodec;

        fn try_from(_: u64) -> core::result::Result<Self, Self::Error> {
            Ok(Self)
        }
    }

    impl Encode<CodecImpl> for Ipld {
        fn encode<W: Write>(&self, _: CodecImpl, w: &mut W) -> Result<()> {
            match self {
                Self::Null => Ok(w.write_all(&[0])?),
                _ => Err(NotNull.into()),
            }
        }
    }

    impl Decode<CodecImpl> for Ipld {
        fn decode<R: Read>(_: CodecImpl, r: &mut R) -> Result<Self> {
            let mut buf = [0; 1];
            r.read_exact(&mut buf)?;
            if buf[0] == 0 {
                Ok(Ipld::Null)
            } else {
                Err(NotNull.into())
            }
        }
    }

    #[test]
    fn test_codec() {
        let bytes = CodecImpl.encode(&Ipld::Null).unwrap();
        let ipld: Ipld = CodecImpl.decode(&bytes).unwrap();
        assert_eq!(ipld, Ipld::Null);
    }
}
