//! Functions for asynchronously writing to cache.
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures::prelude::*;
use ssri::{Algorithm, Integrity};

pub use crate::put::PutOpts;
use crate::content::write;
use crate::errors::Error;
use crate::index;

/// Writes `data` to the `cache`, indexing it under `key`.
pub async fn data<P, D, K>(cache: P, key: K, data: D) -> Result<Integrity, Error>
where
    P: AsRef<Path>,
    D: AsRef<[u8]>,
    K: AsRef<str>,
{
    let mut writer = PutOpts::new()
        .algorithm(Algorithm::Sha256)
        .open_async(cache.as_ref(), key.as_ref()).await?;
    writer.write_all(data.as_ref()).await?;
    writer.commit().await
}

impl PutOpts {
    /// Opens the file handle for writing, returning a Put instance.
    pub async fn open_async<P, K>(self, cache: P, key: K) -> Result<AsyncPut, Error>
    where
        P: AsRef<Path>,
        K: AsRef<str>,
    {
        Ok(AsyncPut {
            cache: cache.as_ref().to_path_buf(),
            key: String::from(key.as_ref()),
            written: 0,
            writer: write::AsyncWriter::new(
                cache.as_ref(),
                *self.algorithm.as_ref().unwrap_or(&Algorithm::Sha256),
            ).await?,
            opts: self,
        })
    }
}

/// A reference to an open file writing to the cache.
pub struct AsyncPut {
    cache: PathBuf,
    key: String,
    written: usize,
    pub(crate) writer: write::AsyncWriter,
    opts: PutOpts,
}

impl AsyncWrite for AsyncPut {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.writer).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.writer).poll_flush(cx)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.writer).poll_close(cx)
    }
}

impl AsyncPut {
    /// Closes the Put handle and writes content and index entries. Also
    /// verifies data against `size` and `integrity` options, if provided.
    /// Must be called manually in order to complete the writing process,
    /// otherwise everything will be thrown out.
    pub async fn commit(self) -> Result<Integrity, Error> {
        let writer_sri = self.writer.close().await?;
        if let Some(sri) = &self.opts.sri {
            // TODO - ssri should have a .matches method
            let algo = sri.pick_algorithm();
            let matched = sri
                .hashes
                .iter()
                .take_while(|h| h.algorithm == algo)
                .find(|&h| *h == writer_sri.hashes[0]);
            if matched.is_none() {
                return Err(Error::IntegrityError);
            }
        }
        if let Some(size) = self.opts.size {
            if size != self.written {
                return Err(Error::SizeError);
            }
        }
        index::insert(&self.cache, &self.key, self.opts)?;
        Ok(writer_sri)
    }
}

