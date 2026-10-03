//! Bounded, incremental JSONL decoding for Message Batch results.

use crate::{
    error::{AnthropicError, Result},
    models::batch::MessageBatchResultEntry,
};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Memory ceiling for one JSONL result row, excluding its newline.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct BatchResultsStreamOptions {
    /// Maximum row size in bytes. Defaults to 8 MiB.
    pub max_row_bytes: usize,
}

impl BatchResultsStreamOptions {
    /// Construct a positive maximum row-byte ceiling.
    pub fn new(max_row_bytes: usize) -> Result<Self> {
        let limits = Self { max_row_bytes };
        limits.validate()?;
        Ok(limits)
    }

    pub(super) fn validate(self) -> Result<()> {
        if self.max_row_bytes == 0 {
            return Err(AnthropicError::invalid_input(
                "Batch result row limit must be positive",
            ));
        }
        Ok(())
    }
}

impl Default for BatchResultsStreamOptions {
    fn default() -> Self {
        Self {
            max_row_bytes: 8 * 1024 * 1024,
        }
    }
}

/// Incremental batch results. Dropping releases the response and cancels further reads.
pub struct BatchResultsStream {
    inner: Pin<Box<dyn Stream<Item = Result<MessageBatchResultEntry>> + Send>>,
}

impl Stream for BatchResultsStream {
    type Item = Result<MessageBatchResultEntry>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

impl BatchResultsStream {
    pub(super) fn from_response(
        response: reqwest::Response,
        limits: BatchResultsStreamOptions,
    ) -> Result<Self> {
        if !response.status().is_success() {
            return Err(AnthropicError::api_error(
                response.status().as_u16(),
                "Batch results request failed".to_string(),
                None,
            ));
        }
        let state = JsonlState::new(Box::pin(response.bytes_stream()), limits);
        Ok(Self {
            inner: Box::pin(futures::stream::try_unfold(state, JsonlState::next_entry)),
        })
    }
}

struct JsonlState<S> {
    source: S,
    chunk: Vec<u8>,
    row: Vec<u8>,
    offset: usize,
    line: usize,
    eof: bool,
    limits: BatchResultsStreamOptions,
}

impl<S, B> JsonlState<S>
where
    S: Stream<Item = reqwest::Result<B>> + Unpin,
    B: AsRef<[u8]>,
{
    fn new(source: S, limits: BatchResultsStreamOptions) -> Self {
        Self {
            source,
            chunk: Vec::new(),
            row: Vec::new(),
            offset: 0,
            line: 1,
            eof: false,
            limits,
        }
    }

    async fn next_entry(mut self) -> Result<Option<(MessageBatchResultEntry, Self)>> {
        loop {
            if let Some(entry) = self.read_chunk()? {
                return Ok(Some((entry, self)));
            }
            if self.eof {
                return Ok(self.parse_row()?.map(|entry| (entry, self)));
            }
            match self.source.next().await {
                Some(Ok(bytes)) => {
                    self.chunk = bytes.as_ref().to_vec();
                    self.offset = 0;
                }
                Some(Err(_)) => {
                    return Err(AnthropicError::stream(format!(
                        "Transport failure reading batch result row {}",
                        self.line
                    )))
                }
                None => {
                    self.eof = true;
                    self.chunk.clear();
                    self.offset = 0;
                }
            }
        }
    }

    fn read_chunk(&mut self) -> Result<Option<MessageBatchResultEntry>> {
        while self.offset < self.chunk.len() {
            let remaining = &self.chunk[self.offset..];
            let newline = remaining.iter().position(|byte| *byte == b'\n');
            let bytes = newline.unwrap_or(remaining.len());
            self.append_segment(bytes)?;
            if newline.is_some() {
                self.offset += 1;
                if let Some(entry) = self.parse_row()? {
                    return Ok(Some(entry));
                }
            }
        }
        Ok(None)
    }

    fn append_segment(&mut self, bytes: usize) -> Result<()> {
        if self.row.len().saturating_add(bytes) > self.limits.max_row_bytes {
            return Err(AnthropicError::json(format!(
                "Batch result row {} exceeds the byte limit",
                self.line
            )));
        }
        self.row
            .extend_from_slice(&self.chunk[self.offset..self.offset + bytes]);
        self.offset += bytes;
        Ok(())
    }

    fn parse_row(&mut self) -> Result<Option<MessageBatchResultEntry>> {
        let row_number = self.line;
        self.line += 1;
        if self.row.iter().all(u8::is_ascii_whitespace) {
            self.row.clear();
            return Ok(None);
        }
        let entry = serde_json::from_slice(&self.row).map_err(|_| {
            AnthropicError::json(format!(
                "Invalid JSON or UTF-8 in batch result row {row_number}"
            ))
        })?;
        self.row.clear();
        Ok(Some(entry))
    }
}
