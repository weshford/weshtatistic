//! Docker/container disk-space domain types shared between the native engine
//! (which collects them from disk or the daemon API) and the GUI (which
//! renders them).
//!
//! This module is pure data plus the [`DockerCollector`] seam: all I/O lives
//! in the native engine crate, keeping `weshtatistic-core` platform- and
//! I/O-agnostic. Collection is disk-first (no daemon socket required);
//! platforms where Docker state lives inside a VM disk image (Docker Desktop
//! on macOS/Windows) are represented by [`VmDisk`] entries in
//! [`DockerEnvironment`].
//!
//! A [`DockerInventory`] is serde-serializable so it can be embedded into
//! `.edst` snapshots as the `EXT_DOCKER_INVENTORY` extension payload (see
//! `crate::extensions`), enabling headless collect-now/review-later
//! workflows. All structs use `#[serde(default)]` so older payloads decode
//! when newer fields are added.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use serde::{Deserialize, Serialize};

use crate::WeshtatisticError;

/// Native-only Docker inventory provider. Implemented by the engine crate;
/// the GUI reaches it through `ScanController::docker_collector`.
pub trait DockerCollector: Send + Sync {
    /// Cheap probe of well-known Docker locations. Must not walk large trees;
    /// safe to call once at GUI startup.
    fn detect_environment(&self) -> DockerEnvironment;

    /// Collect an inventory using the best available source: the daemon API
    /// when a socket is reachable (full attribution on any storage layout),
    /// otherwise a disk parse of the data root (full for `overlay2`, partial
    /// for the containerd image store). May read many small metadata files
    /// and/or talk to the daemon; runs on a background thread and must honor
    /// `cancel` between units of work.
    ///
    /// # Errors
    /// Returns `Err` only when every source fails: no daemon reachable *and*
    /// no parseable data root. A disk root using the containerd image store
    /// is *not* an error: it yields a partial inventory (containers, logs,
    /// volumes, build cache) with a [`InventoryWarning::ContainerdStorePartial`].
    /// An existing but unreadable root is also not an error: it yields an
    /// empty inventory carrying an [`InventoryWarning::RootUnreadable`] so
    /// the UI can explain the privilege requirement.
    fn collect_inventory(&self, cancel: &AtomicBool) -> Result<DockerInventory, WeshtatisticError>;

    /// Delete an image by ID via the daemon API (`DELETE /images/{id}`).
    /// `force` maps to the daemon's force flag (removes even when containers
    /// reference the image). The default implementation reports
    /// "unsupported" — collectors without daemon access (or the wasm
    /// frontend) simply cannot delete.
    ///
    /// # Errors
    /// Returns `Err` when the daemon rejects the deletion (e.g. image in use
    /// without `force`) or is unreachable.
    fn delete_image(&self, image_id: &str, force: bool) -> Result<ImageDeletion, WeshtatisticError> {
        let _ = (image_id, force);
        Err(WeshtatisticError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "image deletion is not supported by this collector",
        )))
    }

    /// Delete a container by ID via the daemon API
    /// (`DELETE /containers/{id}`). `force` removes even running containers;
    /// anonymous volumes are kept (`v=false`).
    ///
    /// # Errors
    /// Returns `Err` when the daemon rejects the deletion (e.g. container
    /// running without `force`) or is unreachable.
    fn delete_container(&self, container_id: &str, force: bool) -> Result<(), WeshtatisticError> {
        let _ = (container_id, force);
        Err(WeshtatisticError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "container deletion is not supported by this collector",
        )))
    }

    /// Delete a named volume via the daemon API (`DELETE /volumes/{name}`).
    /// `force` removes even when containers reference the volume.
    ///
    /// # Errors
    /// Returns `Err` when the daemon rejects the deletion (e.g. volume in use
    /// without `force`) or is unreachable.
    fn delete_volume(&self, name: &str, force: bool) -> Result<(), WeshtatisticError> {
        let _ = (name, force);
        Err(WeshtatisticError::Io(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "volume deletion is not supported by this collector",
        )))
    }
}

/// What the daemon reports after a successful `DELETE /images/{id}`.
#[derive(Debug, Clone, Default)]
pub struct ImageDeletion {
    /// Tag references removed (e.g. `repo:tag`).
    pub untagged: Vec<String>,
    /// Layer/image IDs actually deleted.
    pub deleted: Vec<String>,
}

/// Result of probing well-known Docker locations on this machine.
#[derive(Debug, Clone, Default)]
pub struct DockerEnvironment {
    /// Candidate Docker data roots (e.g. `/var/lib/docker`), most likely first.
    pub data_roots: Vec<DataRoot>,
    /// VM disk images used by Docker Desktop-style setups (opaque blobs).
    pub vm_disks: Vec<VmDisk>,
    /// Reachable-looking daemon socket, if one was discovered (e.g.
    /// `/var/run/docker.sock`, rootless, or `$DOCKER_HOST`). A daemon can be
    /// usable even when no data root is visible in this filesystem namespace
    /// (remote socket, bind-mounted into a container, …).
    pub daemon_socket: Option<PathBuf>,
}

impl DockerEnvironment {
    /// True when no Docker presence of any kind was detected on disk.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.data_roots.is_empty() && self.vm_disks.is_empty()
    }

    /// True when anything Docker-ish is usable: a data root, a VM disk, or a
    /// daemon socket. Collection only needs one of them.
    #[must_use]
    pub const fn has_docker(&self) -> bool {
        !self.is_empty() || self.daemon_socket.is_some()
    }

    /// The preferred root for inventory collection: first accessible one.
    #[must_use]
    pub fn primary_root(&self) -> Option<&DataRoot> {
        self.data_roots
            .iter()
            .find(|r| r.accessible)
            .or_else(|| self.data_roots.first())
    }
}

/// A directory that looks like a Docker (or compatible) data root.
#[derive(Debug, Clone)]
pub struct DataRoot {
    pub path: PathBuf,
    pub kind: StorageKind,
    pub scope: RootScope,
    /// Whether we can read metadata inside it (false → likely needs root).
    pub accessible: bool,
}

/// On-disk storage layout of a data root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageKind {
    /// Classic Docker `overlay2` layout (`overlay2/`, `image/overlay2/`).
    /// The only layout this crate can fully parse from disk so far.
    Overlay2,
    /// Legacy `overlay` (v1) driver.
    Overlay,
    /// Legacy `aufs` driver.
    Aufs,
    /// `btrfs` driver (layers are subvolumes).
    Btrfs,
    /// `zfs` driver (layers are datasets).
    Zfs,
    /// Legacy `devicemapper` driver.
    Devicemapper,
    /// `vfs` driver (full copies per layer; no dedup metadata).
    Vfs,
    /// Windows native containers (`windowsfilter/`).
    WindowsFilter,
    /// Containerd image store (`containerd/`, bolt metadata — not disk-parseable).
    Containerd,
    /// Anything else we cannot classify yet.
    Unknown,
}

/// System-wide vs. per-user (rootless) Docker install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootScope {
    System,
    Rootless,
}

/// A VM disk image that holds all of Docker Desktop's data opaquely.
#[derive(Debug, Clone)]
pub struct VmDisk {
    pub path: PathBuf,
    /// Logical file size (what `len()` reports; can be huge for sparse files).
    pub apparent_bytes: u64,
    /// Blocks actually allocated on the host filesystem.
    pub allocated_bytes: u64,
    pub backend: VmBackend,
}

/// Which Docker Desktop-style backend a [`VmDisk`] belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmBackend {
    /// macOS Docker Desktop (`Docker.raw`).
    DockerDesktopMac,
    /// Windows Docker Desktop on WSL2 (`*.vhdx`).
    DockerDesktopWsl2,
}

/// How an inventory was produced.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum InventorySource {
    /// Parsed from an on-disk `overlay2` data root.
    #[default]
    DiskOverlay2,
    /// Partial disk parse of a containerd-store root (no image attribution).
    DiskContainerdPartial,
    /// Collected from the daemon API (`GET /system/df` over the local socket).
    DockerApi,
}

/// Full disk-parsed inventory of one Docker data root.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerInventory {
    /// The data root this inventory was built from.
    pub root: Option<PathBuf>,
    pub images: Vec<ImageInfo>,
    pub layers: Vec<LayerInfo>,
    pub containers: Vec<ContainerInfo>,
    pub volumes: Vec<VolumeInfo>,
    /// Total bytes under `buildkit/` (per-item attribution needs the daemon).
    pub build_cache_bytes: u64,
    /// Collection time, epoch seconds (0 = unknown).
    pub collected_at: u64,
    pub source: InventorySource,
    pub totals: InventoryTotals,
    pub warnings: Vec<InventoryWarning>,
}

impl DockerInventory {
    /// Recompute [`InventoryTotals`] from the per-item data. Call after
    /// building or mutating the inventory.
    pub fn compute_totals(&mut self) {
        let mut t = InventoryTotals {
            build_cache_bytes: self.build_cache_bytes,
            ..InventoryTotals::default()
        };
        for image in &self.images {
            t.images_bytes += image.size_bytes;
            t.shared_bytes += image.shared_bytes;
        }
        for layer in &self.layers {
            if !layer.container_mount {
                t.unique_image_bytes += layer.size_bytes;
            }
            if !layer.referenced {
                t.reclaimable_bytes += layer.size_bytes;
            }
        }
        for container in &self.containers {
            t.container_rw_bytes += container.rw_size_bytes;
            t.log_bytes += container.log_bytes;
        }
        for volume in &self.volumes {
            t.volumes_bytes += volume.size_bytes;
        }
        self.totals = t;
    }

    /// Encode as the `EXT_DOCKER_INVENTORY` snapshot extension payload (JSON).
    ///
    /// # Errors
    /// Returns `Err` if serialization fails (not expected for this plain-data
    /// shape; surfaced for completeness).
    pub fn to_extension_payload(&self) -> Result<Vec<u8>, WeshtatisticError> {
        serde_json::to_vec(self)
            .map_err(|e| WeshtatisticError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, e)))
    }

    /// Decode an `EXT_DOCKER_INVENTORY` payload; `None` when malformed.
    #[must_use]
    pub fn from_extension_payload(bytes: &[u8]) -> Option<Self> {
        serde_json::from_slice(bytes).ok()
    }
}

/// Aggregate byte counts across the inventory.
///
/// `images_bytes` counts a shared base layer once per referencing image
/// (what `docker images` shows); `unique_image_bytes` counts each physical
/// layer once (what the disk actually holds).
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct InventoryTotals {
    pub images_bytes: u64,
    pub shared_bytes: u64,
    pub unique_image_bytes: u64,
    pub container_rw_bytes: u64,
    pub volumes_bytes: u64,
    pub build_cache_bytes: u64,
    pub log_bytes: u64,
    /// Layers on disk that no image or container references anymore.
    pub reclaimable_bytes: u64,
}

/// One Docker image and its size attribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ImageInfo {
    /// Full image ID (hex, without `sha256:` prefix).
    pub id: String,
    /// `repository:tag` names; empty = dangling.
    pub tags: Vec<String>,
    /// Sum of all layer sizes, including layers shared with other images.
    pub size_bytes: u64,
    /// Portion of `size_bytes` in layers shared with at least one other image.
    pub shared_bytes: u64,
    /// Cache IDs of this image's layers, bottom layer first.
    pub layer_cache_ids: Vec<String>,
    /// Number of layers. Equals `layer_cache_ids.len()` for disk-parsed
    /// inventories; filled from image inspect for API-sourced ones.
    pub layer_count: u32,
    /// How many containers reference this image (0 = prune candidate).
    pub ref_count: u32,
    /// Creation time, epoch seconds (0 = unknown).
    pub created: u64,
}

impl ImageInfo {
    /// First 12 chars of the ID, matching `docker images` output.
    #[must_use]
    pub fn short_id(&self) -> &str {
        self.id.get(..12).unwrap_or(&self.id)
    }

    /// Human label: first tag, or `<none>` for dangling images.
    #[must_use]
    pub fn display_name(&self) -> &str {
        self.tags.first().map_or("<none>", String::as_str)
    }

    /// Bytes no other image uses: `size_bytes - shared_bytes`.
    #[must_use]
    pub const fn exclusive_bytes(&self) -> u64 {
        self.size_bytes.saturating_sub(self.shared_bytes)
    }
}

/// One physical `overlay2/<cache-id>` layer directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LayerInfo {
    /// Directory name under `overlay2/` (the cache ID).
    pub cache_id: String,
    /// Logical size from `layerdb`, bytes.
    pub size_bytes: u64,
    /// On-disk content directory (`overlay2/<cache-id>/diff`).
    pub path: PathBuf,
    /// Images referencing this layer (full image IDs).
    pub image_ids: Vec<String>,
    /// True if this is a container read-write (or init) layer, not an image layer.
    pub container_mount: bool,
    /// False = orphaned data a prune would remove.
    pub referenced: bool,
}

/// One container's disk footprint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    /// Image ID from the container config, if resolvable.
    pub image_id: Option<String>,
    /// Writable-layer bytes (0 when the layer mapping was not found).
    pub rw_size_bytes: u64,
    /// Size of `<id>-json.log`, the classic disk hog.
    pub log_bytes: u64,
    /// Creation time, epoch seconds (0 = unknown).
    pub created: u64,
}

impl ContainerInfo {
    /// First 12 chars of the ID, matching `docker ps` output.
    #[must_use]
    pub fn short_id(&self) -> &str {
        self.id.get(..12).unwrap_or(&self.id)
    }
}

/// One named volume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VolumeInfo {
    pub name: String,
    pub size_bytes: u64,
    /// How many containers mount this volume (0 = prune candidate).
    pub ref_count: u32,
    /// Creation time, epoch seconds (0 = unknown).
    pub created: u64,
}

/// Non-fatal findings worth surfacing in the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryWarning {
    /// A container json log exceeds a sane threshold.
    LargeContainerLog { container: String, bytes: u64 },
    /// An `overlay2/` directory referenced by nothing in `layerdb`.
    UnreferencedLayer { cache_id: String, bytes: u64 },
    /// Data root exists but metadata could not be read (likely needs root).
    RootUnreadable { path: PathBuf },
    /// The root uses the containerd image store: images and layers live in
    /// containerd's bolt metadata (not disk-parseable), so the inventory
    /// covers only containers, logs, volumes, and build cache.
    ContainerdStorePartial,
    /// A metadata file was corrupt or in an unexpected shape.
    MetadataParse { detail: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_payload_roundtrip() -> Result<(), WeshtatisticError> {
        let mut inventory = DockerInventory {
            root: Some(PathBuf::from("/var/lib/docker")),
            build_cache_bytes: 7,
            collected_at: 1_234_567,
            source: InventorySource::DockerApi,
            ..DockerInventory::default()
        };
        inventory.images.push(ImageInfo {
            id: "abc".into(),
            tags: vec!["repo:tag".into()],
            size_bytes: 10,
            shared_bytes: 4,
            layer_cache_ids: vec!["x".into()],
            layer_count: 1,
            ref_count: 2,
            created: 5,
        });
        inventory
            .warnings
            .push(InventoryWarning::ContainerdStorePartial);
        inventory.compute_totals();

        let payload = inventory.to_extension_payload()?;
        let decoded = DockerInventory::from_extension_payload(&payload);
        assert!(decoded.is_some());
        let decoded = decoded.unwrap_or_default();
        assert_eq!(decoded.source, InventorySource::DockerApi);
        assert_eq!(decoded.collected_at, 1_234_567);
        assert_eq!(decoded.images.len(), 1);
        assert_eq!(decoded.images[0].exclusive_bytes(), 6);
        assert_eq!(decoded.totals.images_bytes, 10);
        assert!(matches!(
            decoded.warnings.first(),
            Some(InventoryWarning::ContainerdStorePartial)
        ));
        Ok(())
    }

    #[test]
    fn test_inventory_payload_forward_tolerance() {
        // Unknown fields are ignored; fields added later default cleanly.
        let json =
            br#"{"images":[{"id":"i1","size_bytes":3,"future_field":true}],"future_top":42}"#;
        let decoded = DockerInventory::from_extension_payload(json);
        assert!(decoded.is_some());
        let decoded = decoded.unwrap_or_default();
        assert_eq!(decoded.images.len(), 1);
        assert_eq!(decoded.images[0].id, "i1");
        assert_eq!(decoded.images[0].size_bytes, 3);
        assert_eq!(decoded.collected_at, 0);
        assert!(matches!(decoded.source, InventorySource::DiskOverlay2));

        assert!(DockerInventory::from_extension_payload(b"not json").is_none());
    }
}

#[cfg(test)]
mod env_tests {
    use super::*;

    #[test]
    fn test_has_docker_covers_socket_only_environments() {
        // A bind-mounted daemon socket with no visible data root (e.g. a
        // container workspace) must still count as Docker presence.
        let mut env = DockerEnvironment::default();
        assert!(env.is_empty());
        assert!(!env.has_docker());

        env.daemon_socket = Some(PathBuf::from("/var/run/docker.sock"));
        assert!(env.is_empty());
        assert!(env.has_docker());
    }
}
