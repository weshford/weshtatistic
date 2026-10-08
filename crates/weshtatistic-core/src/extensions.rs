//! Extension container for `.edst` snapshots.
//!
//! The V3 snapshot format reserves `FileHeader.reserved` for "future backward
//! compatibility". Rather than spending one slot per feature, this module
//! implements a single generic mechanism behind one slot: `reserved[0]` holds
//! the absolute offset (in the *decompressed* stream) of an **extension
//! directory** appended after the payload; `0` means "no extensions".
//!
//! Stream layout after the payload (`72 + uncompressed_size`):
//!
//! ```text
//! [extension payload bytes, concatenated in entry order]
//! "EDSX" | count: u32 LE | reserved: u32 LE (0)
//! count × { id: u32 LE | version: u32 LE | offset: u64 LE | length: u64 LE }
//! ```
//!
//! Compatibility properties:
//!
//! * Old readers stop at `72 + uncompressed_size` and ignore the tail, so a
//!   snapshot with extensions still loads in binaries that predate them.
//! * New readers see `reserved[0] == 0` in old snapshots as "no extensions".
//! * Unknown extension IDs are kept as opaque raw bytes, so re-saving a
//!   snapshot preserves extensions the writer does not understand.
//! * A malformed directory never fails the load; it degrades to "no
//!   extensions" because the core payload is independently intact.
//!
//! ## Extension ID registry
//!
//! IDs are 32-bit FourCC-style tags. Register new ones here, never reuse a
//! retired ID, and bump an extension's `version` when its payload layout
//! changes incompatibly.

use std::sync::Arc;

/// Docker disk-space inventory (`crate::docker::DockerInventory` as JSON).
pub const EXT_DOCKER_INVENTORY: u32 = u32::from_le_bytes(*b"DKR0");

/// Current payload version for [`EXT_DOCKER_INVENTORY`].
pub const EXT_DOCKER_INVENTORY_VERSION: u32 = 1;

const EXTENSION_DIR_MAGIC: [u8; 4] = *b"EDSX";
const DIR_HEADER_SIZE: usize = 12;
/// id(4) + version(4) + offset(8) + length(8).
const DIR_ENTRY_SIZE: usize = 24;
/// Sanity bound against corrupt counts pointing at huge allocations.
const MAX_EXTENSIONS: usize = 4096;

/// One extension: a typed payload blob carried inside a snapshot.
#[derive(Debug, Clone)]
pub struct ExtensionEntry {
    pub id: u32,
    pub version: u32,
    pub payload: Arc<[u8]>,
}

/// The set of extensions attached to a snapshot. Cheap to clone: payloads are
/// reference-counted.
#[derive(Debug, Clone, Default)]
pub struct ExtensionStore {
    entries: Vec<ExtensionEntry>,
}

impl ExtensionStore {
    /// Look up an extension by ID (latest inserted wins).
    #[must_use]
    pub fn get(&self, id: u32) -> Option<&ExtensionEntry> {
        self.entries.iter().rev().find(|e| e.id == id)
    }

    /// Insert or replace an extension payload.
    pub fn insert(&mut self, id: u32, version: u32, payload: Arc<[u8]>) {
        self.entries.retain(|e| e.id != id);
        self.entries.push(ExtensionEntry {
            id,
            version,
            payload,
        });
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub const fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate over all entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = &ExtensionEntry> {
        self.entries.iter()
    }

    /// Serialize payloads followed by the directory. `base_offset` is where
    /// the returned bytes will start within the final decompressed stream, so
    /// directory entries can carry absolute payload offsets.
    ///
    /// Returns the bytes and the directory's absolute offset (to be stored in
    /// `FileHeader.reserved[0]`), or `None` when the store is empty.
    #[must_use]
    pub fn encode(&self, base_offset: u64) -> Option<(Vec<u8>, u64)> {
        if self.entries.is_empty() {
            return None;
        }
        let mut bytes = Vec::new();
        let mut dir_entries = Vec::with_capacity(self.entries.len());
        for entry in &self.entries {
            let offset = base_offset + bytes.len() as u64;
            bytes.extend_from_slice(&entry.payload);
            dir_entries.push((entry.id, entry.version, offset, entry.payload.len() as u64));
        }
        let dir_offset = base_offset + bytes.len() as u64;
        bytes.extend_from_slice(&EXTENSION_DIR_MAGIC);
        bytes.extend_from_slice(&(dir_entries.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        for (id, version, offset, length) in dir_entries {
            bytes.extend_from_slice(&id.to_le_bytes());
            bytes.extend_from_slice(&version.to_le_bytes());
            bytes.extend_from_slice(&offset.to_le_bytes());
            bytes.extend_from_slice(&length.to_le_bytes());
        }
        Some((bytes, dir_offset))
    }

    /// Parse the directory at `dir_offset` within the decompressed `stream`.
    ///
    /// Tolerant by design: returns `None` for any malformed input (bad magic,
    /// out-of-bounds offsets, absurd counts) — callers degrade to "no
    /// extensions" rather than failing the whole snapshot load.
    #[must_use]
    pub fn decode(stream: &[u8], dir_offset: u64) -> Option<Self> {
        let dir_offset = usize::try_from(dir_offset).ok()?;
        let dir = stream.get(dir_offset..)?;
        if dir.len() < DIR_HEADER_SIZE || dir[0..4] != EXTENSION_DIR_MAGIC {
            return None;
        }
        let count = u32::from_le_bytes(dir[4..8].try_into().ok()?) as usize;
        if count > MAX_EXTENSIONS {
            return None;
        }
        let entries_len = count.checked_mul(DIR_ENTRY_SIZE)?;
        let entries_end = DIR_HEADER_SIZE.checked_add(entries_len)?;
        if dir.len() < entries_end {
            return None;
        }

        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let base = DIR_HEADER_SIZE + i * DIR_ENTRY_SIZE;
            let id = u32::from_le_bytes(dir[base..base + 4].try_into().ok()?);
            let version = u32::from_le_bytes(dir[base + 4..base + 8].try_into().ok()?);
            let offset = u64::from_le_bytes(dir[base + 8..base + 16].try_into().ok()?);
            let length = u64::from_le_bytes(dir[base + 16..base + 24].try_into().ok()?);
            let offset = usize::try_from(offset).ok()?;
            let length = usize::try_from(length).ok()?;
            let end = offset.checked_add(length)?;
            let payload: Arc<[u8]> = stream.get(offset..end)?.into();
            entries.push(ExtensionEntry {
                id,
                version,
                payload,
            });
        }
        Some(Self { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let mut store = ExtensionStore::default();
        store.insert(EXT_DOCKER_INVENTORY, 1, Arc::from(&b"{\"images\":[]}"[..]));
        store.insert(u32::from_le_bytes(*b"UNKN"), 7, Arc::from(&b"opaque"[..]));

        let base = 72 + 1234u64;
        let (bytes, dir_offset) = store.encode(base).unwrap_or_default();
        // Directory follows the two payloads (13 + 6 bytes).
        assert_eq!(dir_offset, base + 13 + 6);

        // Simulate a stream: base_offset bytes of payload, then extensions.
        let mut stream = vec![0u8; base as usize];
        stream.extend_from_slice(&bytes);

        let decoded = ExtensionStore::decode(&stream, dir_offset);
        assert!(decoded.is_some());
        let decoded = decoded.unwrap_or_default();
        assert_eq!(decoded.len(), 2);
        let dkr = decoded.get(EXT_DOCKER_INVENTORY);
        assert!(dkr.is_some());
        assert_eq!(dkr.map(|e| &*e.payload), Some(&b"{\"images\":[]}"[..]));
        let unknown = decoded.get(u32::from_le_bytes(*b"UNKN"));
        assert_eq!(unknown.map(|e| e.version), Some(7));
    }

    #[test]
    fn test_empty_store_encodes_to_none() {
        let store = ExtensionStore::default();
        assert!(store.encode(1024).is_none());
    }

    #[test]
    fn test_decode_rejects_malformed() {
        let stream = [0u8; 256];
        // Bad magic.
        assert!(ExtensionStore::decode(&stream, 0).is_none());
        // Out-of-bounds directory offset.
        assert!(ExtensionStore::decode(&stream, 10_000).is_none());
        // Valid magic, truncated entries.
        let mut s = Vec::new();
        s.extend_from_slice(&EXTENSION_DIR_MAGIC);
        s.extend_from_slice(&5u32.to_le_bytes());
        s.extend_from_slice(&0u32.to_le_bytes());
        assert!(ExtensionStore::decode(&s, 0).is_none());
        // Absurd count.
        let mut s = Vec::new();
        s.extend_from_slice(&EXTENSION_DIR_MAGIC);
        s.extend_from_slice(&u32::MAX.to_le_bytes());
        s.extend_from_slice(&0u32.to_le_bytes());
        s.resize(4096, 0);
        assert!(ExtensionStore::decode(&s, 0).is_none());
    }

    #[test]
    fn test_insert_replaces_same_id() {
        let mut store = ExtensionStore::default();
        store.insert(EXT_DOCKER_INVENTORY, 1, Arc::from(&b"old"[..]));
        store.insert(EXT_DOCKER_INVENTORY, 2, Arc::from(&b"new"[..]));
        assert_eq!(store.len(), 1);
        let entry = store.get(EXT_DOCKER_INVENTORY);
        assert_eq!(entry.map(|e| e.version), Some(2));
        assert_eq!(entry.map(|e| &*e.payload), Some(&b"new"[..]));
    }
}
