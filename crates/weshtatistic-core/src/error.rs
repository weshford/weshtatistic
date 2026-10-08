use thiserror::Error;

#[derive(Error, Debug)]
pub enum WeshtatisticError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File too small to contain header")]
    HeaderTooSmall,

    #[error("Invalid magic bytes in snapshot header")]
    InvalidMagic,

    #[error("Unsupported snapshot version: {0}")]
    UnsupportedVersion(u16),

    #[error("Truncated snapshot file; nodes missing")]
    TruncatedNodes,

    #[error("Truncated snapshot file; string pool missing")]
    TruncatedStringPool,

    /// A snapshot integer field was too large for the host's address space or
    /// target integer width (e.g. an offset or count that overflows `usize`).
    #[error("Snapshot value out of range: {0}")]
    OutOfRange(&'static str),

    /// The snapshot's metadata is internally inconsistent — for example
    /// non-monotonic string-pool offsets, or a column shorter than the declared
    /// node count. The file is not merely truncated; its declared sizes
    /// contradict each other.
    #[error("Corrupt snapshot file: {0}")]
    Corrupt(&'static str),

    /// The byte stream could not be decoded — e.g. an overlong or unterminated
    /// varint. Reported separately from [`Self::TruncatedNodes`] so callers can
    /// distinguish "ran out of bytes" from "bytes are malformed".
    #[error("Malformed snapshot data: {0}")]
    Decode(&'static str),

    /// The string pool contained bytes that are not valid UTF-8.
    #[error("Snapshot string pool is not valid UTF-8")]
    InvalidUtf8,

    /// A Zstandard decompression failure on the snapshot container or payload.
    /// Kept separate from generic I/O so a truncated/corrupt *container* is
    /// distinguishable from a failing disk read.
    #[error("Zstd decompression error: {0}")]
    Zstd(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            WeshtatisticError::HeaderTooSmall.to_string(),
            "File too small to contain header"
        );
        assert_eq!(
            WeshtatisticError::InvalidMagic.to_string(),
            "Invalid magic bytes in snapshot header"
        );
        assert_eq!(
            WeshtatisticError::UnsupportedVersion(9).to_string(),
            "Unsupported snapshot version: 9"
        );
        assert_eq!(
            WeshtatisticError::TruncatedNodes.to_string(),
            "Truncated snapshot file; nodes missing"
        );
        assert_eq!(
            WeshtatisticError::TruncatedStringPool.to_string(),
            "Truncated snapshot file; string pool missing"
        );
        assert_eq!(
            WeshtatisticError::InvalidUtf8.to_string(),
            "Snapshot string pool is not valid UTF-8"
        );
        assert_eq!(
            WeshtatisticError::OutOfRange("custom").to_string(),
            "Snapshot value out of range: custom"
        );
        assert_eq!(
            WeshtatisticError::Corrupt("bad").to_string(),
            "Corrupt snapshot file: bad"
        );
        assert_eq!(
            WeshtatisticError::Decode("junk").to_string(),
            "Malformed snapshot data: junk"
        );
        assert_eq!(
            WeshtatisticError::Zstd("z".to_string()).to_string(),
            "Zstd decompression error: z"
        );
    }

    #[test]
    fn test_io_display_prefix() {
        let err = WeshtatisticError::from(std::io::Error::new(std::io::ErrorKind::NotFound, "gone"));
        assert!(matches!(err, WeshtatisticError::Io(_)));
        assert_eq!(err.to_string(), "I/O error: gone");
    }

    #[test]
    fn test_from_io_error_via_question_mark() {
        fn read_missing() -> Result<(), WeshtatisticError> {
            let dir = std::env::current_dir()?
                .join("target")
                .join("weshtatistic_core_error_test_nonexistent");
            // `?` must convert the io::Error into WeshtatisticError::Io.
            std::fs::read(dir.join("no_such_file.bin"))?;
            Ok(())
        }

        let result = read_missing();
        assert!(matches!(result, Err(WeshtatisticError::Io(_))));
    }
}
