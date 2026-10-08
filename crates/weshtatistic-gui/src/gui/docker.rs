//! Docker disk-space tab.
//!
//! The tab renders from the snapshot: an inventory collected at scan time or
//! by a previous "Analyze"/"Refresh" lives in the snapshot as the
//! `EXT_DOCKER_INVENTORY` extension ([`crate::extensions`]) and is decoded via
//! [`FileArenaSnapshot::docker_inventory`]. Refreshing collects a new
//! inventory on a worker thread, refines it against the scanned tree, and
//! republishes it into the current snapshot — after which rendering is pure
//! snapshot logic like every other tab.
//!
//! The environment probe (data roots / VM disks) stays live: it describes the
//! machine, not the snapshot. Collection is native-only in practice — wasm
//! backends report no [`DockerCollector`], so no collect button is offered
//! there. Per-resource deletion (images, containers, volumes) is the one live
//! write path: it runs the daemon `DELETE` endpoints on a background thread
//! via [`DockerCollector`] and re-collects the inventory on success. All
//! three sub-tables are egui-table-kit tables (the same framework as the
//! explorer panel), each with a slim operations toolbar and a context menu
//! rendering the docker-scoped `TableOperations` registry:
//! [`DockerImagesProvider`] backs the tables and
//! `crate::gui::operations::DockerDelete*Op` open the shared confirmation
//! modal. Deletion is native-only, so the operations are gated on a live
//! collector.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;
use egui_table_kit::{
    error::TableError,
    interaction::RowEventKind,
    operations::{BorrowedRow, HeaderIter, RowCallback, TableCell, TableProvider},
    state::TableState,
    table::TableKit,
};
use fluent_zero::t;

use crate::arena::FileArenaSnapshot;
use crate::docker::{
    DockerCollector, DockerEnvironment, DockerInventory, ImageDeletion, InventorySource,
    InventoryWarning, RootScope, StorageKind,
};
use crate::extensions::{EXT_DOCKER_INVENTORY, EXT_DOCKER_INVENTORY_VERSION};

/// `TableState::id`s of the three sub-tables; the delete operations are
/// scoped to their table through these (the docker ops registry is rendered
/// only from the docker tables, and the ids are the ops' second line of
/// defense).
pub(crate) const DOCKER_IMAGES_TABLE_ID: &str = "docker_images_table";
pub(crate) const DOCKER_CONTAINERS_TABLE_ID: &str = "docker_containers_table";
pub(crate) const DOCKER_VOLUMES_TABLE_ID: &str = "docker_volumes_table";

/// Table column carrying a resource's daemon key in its hover text (full
/// image/container ID); the delete operations resolve it from here.
pub(crate) const DOCKER_COL_ID: usize = 1;

/// Volumes table column carrying the volume name (the daemon key itself).
pub(crate) const DOCKER_VOLUME_COL_NAME: usize = 0;

/// Container logs at or above this size are highlighted in the containers table.
const LARGE_LOG_BYTES: u64 = 100 * 1024 * 1024;

/// Direct children of a Docker data root that qualify for the explorer badge.
const BADGE_AREAS: [&str; 4] = ["overlay2", "volumes", "containers", "buildkit"];

/// Which results table the Docker tab shows.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DockerSection {
    #[default]
    Images,
    Containers,
    Volumes,
}

impl DockerSection {
    /// The sub-table's `TableState::id` (also the ops' scope key).
    pub(crate) const fn table_id(self) -> &'static str {
        match self {
            Self::Images => DOCKER_IMAGES_TABLE_ID,
            Self::Containers => DOCKER_CONTAINERS_TABLE_ID,
            Self::Volumes => DOCKER_VOLUMES_TABLE_ID,
        }
    }

    /// Column sorted descending until the user picks a sort: image size,
    /// container writable-layer size, volume size.
    const fn default_sort_col(self) -> usize {
        match self {
            Self::Images | Self::Containers => 2,
            Self::Volumes => 1,
        }
    }

    /// Number of columns the section's table renders.
    const fn column_count(self) -> usize {
        match self {
            Self::Images => 8,
            Self::Containers => 5,
            Self::Volumes => 4,
        }
    }

    /// Index of this section's delete op inside the docker `TableOperations`
    /// group (the toolbar/menu render exactly this section's op).
    const fn op_index(self) -> usize {
        match self {
            Self::Images => 0,
            Self::Containers => 1,
            Self::Volumes => 2,
        }
    }
}

/// Which daemon resource a deletion targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockerResourceKind {
    Image,
    Container,
    Volume,
}

impl DockerResourceKind {
    /// The resource's sub-table id (op scope key).
    pub(crate) const fn table_id(self) -> &'static str {
        match self {
            Self::Image => DOCKER_IMAGES_TABLE_ID,
            Self::Container => DOCKER_CONTAINERS_TABLE_ID,
            Self::Volume => DOCKER_VOLUMES_TABLE_ID,
        }
    }

    /// i18n key for the operation/modal title ("Delete Container" etc.).
    pub(crate) const fn title_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-title",
            Self::Container => "docker-delete-container",
            Self::Volume => "docker-delete-volume",
        }
    }

    /// i18n key for the modal title when several resources are selected.
    pub(crate) const fn title_multi_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-title-multi",
            Self::Container => "docker-delete-container-multi",
            Self::Volume => "docker-delete-volume-multi",
        }
    }

    /// i18n key for the busy label used while a deletion runs.
    pub(crate) const fn busy_key(self) -> &'static str {
        match self {
            Self::Image => "docker-deleting",
            Self::Container => "docker-deleting-container",
            Self::Volume => "docker-deleting-volume",
        }
    }

    /// i18n key for the modal body line.
    pub(crate) const fn confirm_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-confirm",
            Self::Container => "docker-delete-container-confirm",
            Self::Volume => "docker-delete-volume-confirm",
        }
    }

    /// i18n key for the modal body line when several resources are selected.
    pub(crate) const fn confirm_multi_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-confirm-multi",
            Self::Container => "docker-delete-container-confirm-multi",
            Self::Volume => "docker-delete-volume-confirm-multi",
        }
    }

    /// i18n key for the modal warning line.
    pub(crate) const fn warning_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-warning",
            Self::Container => "docker-delete-container-warning",
            Self::Volume => "docker-delete-volume-warning",
        }
    }

    /// i18n key for the force-checkbox hover text.
    pub(crate) const fn force_hover_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-force-hover",
            Self::Container => "docker-delete-container-force-hover",
            Self::Volume => "docker-delete-volume-force-hover",
        }
    }

    /// i18n key for the success toast when several resources were deleted.
    pub(crate) const fn done_multi_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-done-multi",
            Self::Container => "docker-delete-container-done-multi",
            Self::Volume => "docker-delete-volume-done-multi",
        }
    }

    /// i18n key for the success toast.
    pub(crate) const fn done_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-done",
            Self::Container => "docker-delete-container-done",
            Self::Volume => "docker-delete-volume-done",
        }
    }

    /// i18n key for the failure toast.
    pub(crate) const fn failed_key(self) -> &'static str {
        match self {
            Self::Image => "docker-delete-failed",
            Self::Container => "docker-delete-container-failed",
            Self::Volume => "docker-delete-volume-failed",
        }
    }
}

type InventoryResult = Result<DockerInventory, crate::WeshtatisticError>;

/// Result of one background resource deletion: images additionally report
/// what the daemon untagged/deleted; containers and volumes only confirm.
/// Errors carry the daemon's verbatim rejection (e.g. "image is being used by
/// stopped container ...").
type DeleteResourceResult = Result<Option<ImageDeletion>, crate::WeshtatisticError>;

/// Max resource names listed in the multi-delete modal before collapsing
/// into an "...and N more" line.
const MODAL_NAME_CAP: usize = 8;

/// Max characters of a daemon error message shown in a toast.
const TOAST_ERROR_CAP: usize = 100;

/// Splits `names` into the head shown in the multi-delete modal and how many
/// more remain past the cap.
pub(crate) fn modal_name_head(names: &[String]) -> (&[String], usize) {
    let shown = names.len().min(MODAL_NAME_CAP);
    (&names[..shown], names.len() - shown)
}

/// Caps daemon error text for toasts (appending `...` when truncated).
pub(crate) fn cap_error_text(message: &str, max_chars: usize) -> String {
    if message.chars().count() <= max_chars {
        return message.to_string();
    }
    let head: String = message.chars().take(max_chars).collect();
    format!("{head}...")
}

/// A Docker resource queued for deletion: the daemon-facing key plus the
/// display data the modal and the completion toast need.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockerDeleteTarget {
    pub(crate) kind: DockerResourceKind,
    /// Daemon key: image/container ID, or the volume name.
    pub(crate) id: String,
    /// Display name (first image tag, container name, volume name).
    pub(crate) name: String,
    /// Primary size: image size, container writable layer, volume size.
    pub(crate) size_bytes: u64,
    /// Secondary size: container log bytes (0 for the other kinds).
    pub(crate) extra_bytes: u64,
}

impl DockerDeleteTarget {
    /// Resolves a deletion target from the snapshot-carried inventory by its
    /// daemon key (`key` is an image/container ID, or a volume name).
    pub(crate) fn resolve(
        kind: DockerResourceKind,
        key: &str,
        inventory: &DockerInventory,
    ) -> Option<Self> {
        match kind {
            DockerResourceKind::Image => {
                inventory.images.iter().find(|i| i.id == key).map(|i| Self {
                    kind,
                    id: i.id.clone(),
                    name: i.display_name().to_string(),
                    size_bytes: i.size_bytes,
                    extra_bytes: 0,
                })
            }
            DockerResourceKind::Container => {
                inventory
                    .containers
                    .iter()
                    .find(|c| c.id == key)
                    .map(|c| Self {
                        kind,
                        id: c.id.clone(),
                        name: c.name.clone(),
                        size_bytes: c.rw_size_bytes,
                        extra_bytes: c.log_bytes,
                    })
            }
            DockerResourceKind::Volume => {
                inventory
                    .volumes
                    .iter()
                    .find(|v| v.name == key)
                    .map(|v| Self {
                        kind,
                        id: v.name.clone(),
                        name: v.name.clone(),
                        size_bytes: v.size_bytes,
                        extra_bytes: 0,
                    })
            }
        }
    }
}

/// Aggregate outcome of a (possibly multi-target) background deletion, kept
/// in submission order.
pub(crate) struct DockerDeletionSummary {
    pub(crate) results: Vec<(DockerDeleteTarget, DeleteResourceResult)>,
}

impl DockerDeletionSummary {
    /// True when no targets were queued (defensive; `begin` rejects empty).
    pub(crate) const fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Number of targets in the batch.
    pub(crate) const fn len(&self) -> usize {
        self.results.len()
    }

    /// Number of targets the daemon deleted successfully.
    pub(crate) fn success_count(&self) -> usize {
        self.results
            .iter()
            .filter(|(_, result)| result.is_ok())
            .count()
    }

    /// `(name, verbatim daemon message)` for each failed target, in order.
    pub(crate) fn failures(&self) -> Vec<(String, String)> {
        self.results
            .iter()
            .filter_map(|(target, result)| {
                result
                    .as_ref()
                    .err()
                    .map(|err| (target.name.clone(), err.to_string()))
            })
            .collect()
    }

    /// Sums of untagged/deleted entries across successful image deletions.
    pub(crate) fn image_totals(&self) -> (usize, usize) {
        self.results
            .iter()
            .fold((0, 0), |(untagged, deleted), (_, result)| {
                let Ok(Some(deletion)) = result else {
                    return (untagged, deleted);
                };
                (
                    untagged + deletion.untagged.len(),
                    deleted + deletion.deleted.len(),
                )
            })
    }

    /// The batch's resource kind — every target of a batch comes from the
    /// same table, hence shares the kind.
    pub(crate) fn kind(&self) -> Option<DockerResourceKind> {
        self.results.first().map(|(target, _)| target.kind)
    }
}

/// State of a background resource deletion (mirrors the collection pattern:
/// a `running` flag plus slots the render loop drains). Targets delete
/// sequentially on one worker thread, continuing past individual failures.
#[derive(Default)]
pub(crate) struct DockerDeletionState {
    /// True while the background deletion thread is running.
    running: Arc<AtomicBool>,
    /// Targets of the in-flight deletion (`None` when idle or finished).
    inflight: Arc<parking_lot::RwLock<Option<Vec<DockerDeleteTarget>>>>,
    /// Completed batch not yet consumed by the render loop.
    pending: Arc<parking_lot::RwLock<Option<DockerDeletionSummary>>>,
}

impl DockerDeletionState {
    /// True while a deletion thread is running.
    #[must_use]
    fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Runs the deletions on a background thread, sequentially, continuing
    /// past individual failures. Returns false (starting nothing) when a
    /// deletion is already running or no targets were given.
    #[must_use]
    fn begin(
        &self,
        collector: Arc<dyn DockerCollector>,
        targets: Vec<DockerDeleteTarget>,
        force: bool,
        ctx: &egui::Context,
    ) -> bool {
        if targets.is_empty() || self.running.swap(true, Ordering::SeqCst) {
            return false;
        }
        *self.inflight.write() = Some(targets.clone());
        *self.pending.write() = None;

        let running = self.running.clone();
        let inflight = self.inflight.clone();
        let pending = self.pending.clone();
        let ctx = ctx.clone();

        #[cfg(not(target_family = "wasm"))]
        std::thread::spawn(move || {
            let results = targets
                .iter()
                .map(|target| {
                    let result = match target.kind {
                        DockerResourceKind::Image => {
                            collector.delete_image(&target.id, force).map(Some)
                        }
                        DockerResourceKind::Container => {
                            collector.delete_container(&target.id, force).map(|()| None)
                        }
                        DockerResourceKind::Volume => {
                            collector.delete_volume(&target.id, force).map(|()| None)
                        }
                    };
                    (target.clone(), result)
                })
                .collect();
            *inflight.write() = None;
            *pending.write() = Some(DockerDeletionSummary { results });
            running.store(false, Ordering::Release);
            ctx.request_repaint();
        });

        #[cfg(target_family = "wasm")]
        {
            // Unreachable: wasm backends report no collector, so the delete
            // ops are never enabled. Keep the state consistent regardless.
            let _ = (collector, targets, force, inflight, pending, ctx);
            running.store(false, Ordering::SeqCst);
        }
        true
    }

    /// Drains a finished batch, if one has landed since the last drain.
    fn take(&self) -> Option<DockerDeletionSummary> {
        self.pending.write().take()
    }

    /// Shares the running flag (e.g. with the delete `TableOperation`s, whose
    /// enablement lives outside this view state).
    pub(crate) fn running_flag(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}

/// GUI state for the Docker tab, owned by `GuiApp` (mirrors the deduplicator
/// tab's "state lives on the app" pattern).
pub(crate) struct DockerViewState {
    /// Lazily probed environment (first tab render or badge lookup).
    environment: Option<Arc<DockerEnvironment>>,
    /// Whether the one-shot environment probe has run.
    probed: bool,
    /// Precomputed explorer-badge lookup derived from `environment`.
    badge_context: Option<Arc<DockerBadgeContext>>,
    /// Latest background collection result not yet reflected in the snapshot:
    /// `Ok` is published as the `EXT_DOCKER_INVENTORY` extension on the next
    /// render; `Err` stays visible until the next collection starts.
    pending: Arc<parking_lot::RwLock<Option<InventoryResult>>>,
    /// Cancel flag for the background collection thread.
    cancel: Arc<AtomicBool>,
    /// True while the background collection thread is running.
    pub(crate) running: Arc<AtomicBool>,
    /// Background resource deletion (running flag + result slot).
    pub(crate) deletion: DockerDeletionState,
    /// The "force" checkbox in the delete confirmation modal.
    pub(crate) delete_force: bool,
    /// egui-table-kit states of the three sub-tables (selection, sort,
    /// filters), one per section.
    pub(crate) images_state: TableState,
    pub(crate) containers_state: TableState,
    pub(crate) volumes_state: TableState,
    /// Row counts the table states were last synced against; a change marks
    /// the view dirty and clears the (index-based) selection.
    images_row_count: usize,
    containers_row_count: usize,
    volumes_row_count: usize,
    /// Selected results sub-section.
    section: DockerSection,
}

impl DockerViewState {
    /// The section's table state.
    pub(crate) const fn table_state_mut(&mut self, section: DockerSection) -> &mut TableState {
        match section {
            DockerSection::Images => &mut self.images_state,
            DockerSection::Containers => &mut self.containers_state,
            DockerSection::Volumes => &mut self.volumes_state,
        }
    }

    /// The row count the section's table state was last synced against.
    const fn row_count_mut(&mut self, section: DockerSection) -> &mut usize {
        match section {
            DockerSection::Images => &mut self.images_row_count,
            DockerSection::Containers => &mut self.containers_row_count,
            DockerSection::Volumes => &mut self.volumes_row_count,
        }
    }
}

impl Default for DockerViewState {
    fn default() -> Self {
        Self {
            environment: None,
            probed: false,
            badge_context: None,
            pending: Arc::new(parking_lot::RwLock::new(None)),
            cancel: Arc::new(AtomicBool::new(false)),
            running: Arc::new(AtomicBool::new(false)),
            deletion: DockerDeletionState::default(),
            delete_force: false,
            images_state: TableState::new(DOCKER_IMAGES_TABLE_ID, 0),
            containers_state: TableState::new(DOCKER_CONTAINERS_TABLE_ID, 0),
            volumes_state: TableState::new(DOCKER_VOLUMES_TABLE_ID, 0),
            images_row_count: 0,
            containers_row_count: 0,
            volumes_row_count: 0,
            section: DockerSection::default(),
        }
    }
}

/// egui-table-kit [`TableProvider`] over a Docker inventory sub-section.
///
/// Rows are flat (no hierarchy) and indexed directly into the section's vec.
/// Only Images is rendered through the kit today; the Containers/Volumes
/// mappings are implemented as well so those sub-tabs can migrate the same
/// way later. Size-like columns sort numerically, text columns
/// lexicographically (the kit's default sort is string-based, which would
/// order "9 MB" above "10 MB").
pub(crate) struct DockerImagesProvider<'a> {
    inventory: &'a DockerInventory,
    section: DockerSection,
    time_format: crate::time_utils::TimeFormat,
}

impl<'a> DockerImagesProvider<'a> {
    #[must_use]
    pub(crate) const fn new(
        inventory: &'a DockerInventory,
        section: DockerSection,
        time_format: crate::time_utils::TimeFormat,
    ) -> Self {
        Self {
            inventory,
            section,
            time_format,
        }
    }

    /// Formats an epoch-seconds created timestamp per the user's time
    /// format (0 renders as the localized "unknown" via `format_epoch`).
    fn created_label(&self, created: u64) -> String {
        crate::time_utils::format_epoch(
            u32::try_from(created).unwrap_or(u32::MAX),
            &self.time_format,
        )
    }

    /// Orders `rows` by `key`, honoring the ascending/descending direction.
    fn sort_rows<T: Ord>(rows: &mut [usize], ascending: bool, key: impl Fn(usize) -> T) {
        rows.sort_by(|a, b| {
            let ord = key(*a).cmp(&key(*b));
            if ascending { ord } else { ord.reverse() }
        });
    }
}

impl TableProvider for DockerImagesProvider<'_> {
    fn column_count(&self) -> usize {
        match self.section {
            DockerSection::Images => 8,
            DockerSection::Containers => 5,
            DockerSection::Volumes => 4,
        }
    }

    fn header(&self, index: usize) -> Option<std::borrow::Cow<'_, str>> {
        let key = match (self.section, index) {
            (_, 0) => "explorer-hdr-name",
            (DockerSection::Images | DockerSection::Containers, 1) => "docker-hdr-id",
            (DockerSection::Images, 2) | (DockerSection::Volumes, 1) => "explorer-hdr-size",
            (DockerSection::Images, 3) => "docker-hdr-shared",
            (DockerSection::Images, 4) => "docker-hdr-exclusive",
            (DockerSection::Images, 5) => "docker-hdr-layers",
            (DockerSection::Images, 6)
            | (DockerSection::Containers, 4)
            | (DockerSection::Volumes, 3) => "docker-hdr-created",
            (DockerSection::Images, 7) | (DockerSection::Volumes, 2) => "docker-hdr-refs",
            (DockerSection::Containers, 2) => "docker-hdr-rw",
            (DockerSection::Containers, 3) => "docker-hdr-log",
            _ => return None,
        };
        Some(t!(key))
    }

    fn headers(&self) -> HeaderIter<'_> {
        HeaderIter::new(self)
    }

    fn row_count(&self) -> usize {
        match self.section {
            DockerSection::Images => self.inventory.images.len(),
            DockerSection::Containers => self.inventory.containers.len(),
            DockerSection::Volumes => self.inventory.volumes.len(),
        }
    }

    fn cell_at(
        &self,
        row_index: usize,
        col_index: usize,
    ) -> Result<Option<TableCell<'_>>, TableError> {
        let cell: TableCell<'_> = match self.section {
            DockerSection::Images => {
                let Some(image) = self.inventory.images.get(row_index) else {
                    return Ok(None);
                };
                match col_index {
                    // Hover carries the full name/ID; display stays compact.
                    0 => (
                        std::borrow::Cow::Borrowed(image.display_name()),
                        Some(std::borrow::Cow::Borrowed(image.display_name())),
                    ),
                    1 => (
                        std::borrow::Cow::Borrowed(image.short_id()),
                        Some(std::borrow::Cow::Borrowed(image.id.as_str())),
                    ),
                    2 => (std::borrow::Cow::Owned(format_size(image.size_bytes)), None),
                    3 => (
                        std::borrow::Cow::Owned(format_size(image.shared_bytes)),
                        None,
                    ),
                    4 => (
                        std::borrow::Cow::Owned(format_size(image.exclusive_bytes())),
                        None,
                    ),
                    // `layer_count` (not the cache-id vec) so API-sourced
                    // inventories report real counts.
                    5 => (std::borrow::Cow::Owned(image.layer_count.to_string()), None),
                    6 => (
                        std::borrow::Cow::Owned(self.created_label(image.created)),
                        None,
                    ),
                    7 => (std::borrow::Cow::Owned(image.ref_count.to_string()), None),
                    _ => return Ok(None),
                }
            }
            DockerSection::Containers => {
                let Some(container) = self.inventory.containers.get(row_index) else {
                    return Ok(None);
                };
                match col_index {
                    0 => (
                        std::borrow::Cow::Borrowed(container.name.as_str()),
                        Some(std::borrow::Cow::Borrowed(container.name.as_str())),
                    ),
                    1 => (
                        std::borrow::Cow::Borrowed(container.short_id()),
                        Some(std::borrow::Cow::Borrowed(container.id.as_str())),
                    ),
                    2 => (
                        std::borrow::Cow::Owned(format_size(container.rw_size_bytes)),
                        None,
                    ),
                    3 => (
                        std::borrow::Cow::Owned(format_size(container.log_bytes)),
                        None,
                    ),
                    4 => (
                        std::borrow::Cow::Owned(self.created_label(container.created)),
                        None,
                    ),
                    _ => return Ok(None),
                }
            }
            DockerSection::Volumes => {
                let Some(volume) = self.inventory.volumes.get(row_index) else {
                    return Ok(None);
                };
                match col_index {
                    0 => (
                        std::borrow::Cow::Borrowed(volume.name.as_str()),
                        Some(std::borrow::Cow::Borrowed(volume.name.as_str())),
                    ),
                    1 => (
                        std::borrow::Cow::Owned(format_size(volume.size_bytes)),
                        None,
                    ),
                    2 => (std::borrow::Cow::Owned(volume.ref_count.to_string()), None),
                    3 => (
                        std::borrow::Cow::Owned(self.created_label(volume.created)),
                        None,
                    ),
                    _ => return Ok(None),
                }
            }
        };
        Ok(Some(cell))
    }

    fn row_at(
        &self,
        index: usize,
    ) -> Result<Option<egui_table_kit::operations::OwnedRow>, TableError> {
        if index >= self.row_count() {
            return Ok(None);
        }
        let mut cells = Vec::with_capacity(self.column_count());
        for col_idx in 0..self.column_count() {
            if let Some((val, hover)) = self.cell_at(index, col_idx)? {
                cells.push((
                    compact_str::CompactString::from(val.as_ref()),
                    hover.map(|h| compact_str::CompactString::from(h.as_ref())),
                ));
            } else {
                cells.push((compact_str::CompactString::default(), None));
            }
        }
        Ok(Some(egui_table_kit::operations::OwnedRow { cells }))
    }

    fn for_selected_rows(
        &self,
        state: &TableState,
        f: &mut RowCallback<'_>,
    ) -> Result<(), TableError> {
        for row_idx in &state.selected_rows {
            if (row_idx as usize) < self.row_count() {
                f(&BorrowedRow {
                    provider: self,
                    row_index: row_idx as usize,
                })?;
            }
        }
        Ok(())
    }

    fn for_all_rows(&self, f: &mut RowCallback<'_>) -> Result<(), TableError> {
        for row_idx in 0..self.row_count() {
            f(&BorrowedRow {
                provider: self,
                row_index: row_idx,
            })?;
        }
        Ok(())
    }

    fn sort_active_rows(
        &self,
        active_rows: &mut Vec<usize>,
        col_index: usize,
        ascending: bool,
    ) -> Result<(), TableError> {
        match self.section {
            DockerSection::Images => {
                let images = &self.inventory.images;
                match col_index {
                    0 => Self::sort_rows(active_rows, ascending, |i| images[i].display_name()),
                    1 => Self::sort_rows(active_rows, ascending, |i| images[i].id.as_str()),
                    2 => Self::sort_rows(active_rows, ascending, |i| images[i].size_bytes),
                    3 => Self::sort_rows(active_rows, ascending, |i| images[i].shared_bytes),
                    4 => Self::sort_rows(active_rows, ascending, |i| images[i].exclusive_bytes()),
                    5 => Self::sort_rows(active_rows, ascending, |i| images[i].layer_count),
                    6 => Self::sort_rows(active_rows, ascending, |i| images[i].created),
                    7 => Self::sort_rows(active_rows, ascending, |i| images[i].ref_count),
                    _ => {}
                }
            }
            DockerSection::Containers => {
                let containers = &self.inventory.containers;
                match col_index {
                    0 => Self::sort_rows(active_rows, ascending, |i| containers[i].name.as_str()),
                    1 => Self::sort_rows(active_rows, ascending, |i| containers[i].id.as_str()),
                    2 => Self::sort_rows(active_rows, ascending, |i| containers[i].rw_size_bytes),
                    3 => Self::sort_rows(active_rows, ascending, |i| containers[i].log_bytes),
                    4 => Self::sort_rows(active_rows, ascending, |i| containers[i].created),
                    _ => {}
                }
            }
            DockerSection::Volumes => {
                let volumes = &self.inventory.volumes;
                match col_index {
                    0 => Self::sort_rows(active_rows, ascending, |i| volumes[i].name.as_str()),
                    1 => Self::sort_rows(active_rows, ascending, |i| volumes[i].size_bytes),
                    2 => Self::sort_rows(active_rows, ascending, |i| volumes[i].ref_count),
                    3 => Self::sort_rows(active_rows, ascending, |i| volumes[i].created),
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

/// Classifies an explorer path for the Docker badge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DockerBadgeKind {
    /// A Docker data root itself.
    Root,
    /// A Docker Desktop-style VM disk image.
    VmDisk,
    /// A direct `overlay2`/`volumes`/`containers`/`buildkit` child of a root.
    Area(&'static str),
}

/// Exact-path lookup backing the explorer's Docker badge. Built once from the
/// probed environment and shared with the explorer via `GuiApp`.
pub(crate) struct DockerBadgeContext {
    entries: Vec<(String, DockerBadgeKind)>,
}

impl DockerBadgeContext {
    #[must_use]
    pub fn from_environment(environment: &DockerEnvironment) -> Self {
        let mut entries = Vec::new();
        for root in &environment.data_roots {
            let root_path = normalize_badge_path(&root.path.to_string_lossy());
            entries.push((root_path.clone(), DockerBadgeKind::Root));
            for area in BADGE_AREAS {
                entries.push((format!("{root_path}/{area}"), DockerBadgeKind::Area(area)));
            }
        }
        for disk in &environment.vm_disks {
            entries.push((
                normalize_badge_path(&disk.path.to_string_lossy()),
                DockerBadgeKind::VmDisk,
            ));
        }
        Self { entries }
    }

    /// Returns the badge icon and localized tooltip for a node full path when
    /// it is a Docker data root, VM disk, or one of the badge-eligible root
    /// children.
    #[must_use]
    pub fn badge_for(&self, full_path: &str) -> Option<(&'static str, Cow<'static, str>)> {
        let normalized = normalize_badge_path(full_path);
        let (icon, tooltip) = self
            .entries
            .iter()
            .find(|(path, _)| *path == normalized)
            .map(|(_, kind)| match kind {
                DockerBadgeKind::Root => ("📦", t!("badge-docker")),
                DockerBadgeKind::VmDisk => ("📦", t!("badge-docker-vm")),
                DockerBadgeKind::Area(area) => ("📦", t!("badge-docker-area", { "area" => *area })),
            })?;
        Some((icon, tooltip))
    }
}

/// Separator-normalized form (backslashes and UNC prefixes unified, trailing
/// slashes trimmed) used for badge path comparisons.
fn normalize_badge_path(path: &str) -> String {
    let replaced = path.replace('\\', "/");
    let stripped = replaced.strip_prefix("//?/").unwrap_or(&replaced);
    stripped.trim_end_matches('/').to_string()
}

#[must_use]
fn format_size(bytes: u64) -> String {
    prettier_bytes::ByteFormatter::new()
        .format(bytes)
        .to_string()
}

fn storage_kind_label(kind: StorageKind) -> Cow<'static, str> {
    match kind {
        StorageKind::Overlay2 => t!("docker-kind-overlay2"),
        StorageKind::Overlay => t!("docker-kind-overlay"),
        StorageKind::Aufs => t!("docker-kind-aufs"),
        StorageKind::Btrfs => t!("docker-kind-btrfs"),
        StorageKind::Zfs => t!("docker-kind-zfs"),
        StorageKind::Devicemapper => t!("docker-kind-devicemapper"),
        StorageKind::Vfs => t!("docker-kind-vfs"),
        StorageKind::WindowsFilter => t!("docker-kind-windowsfilter"),
        StorageKind::Containerd => t!("docker-kind-containerd"),
        StorageKind::Unknown => t!("docker-kind-unknown"),
    }
}

fn root_scope_label(scope: RootScope) -> Cow<'static, str> {
    match scope {
        RootScope::System => t!("docker-scope-system"),
        RootScope::Rootless => t!("docker-scope-rootless"),
    }
}

/// Short provenance label for how the snapshot-carried inventory was produced.
fn source_label(source: InventorySource) -> Cow<'static, str> {
    match source {
        InventorySource::DiskOverlay2 => t!("docker-source-disk"),
        InventorySource::DiskContainerdPartial => t!("docker-source-partial"),
        InventorySource::DockerApi => t!("docker-source-api"),
    }
}

/// Provenance line for a snapshot-carried inventory: collection time (in the
/// user's preferred format) plus the collection source.
fn draw_provenance(
    ui: &mut egui::Ui,
    inventory: &DockerInventory,
    time_format: &crate::time_utils::TimeFormat,
) {
    let collected = crate::time_utils::format_epoch(
        u32::try_from(inventory.collected_at).unwrap_or(u32::MAX),
        time_format,
    );
    ui.weak(format!(
        "{} — {}",
        t!("docker-collected-at", { "time" => collected }),
        source_label(inventory.source)
    ));
}

/// Refines parser-reported sizes in `inventory` from the scanned tree (when
/// the scan covers the data root): volume sizes, container rw/log sizes and
/// layer sizes are replaced by the on-disk subtree sizes of the matching
/// scanned nodes, and the totals are recomputed. Returns true when any size
/// was refined.
fn refine_inventory_from_snapshot(
    inventory: &mut DockerInventory,
    snapshot: &FileArenaSnapshot,
) -> bool {
    let Some(root) = inventory.root.as_ref() else {
        return false;
    };
    let mut changed = false;

    for volume in &mut inventory.volumes {
        let path = format!("{}/volumes/{}", root.display(), volume.name);
        if let Some(idx) = snapshot.resolve_path_index(&path)
            && snapshot.nodes[idx as usize].is_directory()
        {
            volume.size_bytes = snapshot.nodes[idx as usize].size;
            changed = true;
        }
    }

    for container in &mut inventory.containers {
        let dir = format!("{}/containers/{}", root.display(), container.id);
        let dir_size = snapshot
            .resolve_path_index(&dir)
            .map(|idx| snapshot.nodes[idx as usize].size);
        let log_path = format!("{}/{}-json.log", dir, container.id);
        if let Some(idx) = snapshot.resolve_path_index(&log_path)
            && !snapshot.nodes[idx as usize].is_directory()
        {
            container.log_bytes = snapshot.nodes[idx as usize].size;
            changed = true;
        }
        if let Some(total) = dir_size {
            // The scanned container directory includes the log file; report
            // the writable layer separately to avoid double counting.
            container.rw_size_bytes = total.saturating_sub(container.log_bytes);
            changed = true;
        }
    }

    for layer in &mut inventory.layers {
        let path = if layer.path.is_absolute() {
            layer.path.to_string_lossy().into_owned()
        } else {
            format!("{}/{}", root.display(), layer.path.display())
        };
        if let Some(idx) = snapshot.resolve_path_index(&path)
            && snapshot.nodes[idx as usize].is_directory()
        {
            layer.size_bytes = snapshot.nodes[idx as usize].size;
            changed = true;
        }
    }

    if changed {
        inventory.compute_totals();
    }
    changed
}

impl super::GuiApp {
    /// Runs the one-shot Docker environment probe on first use. Cheap by
    /// contract: well-known location probes only, no tree walks.
    fn ensure_docker_probe(&mut self) {
        if self.docker_view.probed {
            return;
        }
        self.docker_view.probed = true;
        let Some(scanner) = self.scanner.clone() else {
            return;
        };
        let Some(collector) = scanner.docker_collector() else {
            return;
        };
        let environment = Arc::new(collector.detect_environment());
        self.docker_view.badge_context =
            Some(Arc::new(DockerBadgeContext::from_environment(&environment)));
        self.docker_view.environment = Some(environment);
    }

    /// Explorer badge lookup; probes the environment on first use.
    pub(crate) fn docker_badge_context(&mut self) -> Option<Arc<DockerBadgeContext>> {
        self.ensure_docker_probe();
        self.docker_view.badge_context.clone()
    }

    pub(crate) fn render_docker_tab(&mut self, ui: &mut egui::Ui, snapshot: &FileArenaSnapshot) {
        self.ensure_docker_probe();

        if self.docker_view.running.load(Ordering::Acquire)
            || self.docker_view.deletion.is_running()
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }

        ui.label(t!("docker-desc"));
        ui.separator();

        // Merge a completed live collection into the snapshot first; this
        // frame then renders the freshly published snapshot like any other.
        // Row indices changed underneath the table selections: drop them.
        let published = self.publish_pending_inventory();
        if published {
            self.clear_docker_table_selections();
        }
        let fresh;
        let snapshot = if published {
            fresh = self.shared_state.current_snapshot.load();
            &fresh
        } else {
            snapshot
        };

        // Surface a finished background deletion batch: toast the outcome
        // (the daemon's verbatim message on failure — the modal's force
        // checkbox is the remedy), then re-collect so the tab reflects the
        // daemon's new state. A single selected resource keeps the singular
        // per-kind toasts; several use the multi-summary toasts.
        if let Some(batch) = self.docker_view.deletion.take() {
            self.clear_docker_table_selections();
            if batch.is_empty() {
                // Defensive: `begin` rejects empty batches.
            } else if batch.len() == 1 {
                let (target, result) = &batch.results[0];
                match result {
                    Ok(option) => {
                        match target.kind {
                            DockerResourceKind::Image => {
                                let deletion = option.clone().unwrap_or_default();
                                crate::gui::toast_success(t!("docker-delete-done", {
                                    "name" => target.name.as_str(),
                                    "untagged" => deletion.untagged.len(),
                                    "deleted" => deletion.deleted.len()
                                }));
                            }
                            kind => {
                                crate::gui::toast_success(t!(kind.done_key(), {
                                    "name" => target.name.as_str()
                                }));
                            }
                        }
                        self.start_docker_collection();
                    }
                    Err(err) => {
                        let error = err.to_string();
                        crate::gui::toast_error(t!(target.kind.failed_key(), {
                            "error" => error.as_str()
                        }));
                    }
                }
            } else {
                let failures = batch.failures();
                if failures.is_empty() {
                    // Full success.
                    match batch.kind() {
                        Some(DockerResourceKind::Image) => {
                            let (untagged, deleted) = batch.image_totals();
                            crate::gui::toast_success(t!("docker-delete-done-multi", {
                                "count" => batch.len(),
                                "untagged" => untagged,
                                "deleted" => deleted
                            }));
                        }
                        Some(kind) => {
                            crate::gui::toast_success(t!(kind.done_multi_key(), {
                                "count" => batch.len()
                            }));
                        }
                        None => {}
                    }
                    self.start_docker_collection();
                } else {
                    // Partial or total failure: report the counts and the
                    // first (capped) daemon message.
                    let error = cap_error_text(&failures[0].1, TOAST_ERROR_CAP);
                    crate::gui::toast_error(t!("docker-delete-partial-multi", {
                        "succeeded" => batch.success_count(),
                        "count" => batch.len(),
                        "failed" => failures.len(),
                        "error" => error.as_str()
                    }));
                    if batch.success_count() > 0 {
                        self.start_docker_collection();
                    }
                }
            }
        }

        // The environment probe is live machine state, independent of the
        // snapshot: data roots and VM disks render (and the collect button is
        // offered) even when the snapshot itself carries no Docker data.
        let environment = self.docker_view.environment.clone();
        if let Some(environment) = environment.as_ref().filter(|env| !env.is_empty()) {
            draw_docker_environment(ui, environment);
            ui.separator();
        }

        let inventory = snapshot.docker_inventory();
        self.draw_docker_controls(ui, environment.as_deref());

        if inventory.is_some() || self.docker_view.pending.read().is_some() {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.docker_view.section,
                    DockerSection::Images,
                    t!("docker-section-images"),
                );
                ui.selectable_value(
                    &mut self.docker_view.section,
                    DockerSection::Containers,
                    t!("docker-section-containers"),
                );
                ui.selectable_value(
                    &mut self.docker_view.section,
                    DockerSection::Volumes,
                    t!("docker-section-volumes"),
                );
            });
            ui.separator();
        }

        // A failed refresh keeps the error visible; any (older) snapshot data
        // below it remains rendered, since the snapshot is the primary source.
        let pending_error = match self.docker_view.pending.read().as_ref() {
            Some(Err(err)) => Some(err.to_string()),
            _ => None,
        };
        if let Some(err) = &pending_error {
            ui.colored_label(
                crate::colors::WARNING_RED,
                t!("docker-error", { "error" => err.as_str() }),
            );
        }

        // Ops toolbar: refresh plus the active section's delete op. Rendered
        // whenever a collector exists so the first collection is
        // discoverable without any data present.
        let has_collector = self
            .scanner
            .as_ref()
            .is_some_and(|scanner| scanner.docker_collector().is_some());
        if has_collector {
            let section = self.docker_view.section;
            // With no data yet, an empty inventory backs the ops (the
            // refresh op still works — and shows — without rows).
            match inventory.as_ref() {
                Some(inventory) => self.draw_docker_toolbar(ui, inventory, section),
                None => self.draw_docker_toolbar(ui, &DockerInventory::default(), section),
            }
            ui.add_space(2.0);
        }

        if let Some(inventory) = inventory {
            draw_provenance(ui, &inventory, &self.time_format);
            draw_summary_band(ui, &inventory);
            let section = self.docker_view.section;
            // Warnings go above the table so the table's remaining-height
            // budget below is honest (they're capped at 140px internally).
            if !inventory.warnings.is_empty() {
                ui.add_space(4.0);
                draw_warnings(ui, &inventory.warnings);
            }
            ui.add_space(4.0);
            self.draw_docker_table(ui, &inventory, section);
        } else if pending_error.is_none() {
            // No inventory anywhere: distinguish "this machine has no Docker"
            // (probe ran and found nothing) from "snapshot carries no Docker
            // data" (covers the wasm viewer and headless-loaded snapshots).
            // No collector at all (wasm) cannot probe, so it lands in the
            // snapshot-data message rather than "no installation". Sandboxed
            // macOS builds get their own explanation: the sandbox blocks every
            // Docker access path, but snapshot review still works.
            let no_install = environment.as_ref().is_some_and(|env| !env.has_docker());
            ui.centered_and_justified(|ui| {
                if super::operations::is_macos_sandbox() {
                    ui.label(t!("docker-sandbox-unavailable"));
                } else if no_install {
                    ui.label(t!("docker-no-install"));
                } else {
                    ui.label(t!("docker-no-snapshot-data"));
                }
            });
        }
    }

    fn draw_docker_controls(&self, ui: &mut egui::Ui, environment: Option<&DockerEnvironment>) {
        let running = self.docker_view.running.load(Ordering::Acquire);
        // Collection works with an accessible overlay2/containerd data root
        // (disk parse) OR a daemon socket (API) — the socket alone suffices,
        // e.g. when it is bind-mounted into a container with no local root.
        let can_collect = environment.is_some_and(|env| {
            env.daemon_socket.is_some()
                || env.data_roots.iter().any(|root| {
                    root.accessible
                        && matches!(root.kind, StorageKind::Overlay2 | StorageKind::Containerd)
                })
        });

        if running {
            // Interactive spinner: turns into a red cancel X on hover (dedup pattern)
            ui.horizontal_wrapped(|ui| {
                let spinner_size = 18.0;
                let (rect, mut response) = ui.allocate_exact_size(
                    egui::vec2(spinner_size, spinner_size),
                    egui::Sense::click(),
                );

                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    response = response.on_hover_text(t!("docker-cancel-hover"));

                    let stroke = egui::Stroke::new(2.0f32, crate::colors::WARNING_RED);
                    let inset = 4.0;
                    ui.painter().line_segment(
                        [
                            rect.left_top() + egui::vec2(inset, inset),
                            rect.right_bottom() - egui::vec2(inset, inset),
                        ],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [
                            rect.right_top() + egui::vec2(-inset, inset),
                            rect.left_bottom() - egui::vec2(-inset, inset),
                        ],
                        stroke,
                    );

                    if response.clicked() {
                        self.docker_view.cancel.store(true, Ordering::SeqCst);
                    }
                } else {
                    ui.put(rect, egui::Spinner::new().size(spinner_size));
                }

                ui.label(t!("docker-analyzing"));
            });
        }
        // Note: collection is started from the docker toolbar's refresh op
        // (`DockerRefreshOp`), not a standalone button here.

        if let Some(environment) = environment
            && !can_collect
            && !environment.data_roots.is_empty()
        {
            // Two distinct situations with different remedies: no readable
            // root (privileges) vs. a readable root in a layout we cannot
            // yet parse (e.g. btrfs, zfs, containerd store).
            let any_accessible = environment.data_roots.iter().any(|root| root.accessible);
            let message = if any_accessible {
                t!("docker-unsupported-layout")
            } else {
                t!("docker-needs-elevated")
            };
            ui.colored_label(crate::colors::COLOR_WARNING_YELLOW, message);
        }
    }

    /// Starts the background resource deletion through the Docker collector;
    /// the confirmation modal has already collected consent and the force
    /// flag.
    pub(crate) fn start_docker_resource_deletion(
        &self,
        targets: Vec<DockerDeleteTarget>,
        force: bool,
        ctx: &egui::Context,
    ) {
        let Some(scanner) = self.scanner.clone() else {
            return;
        };
        let Some(collector) = scanner.docker_collector() else {
            return;
        };
        let _ = self
            .docker_view
            .deletion
            .begin(collector, targets, force, ctx);
    }

    /// Drops the selections of all three sub-tables (row indices remap after
    /// every re-collection and every deletion).
    fn clear_docker_table_selections(&mut self) {
        self.docker_view.images_state.selected_rows.clear();
        self.docker_view.containers_state.selected_rows.clear();
        self.docker_view.volumes_state.selected_rows.clear();
    }

    pub(crate) fn start_docker_collection(&self) {
        let Some(scanner) = self.scanner.clone() else {
            return;
        };
        let Some(collector) = scanner.docker_collector() else {
            return;
        };

        self.docker_view.cancel.store(false, Ordering::SeqCst);
        *self.docker_view.pending.write() = None;

        let cancel = self.docker_view.cancel.clone();
        let running = self.docker_view.running.clone();
        let slot = self.docker_view.pending.clone();
        running.store(true, Ordering::SeqCst);

        #[cfg(not(target_family = "wasm"))]
        std::thread::spawn(move || {
            let result = collector.collect_inventory(&cancel);
            *slot.write() = Some(result);
            running.store(false, Ordering::Release);
        });

        #[cfg(target_family = "wasm")]
        {
            // Unreachable: wasm backends report no collector, so the Analyze
            // button is never shown. Keep the state consistent regardless.
            let _ = (collector, cancel, slot);
            running.store(false, Ordering::SeqCst);
        }
    }

    /// Merges a completed live collection into the snapshot as the
    /// `EXT_DOCKER_INVENTORY` extension, preserving any other extensions.
    /// Returns true when the snapshot was republished and this frame should
    /// re-read it. Errors are left in `pending` for display; an empty pending
    /// slot is a cheap no-op on every frame.
    fn publish_pending_inventory(&self) -> bool {
        let Some(result) = self.docker_view.pending.write().take() else {
            return false;
        };
        let mut inventory = match result {
            Ok(inventory) => inventory,
            Err(err) => {
                // Collection failed: keep the error visible until the next
                // collection starts.
                *self.docker_view.pending.write() = Some(Err(err));
                return false;
            }
        };

        // Reconcile with the scanned tree before publishing (this replaces
        // the old live-slot refinement): parser-reported sizes get replaced
        // by on-disk subtree sizes wherever the scan covers the data root.
        // Inventories already carried by the snapshot are collect-time
        // accurate and are never re-refined.
        let snapshot = self.shared_state.current_snapshot.load();
        if !snapshot.nodes.is_empty() {
            refine_inventory_from_snapshot(&mut inventory, &snapshot);
        }

        // Stamp the provenance the disk collector does not set: collection
        // time, and the containerd-store source implied by its warning.
        if inventory.collected_at == 0 {
            inventory.collected_at = u64::from(crate::time_utils::system_time_to_unix_timestamp(
                std::time::SystemTime::now(),
            ));
        }
        if inventory
            .warnings
            .iter()
            .any(|w| matches!(w, InventoryWarning::ContainerdStorePartial))
        {
            inventory.source = InventorySource::DiskContainerdPartial;
        }

        let payload = match inventory.to_extension_payload() {
            Ok(payload) => payload,
            Err(err) => {
                *self.docker_view.pending.write() = Some(Err(err));
                return false;
            }
        };

        let mut new_snapshot = (**snapshot).clone();
        new_snapshot.extensions.insert(
            EXT_DOCKER_INVENTORY,
            EXT_DOCKER_INVENTORY_VERSION,
            payload.into(),
        );
        self.shared_state.store_snapshot(new_snapshot);
        true
    }
}

/// Collapsed-by-default environment section; state persists across frames
/// via the stable `id` (egui memory). The header keeps the strong title
/// styling and summarizes the entry count.
fn env_collapsing_header(
    ui: &mut egui::Ui,
    id: &'static str,
    title: Cow<'static, str>,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::collapsing_header::CollapsingResponse<()> {
    let response = egui::CollapsingHeader::new(egui::RichText::new(title).strong())
        .id_salt(id)
        .default_open(false)
        .show(ui, body);
    ui.add_space(2.0);
    response
}

fn draw_docker_environment(ui: &mut egui::Ui, environment: &DockerEnvironment) {
    // Both blocks are collapsed by default: they describe the machine, not
    // the results, and would otherwise eat the table's height budget (see
    // DOCKER_TAB_CHROME_MIN).
    if !environment.data_roots.is_empty() || environment.daemon_socket.is_some() {
        env_collapsing_header(
            ui,
            "docker_env_roots",
            t!("docker-env-roots-header", { "count" => environment.data_roots.len() }),
            |ui| {
                for root in &environment.data_roots {
                    ui.horizontal_wrapped(|ui| {
                        ui.monospace(root.path.display().to_string());
                        ui.small(storage_kind_label(root.kind));
                        ui.small(root_scope_label(root.scope));
                        if root.accessible {
                            ui.colored_label(
                                crate::colors::COLOR_SCAN_COMPLETE,
                                t!("docker-readable"),
                            );
                        } else {
                            ui.colored_label(crate::colors::WARNING_RED, t!("docker-not-readable"));
                        }
                    });
                }
                if let Some(socket) = &environment.daemon_socket {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new(t!("docker-env-socket")).strong());
                        ui.monospace(socket.display().to_string());
                    });
                }
            },
        );
    }

    if !environment.vm_disks.is_empty() {
        env_collapsing_header(
            ui,
            "docker_env_vmdisks",
            t!("docker-env-vmdisks-header", { "count" => environment.vm_disks.len() }),
            |ui| {
                for disk in &environment.vm_disks {
                    ui.horizontal_wrapped(|ui| {
                        ui.monospace(disk.path.display().to_string());
                        let size_text = t!("docker-vm-size", {
                            "apparent" => format_size(disk.apparent_bytes),
                            "allocated" => format_size(disk.allocated_bytes),
                        });
                        // Highlight sparse images (e.g. Docker.raw): a huge apparent
                        // size with a small host allocation.
                        if disk.allocated_bytes.saturating_mul(4) < disk.apparent_bytes {
                            ui.colored_label(crate::colors::COLOR_WARNING_YELLOW, size_text);
                        } else {
                            ui.small(size_text);
                        }
                    });
                }
                ui.add_space(2.0);
                ui.weak(t!("docker-vm-note"));
            },
        );
    }
}

fn draw_summary_band(ui: &mut egui::Ui, inventory: &DockerInventory) {
    let totals = &inventory.totals;
    ui.horizontal_wrapped(|ui| {
        summary_card(ui, &t!("docker-lbl-images"), totals.images_bytes);
        summary_card(ui, &t!("docker-lbl-shared"), totals.shared_bytes);
        summary_card(ui, &t!("docker-lbl-unique"), totals.unique_image_bytes);
        summary_card(ui, &t!("docker-lbl-containers"), totals.container_rw_bytes);
        summary_card(ui, &t!("docker-lbl-volumes"), totals.volumes_bytes);
        summary_card(ui, &t!("docker-lbl-build-cache"), totals.build_cache_bytes);
        summary_card(ui, &t!("docker-lbl-logs"), totals.log_bytes);
        summary_card(ui, &t!("docker-lbl-reclaimable"), totals.reclaimable_bytes);
    });
}

fn summary_card(ui: &mut egui::Ui, label: &str, bytes: u64) {
    egui::Frame::new()
        .fill(ui.visuals().selection.bg_fill.linear_multiply(0.12))
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().selection.stroke.color.linear_multiply(0.35),
        ))
        .inner_margin(egui::Margin::symmetric(8, 4))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).size(11.0).weak());
                ui.strong(format_size(bytes));
            });
        });
}

impl super::GuiApp {
    /// Keeps a sub-table's state consistent with the rendered inventory:
    /// marks the view dirty (and drops the index-based selection) when the
    /// row count changes, and establishes the section's default sort — size
    /// descending — until the user picks one.
    fn sync_docker_table_state(&mut self, section: DockerSection, rows: usize) {
        let count_changed = *self.docker_view.row_count_mut(section) != rows;
        if count_changed {
            *self.docker_view.row_count_mut(section) = rows;
        }
        let state = self.docker_view.table_state_mut(section);
        if state.columns.len() < section.column_count() {
            state.columns.resize_with(
                section.column_count(),
                egui_table_kit::header::ColumnState::default,
            );
        }
        if count_changed {
            state.filter_cache_dirty = true;
            state.selected_rows.clear();
            state.focused_key = None;
            state.anchor_key = None;
            state.last_clicked_visible_index = None;
        }
        if state.columns.iter().all(|col| col.sort_up.is_none()) {
            state.columns[section.default_sort_col()].sort_up = Some(false); // descending
            state.filter_cache_dirty = true;
        }
    }

    /// Slim operations toolbar directly above the results region: the
    /// section-agnostic refresh op plus the active section's delete op,
    /// icon-only like (but smaller than) the primary view's toolbar. Renders
    /// even without inventory so first-time users can run the first
    /// collection from here.
    fn draw_docker_toolbar(
        &mut self,
        ui: &mut egui::Ui,
        inventory: &DockerInventory,
        section: DockerSection,
    ) {
        let provider = DockerImagesProvider::new(inventory, section, self.time_format.clone());
        let state = self.docker_view.table_state_mut(section);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            // Refresh (registry group 0) then the section's delete op (group
            // 1, op index = section).
            for (group, op_idx) in [(0_usize, 0_usize), (1, section.op_index())] {
                let _ = self.docker_operations.show_operation(
                    ui,
                    &provider,
                    state,
                    group,
                    op_idx,
                    false,
                    |ui, op, enabled, reason| {
                        super::render_custom_op_button(
                            ui,
                            op.icon(),
                            op.name().as_ref(),
                            enabled,
                            reason,
                        )
                    },
                );
            }
        });
    }

    /// Right-click menu for a sub-table: the docker-scoped table operation
    /// for the section, rendered through the kit's op framework exactly like
    /// the explorer's file menu.
    fn draw_docker_table_menu_contents(
        &mut self,
        ui: &mut egui::Ui,
        inventory: &DockerInventory,
        section: DockerSection,
    ) {
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
        ui.set_min_width(200.0);
        let provider = DockerImagesProvider::new(inventory, section, self.time_format.clone());
        let state = self.docker_view.table_state_mut(section);
        // Refresh (registry group 0) then the section's delete op (group 1).
        for (group, op_idx) in [(0_usize, 0_usize), (1, section.op_index())] {
            let _ = self.docker_operations.show_operation(
                ui,
                &provider,
                state,
                group,
                op_idx,
                true,
                |ui, op, enabled, reason| {
                    let mut button = ui
                        .add_enabled(enabled, egui::Button::new(op.get_name(true)))
                        .on_hover_text(op.name());
                    if !enabled {
                        button = button.on_disabled_hover_text(format!("{}\n{reason}", op.name()));
                    }
                    button
                },
            );
        }
    }

    /// Renders a sub-table on the egui-table-kit framework (same as the
    /// explorer panel): slim ops toolbar above it, kit selection model,
    /// sortable/filterable headers, and the delete exposed as a kit
    /// `TableOperation` through the right-click context menu — no per-row
    /// action buttons.
    fn draw_docker_table(
        &mut self,
        ui: &mut egui::Ui,
        inventory: &DockerInventory,
        section: DockerSection,
    ) {
        let provider = DockerImagesProvider::new(inventory, section, self.time_format.clone());
        if provider.row_count() == 0 {
            ui.centered_and_justified(|ui| {
                ui.label(t!("docker-no-entries"));
            });
            return;
        }

        self.sync_docker_table_state(section, provider.row_count());

        let columns = docker_table_columns(section);
        let table_id = section.table_id();

        let output = {
            let state = self.docker_view.table_state_mut(section);
            TableKit::new(table_id, &provider, state)
                .with_columns(columns)
                .with_row_height(22.0)
                .with_max_height(Some(ui.available_height()))
                .with_striped(true)
                .show_with_output(ui, |ui, cell_info, row_data, text_color| {
                    docker_custom_cell(ui, cell_info, row_data, text_color, inventory, section)
                })
        };
        let Ok(output) = output else {
            return;
        };

        // Right-click opens the operations menu; the kit's Legacy selection
        // model leaves the selection untouched on context clicks, so anchor
        // it to the clicked row first (same pattern as the explorer).
        let mut show_menu = false;
        let mut menu_pos = egui::Pos2::ZERO;
        for event in &output.events {
            if event.kind == RowEventKind::SecondaryClicked {
                let state = self.docker_view.table_state_mut(section);
                if !state.selected_rows.contains(event.row_index as u32) {
                    state.selected_rows.clear();
                    state.selected_rows.insert(event.row_index as u32);
                }
                if let Some(pos) = ui.ctx().pointer_latest_pos() {
                    menu_pos = pos;
                    show_menu = true;
                }
            }
        }

        let popup_id = ui.make_persistent_id(("docker_table_context_menu", table_id));
        if show_menu {
            ui.data_mut(|d| d.insert_temp(popup_id.with("pos"), menu_pos));
            egui::Popup::toggle_id(ui.ctx(), popup_id);
        }

        let saved_pos = ui.data(|d| d.get_temp::<egui::Pos2>(popup_id.with("pos")));
        if let Some(pos) = saved_pos {
            let anchor_rect = egui::Rect::from_center_size(pos, egui::Vec2::splat(1.0));
            let dummy_response = ui.interact(
                anchor_rect,
                popup_id.with("dummy_anchor"),
                egui::Sense::hover(),
            );
            egui::Popup::menu(&dummy_response)
                .id(popup_id)
                .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    self.draw_docker_table_menu_contents(ui, inventory, section);
                });
        }

        if !egui::Popup::is_id_open(ui.ctx(), popup_id) {
            ui.data_mut(|d| {
                d.remove::<egui::Pos2>(popup_id.with("pos"));
            });
        }
    }
}

/// Column layouts of the three sub-tables (resizable).
fn docker_table_columns(section: DockerSection) -> Vec<egui_table_kit::layout::Column> {
    #[allow(clippy::shadow_unrelated)]
    let mk = |w: f32, min: f32, max: f32| {
        egui_table_kit::layout::Column::new(w)
            .range(min..=max)
            .resizable(true)
    };
    match section {
        DockerSection::Images => vec![
            mk(240.0, 80.0, 500.0),  // Name
            mk(120.0, 60.0, 300.0),  // ID
            mk(100.0, 50.0, 200.0),  // Size
            mk(100.0, 50.0, 200.0),  // Shared
            mk(100.0, 50.0, 200.0),  // Exclusive
            mk(80.0, 40.0, 150.0),   // Layers
            mk(150.0, 100.0, 300.0), // Created
            mk(60.0, 40.0, 150.0),   // Refs
        ],
        DockerSection::Containers => vec![
            mk(240.0, 80.0, 500.0),  // Name
            mk(120.0, 60.0, 300.0),  // ID
            mk(100.0, 50.0, 200.0),  // RW size
            mk(100.0, 50.0, 200.0),  // Log size
            mk(150.0, 100.0, 300.0), // Created
        ],
        DockerSection::Volumes => vec![
            mk(300.0, 80.0, 600.0),  // Name
            mk(110.0, 50.0, 300.0),  // Size
            mk(70.0, 40.0, 150.0),   // Refs
            mk(150.0, 100.0, 300.0), // Created
        ],
    }
}

/// Per-section cell styling layered on the kit's default label rendering:
/// dangling (untagged) images are de-emphasized in the name column, a zero
/// reference count (prune candidate) is de-emphasized in the refs column,
/// and large container logs keep the warning highlight the old bespoke
/// table had. Everything else renders with the kit default.
fn docker_custom_cell(
    ui: &mut egui::Ui,
    cell_info: &egui_table_kit::layout::CellInfo,
    row_data: &dyn egui_table_kit::operations::Row,
    text_color: egui::Color32,
    inventory: &DockerInventory,
    section: DockerSection,
) -> Option<egui::Response> {
    let provider_idx = row_data.row_index()?;
    let (val, _) = row_data.cell(cell_info.col_nr)?;
    let deemphasized = match (section, cell_info.col_nr) {
        (DockerSection::Images, 0) => inventory
            .images
            .get(provider_idx)
            .is_some_and(|image| image.tags.is_empty()),
        (DockerSection::Images, 7) => inventory
            .images
            .get(provider_idx)
            .is_some_and(|image| image.ref_count == 0),
        (DockerSection::Volumes, 2) => inventory
            .volumes
            .get(provider_idx)
            .is_some_and(|volume| volume.ref_count == 0),
        _ => false,
    };
    let warn = matches!(section, DockerSection::Containers) && cell_info.col_nr == 3;
    if !deemphasized && !warn {
        return None;
    }
    let mut rich = egui::RichText::new(val.as_ref());
    if deemphasized {
        rich = rich.weak();
    }
    if warn
        && inventory
            .containers
            .get(provider_idx)
            .is_some_and(|container| container.log_bytes >= LARGE_LOG_BYTES)
    {
        rich = rich.color(crate::colors::COLOR_WARNING_YELLOW);
    } else {
        rich = rich.color(text_color);
    }
    Some(
        ui.add(
            egui::Label::new(rich)
                .selectable(false)
                .wrap_mode(egui::TextWrapMode::Truncate),
        ),
    )
}

fn draw_warnings(ui: &mut egui::Ui, warnings: &[InventoryWarning]) {
    ui.label(
        egui::RichText::new(t!("docker-warnings-title"))
            .strong()
            .color(crate::colors::COLOR_WARNING_YELLOW),
    );
    egui::ScrollArea::vertical()
        .id_salt("docker_warnings_scroll")
        .max_height(140.0)
        .show(ui, |ui| {
            for warning in warnings {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(crate::colors::COLOR_WARNING_YELLOW, "⚠");
                    ui.label(warning_text(warning));
                });
            }
        });
}

fn warning_text(warning: &InventoryWarning) -> String {
    match warning {
        InventoryWarning::LargeContainerLog { container, bytes } => t!("docker-warn-large-log", {
            "container" => container.as_str(),
            "size" => format_size(*bytes),
        })
        .into_owned(),
        InventoryWarning::UnreferencedLayer { cache_id, bytes } => t!("docker-warn-unref-layer", {
            "cache_id" => cache_id.as_str(),
            "size" => format_size(*bytes),
        })
        .into_owned(),
        InventoryWarning::RootUnreadable { path } => t!("docker-warn-root-unreadable", {
            "path" => path.display().to_string(),
        })
        .into_owned(),
        InventoryWarning::ContainerdStorePartial => {
            t!("docker-warn-containerd-partial").into_owned()
        }
        InventoryWarning::MetadataParse { detail } => t!("docker-warn-metadata", {
            "detail" => detail.as_str(),
        })
        .into_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::arena::{FileNode, NodeStorage, StringPool};
    use crate::docker::{DataRoot, VmBackend, VmDisk};

    fn sample_environment() -> DockerEnvironment {
        DockerEnvironment {
            data_roots: vec![DataRoot {
                path: PathBuf::from("/var/lib/docker"),
                kind: StorageKind::Overlay2,
                scope: RootScope::System,
                accessible: false,
            }],
            vm_disks: vec![VmDisk {
                path: PathBuf::from("/home/user/.docker/desktop/Docker.raw"),
                apparent_bytes: 1_000_000,
                allocated_bytes: 10_000,
                backend: VmBackend::DockerDesktopMac,
            }],
            daemon_socket: None,
        }
    }

    #[test]
    fn badge_context_matches_roots_disks_and_areas() {
        let ctx = DockerBadgeContext::from_environment(&sample_environment());
        assert_eq!(
            ctx.badge_for("/var/lib/docker"),
            Some(("📦", t!("badge-docker")))
        );
        assert_eq!(
            ctx.badge_for("/var/lib/docker/overlay2"),
            Some(("📦", t!("badge-docker-area", { "area" => "overlay2" })))
        );
        assert_eq!(
            ctx.badge_for("/var/lib/docker/buildkit"),
            Some(("📦", t!("badge-docker-area", { "area" => "buildkit" })))
        );
        assert_eq!(
            ctx.badge_for("/home/user/.docker/desktop/Docker.raw"),
            Some(("📦", t!("badge-docker-vm")))
        );
    }

    #[test]
    fn badge_context_normalizes_separators() {
        let ctx = DockerBadgeContext::from_environment(&sample_environment());
        assert!(ctx.badge_for("/var/lib/docker/").is_some());
        assert!(ctx.badge_for("\\var\\lib\\docker").is_some());
    }

    #[test]
    fn badge_context_windows_paths() {
        let environment = DockerEnvironment {
            data_roots: vec![DataRoot {
                path: PathBuf::from("C:\\var\\lib\\docker"),
                kind: StorageKind::Overlay2,
                scope: RootScope::System,
                accessible: true,
            }],
            vm_disks: Vec::new(),
            daemon_socket: None,
        };
        let ctx = DockerBadgeContext::from_environment(&environment);
        assert!(ctx.badge_for("C:/var/lib/docker").is_some());
        assert!(ctx.badge_for("C:\\var\\lib\\docker\\volumes").is_some());
        assert!(ctx.badge_for("\\\\?\\C:\\var\\lib\\docker").is_some());
    }

    #[test]
    fn badge_context_rejects_unrelated_paths() {
        let ctx = DockerBadgeContext::from_environment(&sample_environment());
        // Descendants beyond the direct children do not match.
        assert_eq!(ctx.badge_for("/var/lib/docker/overlay2/abc123"), None);
        assert_eq!(ctx.badge_for("/var/lib"), None);
        assert_eq!(ctx.badge_for("/var/lib/dockerx"), None);
        assert_eq!(
            ctx.badge_for("/home/user/.docker/desktop/Docker.raw.extra"),
            None
        );
        // An empty environment never matches.
        let empty = DockerBadgeContext::from_environment(&DockerEnvironment::default());
        assert_eq!(empty.badge_for("/var/lib/docker"), None);
    }

    /// Builds a snapshot rooted at `/var/lib/docker` with one volume directory
    /// and one container directory (rw layer + json log).
    fn docker_tree_snapshot() -> FileArenaSnapshot {
        let mut pool = StringPool::new();
        let root_id = pool.get_or_insert(b"/var/lib/docker");
        let volumes_id = pool.get_or_insert(b"volumes");
        let vol1_id = pool.get_or_insert(b"vol1");
        let containers_id = pool.get_or_insert(b"containers");
        let cid_id = pool.get_or_insert(b"cid");
        let log_id = pool.get_or_insert(b"cid-json.log");

        let mut root = FileNode::new(root_id, None, true, false, 0, 0);
        root.size = 70;
        root.first_child = 1;

        let mut volumes = FileNode::new(volumes_id, Some(0), true, false, 0, 0);
        volumes.size = 30;
        volumes.first_child = 2;
        volumes.next_sibling = 3;

        let mut vol1 = FileNode::new(vol1_id, Some(1), true, false, 0, 0);
        vol1.size = 30;

        let mut containers = FileNode::new(containers_id, Some(0), true, false, 0, 0);
        containers.size = 40;
        containers.first_child = 4;

        let mut cid = FileNode::new(cid_id, Some(3), true, false, 0, 0);
        // Container dir (40) = rw layer (25) + json log (15).
        cid.size = 40;
        cid.first_child = 5;

        let mut log = FileNode::new(log_id, Some(4), false, false, 0, 0);
        log.size = 15;

        let nodes = vec![root, volumes, vol1, containers, cid, log];
        FileArenaSnapshot {
            nodes: Arc::new(NodeStorage::Owned(nodes)),
            string_pool: Arc::new(pool),
            dir_counts: Arc::new(vec![]),
            extensions: crate::extensions::ExtensionStore::default(),
        }
    }

    #[test]
    fn refine_updates_sizes_and_totals() {
        let snapshot = docker_tree_snapshot();
        let root = PathBuf::from("/var/lib/docker");
        let mut inventory = DockerInventory {
            root: Some(root.clone()),
            volumes: vec![crate::docker::VolumeInfo {
                name: "vol1".to_string(),
                size_bytes: 5,
                ref_count: 0,
                created: 0,
            }],
            containers: vec![crate::docker::ContainerInfo {
                id: "cid".to_string(),
                name: "ctr".to_string(),
                image_id: None,
                rw_size_bytes: 5,
                log_bytes: 5,
                created: 0,
            }],
            layers: vec![crate::docker::LayerInfo {
                cache_id: "lay1".to_string(),
                size_bytes: 5,
                path: root.join("overlay2/lay1/diff"),
                image_ids: vec![],
                container_mount: false,
                referenced: true,
            }],
            build_cache_bytes: 0,
            ..DockerInventory::default()
        };
        inventory.compute_totals();
        let before_unique = inventory.totals.unique_image_bytes;
        assert_eq!(before_unique, 5);

        // The tree has no overlay2 node, so the layer keeps its parser size.
        assert!(refine_inventory_from_snapshot(&mut inventory, &snapshot));
        assert_eq!(inventory.volumes[0].size_bytes, 30);
        assert_eq!(inventory.containers[0].log_bytes, 15);
        assert_eq!(inventory.containers[0].rw_size_bytes, 25);
        assert_eq!(inventory.layers[0].size_bytes, 5);
        assert_eq!(inventory.totals.volumes_bytes, 30);
        assert_eq!(inventory.totals.container_rw_bytes, 25);
        assert_eq!(inventory.totals.log_bytes, 15);
    }

    #[test]
    fn refine_without_covering_snapshot_keeps_parser_sizes() {
        let snapshot = docker_tree_snapshot();
        let mut inventory = DockerInventory {
            root: Some(PathBuf::from("/var/lib/docker")),
            volumes: vec![crate::docker::VolumeInfo {
                name: "vol1".to_string(),
                size_bytes: 5,
                ref_count: 0,
                created: 0,
            }],
            ..DockerInventory::default()
        };
        inventory.compute_totals();

        // The scanned tree only resolves paths under its own root.
        let mut unrelated = snapshot;
        unrelated.nodes = Arc::new(NodeStorage::Owned(Vec::new()));
        assert!(!refine_inventory_from_snapshot(&mut inventory, &unrelated));
        assert_eq!(inventory.volumes[0].size_bytes, 5);
    }

    fn app_with_snapshot(snapshot: FileArenaSnapshot) -> super::super::GuiApp {
        let shared_state = Arc::new(crate::state::SharedState::new());
        shared_state.store_snapshot(snapshot);
        super::super::GuiApp::new(shared_state, None, None, false)
    }

    fn pending_inventory() -> DockerInventory {
        let mut inventory = DockerInventory {
            root: Some(PathBuf::from("/var/lib/docker")),
            volumes: vec![crate::docker::VolumeInfo {
                name: "vol1".to_string(),
                size_bytes: 5,
                ref_count: 0,
                created: 0,
            }],
            warnings: vec![InventoryWarning::ContainerdStorePartial],
            ..DockerInventory::default()
        };
        inventory.compute_totals();
        inventory
    }

    #[test]
    fn publish_pending_merges_refined_inventory_into_snapshot() {
        let app = app_with_snapshot(docker_tree_snapshot());
        *app.docker_view.pending.write() = Some(Ok(pending_inventory()));

        assert!(app.publish_pending_inventory());

        // The result is consumed; rendering now reads the snapshot.
        assert!(app.docker_view.pending.read().is_none());

        let snapshot = app.shared_state.current_snapshot.load();
        let inventory = snapshot.docker_inventory();
        assert!(inventory.is_some());
        let inventory = inventory.unwrap_or_default();
        // Refined against the scanned tree before publishing.
        assert_eq!(inventory.volumes[0].size_bytes, 30);
        assert_eq!(inventory.totals.volumes_bytes, 30);
        // Provenance stamped: collection time set, containerd source inferred
        // from the partial-store warning.
        assert!(inventory.collected_at > 0);
        assert_eq!(inventory.source, InventorySource::DiskContainerdPartial);
    }

    #[test]
    fn publish_pending_preserves_other_extensions() {
        let mut snapshot = docker_tree_snapshot();
        let unknown_id = u32::from_le_bytes(*b"UNKN");
        snapshot
            .extensions
            .insert(unknown_id, 7, Arc::from(&b"opaque"[..]));
        let app = app_with_snapshot(snapshot);
        *app.docker_view.pending.write() = Some(Ok(pending_inventory()));

        assert!(app.publish_pending_inventory());

        let snapshot = app.shared_state.current_snapshot.load();
        let unknown = snapshot.extensions.get(unknown_id);
        assert!(unknown.is_some());
        assert_eq!(unknown.map(|e| e.version), Some(7));
        assert_eq!(unknown.map(|e| &*e.payload), Some(&b"opaque"[..]));
        assert!(snapshot.docker_inventory().is_some());
    }

    #[test]
    fn publish_pending_keeps_error_visible() {
        let app_shared = Arc::new(crate::state::SharedState::new());
        app_shared.store_snapshot(docker_tree_snapshot());
        let app = super::super::GuiApp::new(app_shared, None, None, false);
        let err = crate::WeshtatisticError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no Docker data root found on this system",
        ));
        *app.docker_view.pending.write() = Some(Err(err));

        assert!(!app.publish_pending_inventory());

        // The error stays until the next collection starts.
        let pending = app.docker_view.pending.read();
        assert!(matches!(pending.as_ref(), Some(Err(_))));
        drop(pending);
        let snapshot = app.shared_state.current_snapshot.load();
        assert!(snapshot.docker_inventory().is_none());
    }

    #[test]
    fn publish_pending_without_result_is_noop() {
        let app = app_with_snapshot(docker_tree_snapshot());
        assert!(!app.publish_pending_inventory());
        let snapshot = app.shared_state.current_snapshot.load();
        assert!(snapshot.docker_inventory().is_none());
    }

    #[test]
    fn source_label_maps_all_variants() {
        assert_eq!(
            source_label(InventorySource::DiskOverlay2),
            t!("docker-source-disk")
        );
        assert_eq!(
            source_label(InventorySource::DiskContainerdPartial),
            t!("docker-source-partial")
        );
        assert_eq!(
            source_label(InventorySource::DockerApi),
            t!("docker-source-api")
        );
    }

    // ---- Background resource deletion ----

    /// What a [`MockDockerCollector`] should report from the delete endpoint
    /// its kind dispatches to. `FailedIf` fails only the matching key, so a
    /// batch can mix success and failure.
    #[derive(Clone)]
    enum MockDeleteOutcome {
        Deleted { untagged: usize, deleted: usize },
        DeletedUnit,
        Failed(String),
        FailedIf(&'static str, &'static str),
    }

    struct MockDockerCollector {
        outcome: MockDeleteOutcome,
    }

    impl DockerCollector for MockDockerCollector {
        fn detect_environment(&self) -> DockerEnvironment {
            DockerEnvironment::default()
        }

        fn collect_inventory(
            &self,
            _cancel: &AtomicBool,
        ) -> Result<DockerInventory, crate::WeshtatisticError> {
            Err(crate::WeshtatisticError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no inventory in tests",
            )))
        }

        fn delete_image(
            &self,
            _image_id: &str,
            _force: bool,
        ) -> Result<ImageDeletion, crate::WeshtatisticError> {
            match &self.outcome {
                MockDeleteOutcome::Deleted { untagged, deleted } => Ok(ImageDeletion {
                    untagged: vec!["repo:tag".to_string(); *untagged],
                    deleted: vec!["sha256:layer".to_string(); *deleted],
                }),
                MockDeleteOutcome::DeletedUnit => Ok(ImageDeletion::default()),
                MockDeleteOutcome::Failed(message) => Err(crate::WeshtatisticError::Io(
                    std::io::Error::other(message.clone()),
                )),
                MockDeleteOutcome::FailedIf(key, message) => {
                    if _image_id == *key {
                        Err(crate::WeshtatisticError::Io(std::io::Error::other(*message)))
                    } else {
                        Ok(ImageDeletion::default())
                    }
                }
            }
        }

        fn delete_container(
            &self,
            _container_id: &str,
            _force: bool,
        ) -> Result<(), crate::WeshtatisticError> {
            match &self.outcome {
                MockDeleteOutcome::Failed(message) => Err(crate::WeshtatisticError::Io(
                    std::io::Error::other(message.clone()),
                )),
                MockDeleteOutcome::FailedIf(key, message) => {
                    if _container_id == *key {
                        Err(crate::WeshtatisticError::Io(std::io::Error::other(*message)))
                    } else {
                        Ok(())
                    }
                }
                _ => Ok(()),
            }
        }

        fn delete_volume(&self, _name: &str, _force: bool) -> Result<(), crate::WeshtatisticError> {
            match &self.outcome {
                MockDeleteOutcome::Failed(message) => Err(crate::WeshtatisticError::Io(
                    std::io::Error::other(message.clone()),
                )),
                MockDeleteOutcome::FailedIf(key, message) => {
                    if _name == *key {
                        Err(crate::WeshtatisticError::Io(std::io::Error::other(*message)))
                    } else {
                        Ok(())
                    }
                }
                _ => Ok(()),
            }
        }
    }

    fn mock_collector(outcome: MockDeleteOutcome) -> Arc<MockDockerCollector> {
        Arc::new(MockDockerCollector { outcome })
    }

    /// Polls `take` until the background batch lands (with a deadline so a
    /// regression cannot hang the test run).
    fn await_deletion(state: &DockerDeletionState) -> DockerDeletionSummary {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Some(done) = state.take() {
                return done;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "background deletion did not complete"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    fn image_target() -> DockerDeleteTarget {
        DockerDeleteTarget {
            kind: DockerResourceKind::Image,
            id: "img1".to_string(),
            name: "repo:tag".to_string(),
            size_bytes: 42,
            extra_bytes: 0,
        }
    }

    #[test]
    fn image_deletion_state_success_transitions() -> Result<(), crate::WeshtatisticError> {
        let state = DockerDeletionState::default();
        let ctx = egui::Context::default();
        let target = image_target();

        assert!(state.begin(
            mock_collector(MockDeleteOutcome::Deleted {
                untagged: 2,
                deleted: 3
            }),
            vec![target.clone()],
            false,
            &ctx
        ));
        assert!(state.is_running());
        // A second batch is refused while one is running.
        assert!(!state.begin(
            mock_collector(MockDeleteOutcome::Deleted {
                untagged: 0,
                deleted: 0
            }),
            vec![target.clone()],
            true,
            &ctx
        ));
        // An empty batch never starts.
        assert!(!state.begin(
            mock_collector(MockDeleteOutcome::DeletedUnit),
            Vec::new(),
            false,
            &ctx
        ));

        let summary = await_deletion(&state);
        assert_eq!(summary.len(), 1);
        assert_eq!(summary.success_count(), 1);
        assert_eq!(summary.results[0].0, target);
        let Ok(Some(deletion)) = summary.results[0].1.as_ref() else {
            return Err(crate::WeshtatisticError::Io(std::io::Error::from(
                std::io::ErrorKind::InvalidData,
            )));
        };
        assert_eq!(deletion.untagged.len(), 2);
        assert_eq!(deletion.deleted.len(), 3);
        assert_eq!(summary.image_totals(), (2, 3));
        // The render loop drains the slot exactly once.
        assert!(!state.is_running());
        assert!(state.take().is_none());
        Ok(())
    }

    #[test]
    fn deletion_state_aggregates_mixed_batch() {
        let state = DockerDeletionState::default();
        let ctx = egui::Context::default();
        let ok = DockerDeleteTarget {
            kind: DockerResourceKind::Image,
            id: "img-ok".to_string(),
            name: "repo:ok".to_string(),
            size_bytes: 10,
            extra_bytes: 0,
        };
        let bad = DockerDeleteTarget {
            kind: DockerResourceKind::Image,
            id: "img-bad".to_string(),
            name: "repo:bad".to_string(),
            size_bytes: 20,
            extra_bytes: 0,
        };

        assert!(state.begin(
            mock_collector(MockDeleteOutcome::FailedIf(
                "img-bad",
                "image is being used by stopped container abc123"
            )),
            vec![ok.clone(), bad.clone()],
            false,
            &ctx
        ));

        let summary = await_deletion(&state);
        assert_eq!(summary.len(), 2);
        assert_eq!(summary.kind(), Some(DockerResourceKind::Image));
        assert_eq!(summary.success_count(), 1);
        // Order preserved: ok first, bad second.
        assert_eq!(summary.results[0].0, ok);
        assert!(summary.results[0].1.is_ok());
        assert_eq!(summary.results[1].0, bad);
        assert!(summary.results[1].1.is_err());
        // The failure carries the daemon's verbatim message.
        let failures = summary.failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].0, "repo:bad");
        assert!(
            failures[0]
                .1
                .contains("image is being used by stopped container abc123")
        );
        // The mixed batch still exposes per-image summary totals (only the
        // successful Ok-image contributes; here it defaults to zero).
        assert_eq!(summary.image_totals(), (0, 0));
        assert!(!state.is_running());
    }

    #[test]
    fn container_deletion_state_success_confirms_without_summary() {
        let state = DockerDeletionState::default();
        let ctx = egui::Context::default();
        let target = DockerDeleteTarget {
            kind: DockerResourceKind::Container,
            id: "ctr1".to_string(),
            name: "ctr".to_string(),
            size_bytes: 25,
            extra_bytes: 15,
        };

        assert!(state.begin(
            mock_collector(MockDeleteOutcome::DeletedUnit),
            vec![target.clone()],
            true,
            &ctx
        ));

        let summary = await_deletion(&state);
        assert_eq!(summary.results[0].0, target);
        assert!(
            summary.results[0].1.is_ok(),
            "container deletion should succeed"
        );
        let Ok(result) = summary.results[0].1.as_ref() else {
            return; // unreachable: the assert above fails first
        };
        assert!(
            result.is_none(),
            "container deletion reports no untagged/deleted summary"
        );
        assert_eq!(summary.failures().len(), 0);
        assert!(!state.is_running());
    }

    #[test]
    fn volume_deletion_state_failure_carries_daemon_message() {
        let state = DockerDeletionState::default();
        let ctx = egui::Context::default();
        let daemon_message = "volume is in use - used by container abc123".to_string();
        let target = DockerDeleteTarget {
            kind: DockerResourceKind::Volume,
            id: "vol1".to_string(),
            name: "vol1".to_string(),
            size_bytes: 30,
            extra_bytes: 0,
        };

        assert!(state.begin(
            mock_collector(MockDeleteOutcome::Failed(daemon_message)),
            vec![target.clone()],
            false,
            &ctx
        ));

        let summary = await_deletion(&state);
        assert_eq!(summary.results[0].0, target);
        assert_eq!(summary.success_count(), 0);
        let failures = summary.failures();
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0]
                .1
                .contains("volume is in use - used by container abc123"),
            "deletion should fail with the daemon's message"
        );
        assert!(!state.is_running());
    }

    #[test]
    fn docker_collector_default_delete_endpoints_are_unsupported() {
        struct NoDeleteCollector;
        impl DockerCollector for NoDeleteCollector {
            fn detect_environment(&self) -> DockerEnvironment {
                DockerEnvironment::default()
            }

            fn collect_inventory(
                &self,
                _cancel: &AtomicBool,
            ) -> Result<DockerInventory, crate::WeshtatisticError> {
                Err(crate::WeshtatisticError::Io(std::io::Error::from(
                    std::io::ErrorKind::Unsupported,
                )))
            }
        }

        // Collectors without daemon access (or the wasm frontend) fall back
        // to the defaults: "unsupported" errors the GUI surfaces as toasts.
        let image_outcome = NoDeleteCollector.delete_image("img1", false);
        assert!(
            image_outcome.is_err(),
            "default delete_image must be unsupported"
        );
        let Some(err) = image_outcome.err() else {
            return; // unreachable: the assert above fails first
        };
        assert!(err.to_string().contains("not supported"));

        let container_outcome = NoDeleteCollector.delete_container("ctr1", false);
        assert!(
            container_outcome.is_err(),
            "default delete_container must be unsupported"
        );
        let Some(err) = container_outcome.err() else {
            return; // unreachable: the assert above fails first
        };
        assert!(err.to_string().contains("not supported"));

        let volume_outcome = NoDeleteCollector.delete_volume("vol1", false);
        assert!(
            volume_outcome.is_err(),
            "default delete_volume must be unsupported"
        );
        let Some(err) = volume_outcome.err() else {
            return; // unreachable: the assert above fails first
        };
        assert!(err.to_string().contains("not supported"));
    }

    // ---- Table provider (egui-table-kit) ----

    /// Sample inventory with out-of-order sizes in every section.
    fn sample_images_inventory() -> DockerInventory {
        DockerInventory {
            root: Some(PathBuf::from("/var/lib/docker")),
            images: vec![
                crate::docker::ImageInfo {
                    id: "aaaa1111".to_string(),
                    tags: Vec::new(), // dangling
                    size_bytes: 100,
                    shared_bytes: 40,
                    layer_cache_ids: vec!["l1".to_string()],
                    // Filled from image inspect even when the cache-id vec
                    // is unavailable (API inventories).
                    layer_count: 7,
                    ref_count: 0,
                    created: 0,
                },
                crate::docker::ImageInfo {
                    id: "bbbb2222cccc".to_string(),
                    tags: vec!["repo:big".to_string()],
                    size_bytes: 300,
                    shared_bytes: 100,
                    layer_cache_ids: vec!["l2".to_string(), "l3".to_string()],
                    layer_count: 2,
                    ref_count: 3,
                    created: 1_700_000_000,
                },
            ],
            containers: vec![
                crate::docker::ContainerInfo {
                    id: "cccc3333".to_string(),
                    name: "ctr-small".to_string(),
                    image_id: None,
                    rw_size_bytes: 25,
                    log_bytes: 15,
                    created: 0,
                },
                crate::docker::ContainerInfo {
                    id: "dddd4444".to_string(),
                    name: "ctr-big".to_string(),
                    image_id: None,
                    rw_size_bytes: 250,
                    log_bytes: 150,
                    created: 1_600_000_000,
                },
            ],
            volumes: vec![
                crate::docker::VolumeInfo {
                    name: "vol-small".to_string(),
                    size_bytes: 30,
                    ref_count: 0,
                    created: 0,
                },
                crate::docker::VolumeInfo {
                    name: "vol-big".to_string(),
                    size_bytes: 3000,
                    ref_count: 2,
                    created: 1_700_000_000,
                },
            ],
            ..DockerInventory::default()
        }
    }

    /// Unwrap-free cell access: `?` propagates provider errors, and a missing
    /// cell becomes an assertion failure via the returned error.
    fn cell(
        provider: &DockerImagesProvider<'_>,
        row: usize,
        col: usize,
    ) -> Result<(String, Option<String>), TableError> {
        let (val, hover) = provider
            .cell_at(row, col)?
            .ok_or(TableError::CorruptedState)?;
        Ok((val.into_owned(), hover.map(std::borrow::Cow::into_owned)))
    }

    #[test]
    fn docker_images_provider_maps_columns() -> Result<(), TableError> {
        let inventory = sample_images_inventory();
        let provider = DockerImagesProvider::new(
            &inventory,
            DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );

        assert_eq!(provider.column_count(), 8);
        assert_eq!(provider.row_count(), 2);
        assert!(provider.header(0).is_some());
        assert!(provider.header(7).is_some());
        assert!(provider.header(8).is_none());

        // Name cell: display name with full-name hover; dangling images
        // report the `<none>` display name.
        let (name, name_hover) = cell(&provider, 0, 0)?;
        assert_eq!(name, "<none>");
        assert_eq!(name_hover.as_deref(), Some("<none>"));
        let (name, _) = cell(&provider, 1, 0)?;
        assert_eq!(name, "repo:big");

        // ID cell: short ID on display, full ID in the hover (the delete op
        // resolves the image through that hover).
        let (short, full) = cell(&provider, 1, 1)?;
        assert_eq!(short, "bbbb2222cccc");
        assert_eq!(full.as_deref(), Some("bbbb2222cccc"));

        // Sizes are formatted; layer count comes from `layer_count` (row 0
        // has one cache-id entry but reports the inspected count 7).
        let (size, _) = cell(&provider, 1, 2)?;
        assert_eq!(size, format_size(300));
        let (layers, _) = cell(&provider, 0, 5)?;
        assert_eq!(layers, "7");

        // Created is formatted per the user's time format (0 = unknown);
        // refs is the plain container-reference count.
        let (created, _) = cell(&provider, 0, 6)?;
        assert_eq!(
            created,
            crate::time_utils::format_epoch(0, &crate::time_utils::TimeFormat::default())
        );
        let (created, _) = cell(&provider, 1, 6)?;
        assert_ne!(created, "");
        let (refs, _) = cell(&provider, 1, 7)?;
        assert_eq!(refs, "3");
        let (refs, _) = cell(&provider, 0, 7)?;
        assert_eq!(refs, "0");

        // Out-of-range rows and columns map to no cell.
        assert!(provider.cell_at(2, 0)?.is_none());
        assert!(provider.cell_at(0, 8)?.is_none());
        Ok(())
    }

    #[test]
    fn docker_images_provider_sorts_numerically() -> Result<(), TableError> {
        let inventory = sample_images_inventory();
        let provider = DockerImagesProvider::new(
            &inventory,
            DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );

        // Default view order: insertion order (small first here).
        let mut rows = vec![0, 1];
        provider.sort_active_rows(&mut rows, 2, false)?;
        assert_eq!(rows, vec![1, 0], "size descending must be numeric");

        provider.sort_active_rows(&mut rows, 2, true)?;
        assert_eq!(rows, vec![0, 1], "size ascending must be numeric");

        provider.sort_active_rows(&mut rows, 0, true)?;
        assert_eq!(rows, vec![0, 1], "<none> sorts before repo:big");

        // Layers sort by `layer_count` (row 0 = 7 layers, row 1 = 2).
        provider.sort_active_rows(&mut rows, 5, false)?;
        assert_eq!(rows, vec![0, 1], "layer count descending");
        // Refs (row 1 = 3, row 0 = 0) and created (row 1 newer).
        provider.sort_active_rows(&mut rows, 7, false)?;
        assert_eq!(rows, vec![1, 0], "ref count descending");
        provider.sort_active_rows(&mut rows, 6, true)?;
        assert_eq!(rows, vec![0, 1], "created ascending");
        Ok(())
    }

    #[test]
    fn docker_images_provider_iterates_selected_rows() -> Result<(), TableError> {
        use egui_table_kit::operations::RowSliceExt as _;

        let inventory = sample_images_inventory();
        let provider = DockerImagesProvider::new(
            &inventory,
            DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );
        let mut state = TableState::new(DOCKER_IMAGES_TABLE_ID, 2);
        state.selected_rows.insert(1);

        let mut seen = Vec::new();
        provider.for_selected_rows(&state, &mut |row| {
            seen.push(row.get_primary(1)?.into_owned());
            Ok(())
        })?;
        assert_eq!(seen, vec!["bbbb2222cccc".to_string()]);
        Ok(())
    }

    #[test]
    fn docker_containers_provider_maps_and_sorts() -> Result<(), TableError> {
        let inventory = sample_images_inventory();
        let provider = DockerImagesProvider::new(
            &inventory,
            DockerSection::Containers,
            crate::time_utils::TimeFormat::default(),
        );

        assert_eq!(provider.column_count(), 5);
        assert_eq!(provider.row_count(), 2);

        let (name, name_hover) = cell(&provider, 1, 0)?;
        assert_eq!(name, "ctr-big");
        assert_eq!(name_hover.as_deref(), Some("ctr-big"));
        // Full container ID rides in the hover (the delete op resolves it).
        let (short, full) = cell(&provider, 1, 1)?;
        assert_eq!(short, "dddd4444");
        assert_eq!(full.as_deref(), Some("dddd4444"));
        let (rw, _) = cell(&provider, 1, 2)?;
        assert_eq!(rw, format_size(250));
        let (log, _) = cell(&provider, 1, 3)?;
        assert_eq!(log, format_size(150));
        let (created, _) = cell(&provider, 1, 4)?;
        assert_ne!(created, "");

        // Default sort: writable-layer size descending.
        let mut rows = vec![0, 1];
        provider.sort_active_rows(&mut rows, 2, false)?;
        assert_eq!(rows, vec![1, 0], "rw size descending must be numeric");
        provider.sort_active_rows(&mut rows, 3, true)?;
        assert_eq!(rows, vec![0, 1], "log size ascending must be numeric");
        provider.sort_active_rows(&mut rows, 4, true)?;
        assert_eq!(rows, vec![0, 1], "created ascending (0 first)");
        Ok(())
    }

    #[test]
    fn docker_volumes_provider_maps_and_sorts() -> Result<(), TableError> {
        let inventory = sample_images_inventory();
        let provider = DockerImagesProvider::new(
            &inventory,
            DockerSection::Volumes,
            crate::time_utils::TimeFormat::default(),
        );

        assert_eq!(provider.column_count(), 4);
        assert_eq!(provider.row_count(), 2);

        let (name, name_hover) = cell(&provider, 1, 0)?;
        assert_eq!(name, "vol-big");
        assert_eq!(name_hover.as_deref(), Some("vol-big"));
        let (size, _) = cell(&provider, 1, 1)?;
        assert_eq!(size, format_size(3000));
        let (refs, _) = cell(&provider, 1, 2)?;
        assert_eq!(refs, "2");
        let (refs, _) = cell(&provider, 0, 2)?;
        assert_eq!(refs, "0");
        let (created, _) = cell(&provider, 1, 3)?;
        assert_ne!(created, "");

        // Default sort: size descending.
        let mut rows = vec![0, 1];
        provider.sort_active_rows(&mut rows, 1, false)?;
        assert_eq!(rows, vec![1, 0], "size descending must be numeric");
        provider.sort_active_rows(&mut rows, 2, false)?;
        assert_eq!(rows, vec![1, 0], "ref count descending");
        provider.sort_active_rows(&mut rows, 3, true)?;
        assert_eq!(rows, vec![0, 1], "created ascending (0 first)");
        provider.sort_active_rows(&mut rows, 0, true)?;
        assert_eq!(rows, vec![1, 0], "name ascending: vol-big before vol-small");
        Ok(())
    }

    #[test]
    fn docker_delete_target_resolves_per_kind() {
        let inventory = sample_images_inventory();

        let image =
            DockerDeleteTarget::resolve(DockerResourceKind::Image, "bbbb2222cccc", &inventory);
        assert!(image.is_some(), "image target should resolve");
        let Some(image) = image else {
            return; // unreachable: the assert above fails first
        };
        assert_eq!(image.name, "repo:big");
        assert_eq!(image.size_bytes, 300);
        assert_eq!(image.extra_bytes, 0);

        let container =
            DockerDeleteTarget::resolve(DockerResourceKind::Container, "dddd4444", &inventory);
        assert!(container.is_some(), "container target should resolve");
        let Some(container) = container else {
            return; // unreachable: the assert above fails first
        };
        assert_eq!(container.name, "ctr-big");
        assert_eq!(container.size_bytes, 250);
        assert_eq!(container.extra_bytes, 150);

        let volume = DockerDeleteTarget::resolve(DockerResourceKind::Volume, "vol-big", &inventory);
        assert!(volume.is_some(), "volume target should resolve");
        let Some(volume) = volume else {
            return; // unreachable: the assert above fails first
        };
        assert_eq!(volume.id, "vol-big");
        assert_eq!(volume.size_bytes, 3000);

        // Unknown keys resolve to nothing.
        assert!(
            DockerDeleteTarget::resolve(DockerResourceKind::Image, "nope", &inventory).is_none()
        );
        assert!(
            DockerDeleteTarget::resolve(DockerResourceKind::Volume, "nope", &inventory).is_none()
        );
    }

    #[test]
    fn docker_table_state_syncs_default_sort_and_clears_on_resize() {
        let shared = Arc::new(crate::state::SharedState::new());
        let mut app = super::super::GuiApp::new(shared, None, None, false);

        for (section, sort_col) in [
            (DockerSection::Images, 2),
            (DockerSection::Containers, 2),
            (DockerSection::Volumes, 1),
        ] {
            app.sync_docker_table_state(section, 2);
            {
                let state = app.docker_view.table_state_mut(section);
                assert_eq!(
                    state.columns[sort_col].sort_up,
                    Some(false),
                    "default sort of {section:?} is column {sort_col}, descending"
                );
                assert!(state.filter_cache_dirty);
                state.selected_rows.insert(0);
            }
            app.sync_docker_table_state(section, 3);
            {
                let state = app.docker_view.table_state_mut(section);
                assert!(
                    state.selected_rows.is_empty(),
                    "a row-count change drops the index-based selection"
                );
                assert!(state.filter_cache_dirty);
                // An unchanged count keeps the user's sort and selection.
                state.columns[0].sort_up = Some(true);
                state.selected_rows.insert(1);
            }
            app.sync_docker_table_state(section, 3);
            let state = app.docker_view.table_state_mut(section);
            assert_eq!(state.columns[0].sort_up, Some(true));
            assert_eq!(state.selected_rows.len(), 1);
        }
    }

    // ---- Refresh regression (trait method is the preferred source) ----

    /// Collector whose trait `collect_inventory` returns a full, API-sourced
    /// inventory — what `start_docker_collection` now collects by contract.
    struct FullInventoryCollector {
        inventory: DockerInventory,
    }

    impl DockerCollector for FullInventoryCollector {
        fn detect_environment(&self) -> DockerEnvironment {
            DockerEnvironment::default()
        }

        fn collect_inventory(
            &self,
            _cancel: &AtomicBool,
        ) -> Result<DockerInventory, crate::WeshtatisticError> {
            Ok(self.inventory.clone())
        }
    }

    fn full_api_inventory() -> DockerInventory {
        let mut inventory = DockerInventory {
            root: Some(PathBuf::from("/var/lib/docker")),
            images: vec![crate::docker::ImageInfo {
                id: "img1".to_string(),
                tags: vec!["repo:full".to_string()],
                size_bytes: 42,
                ..crate::docker::ImageInfo::default()
            }],
            source: InventorySource::DockerApi,
            ..DockerInventory::default()
        };
        inventory.compute_totals();
        inventory
    }

    /// Regression: Refresh must publish what the trait's (now
    /// preferred-source) `collect_inventory` returns — a full API inventory —
    /// even when the snapshot already carries a poor disk-partial from the
    /// post-scan attach, and the provenance stamping must not clobber the
    /// API source (which only yields to `DiskContainerdPartial` when the
    /// partial-store warning is present).
    #[test]
    fn refresh_publishes_full_trait_inventory_without_clobbering_source()
    -> Result<(), crate::WeshtatisticError> {
        // Snapshot carrying the poor disk-partial (no images), as attached
        // after a scan on a containerd-store machine.
        let mut snapshot = docker_tree_snapshot();
        let mut partial = DockerInventory {
            root: Some(PathBuf::from("/var/lib/docker")),
            warnings: vec![InventoryWarning::ContainerdStorePartial],
            ..DockerInventory::default()
        };
        partial.compute_totals();
        let payload = partial.to_extension_payload()?;
        snapshot.extensions.insert(
            EXT_DOCKER_INVENTORY,
            EXT_DOCKER_INVENTORY_VERSION,
            payload.into(),
        );
        let app = app_with_snapshot(snapshot);
        assert!(
            app.shared_state
                .current_snapshot
                .load()
                .docker_inventory()
                .is_some(),
            "snapshot starts with the scan-attached partial inventory"
        );

        // Refresh: the worker thread calls the trait method (preferred
        // source); land its result in the pending slot like the thread does.
        let collector = FullInventoryCollector {
            inventory: full_api_inventory(),
        };
        *app.docker_view.pending.write() =
            Some(collector.collect_inventory(&AtomicBool::new(false)));

        assert!(app.publish_pending_inventory());

        let published = app
            .shared_state
            .current_snapshot
            .load()
            .docker_inventory()
            .unwrap_or_default();
        assert_eq!(
            published.images.len(),
            1,
            "refresh publishes the full trait inventory (images intact)"
        );
        assert_eq!(published.images[0].tags[0], "repo:full");
        assert_eq!(
            published.source,
            InventorySource::DockerApi,
            "API-sourced provenance survives (no warning present)"
        );
        assert!(published.collected_at > 0, "collection time stamped");
        assert!(
            published
                .warnings
                .iter()
                .all(|w| !matches!(w, InventoryWarning::ContainerdStorePartial)),
            "full inventory carries no partial-store warning"
        );
        Ok(())
    }

    /// The partial source stamping stays warning-driven: an API inventory
    /// that DOES carry the containerd-store warning is labeled partial.
    #[test]
    fn publish_marks_partial_source_only_from_warning() {
        let app = app_with_snapshot(docker_tree_snapshot());
        let mut inventory = full_api_inventory();
        inventory
            .warnings
            .push(InventoryWarning::ContainerdStorePartial);
        *app.docker_view.pending.write() = Some(Ok(inventory));

        assert!(app.publish_pending_inventory());

        let published = app
            .shared_state
            .current_snapshot
            .load()
            .docker_inventory()
            .unwrap_or_default();
        assert_eq!(published.source, InventorySource::DiskContainerdPartial);
    }

    // ---- Collapsible environment sections ----

    /// The environment sections render collapsed by default, and a user
    /// toggle (a header click) persists across frames via egui memory.
    #[test]
    fn docker_env_sections_collapse_and_persist() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 200.0));
        let mk_input = |time: f64, events: Vec<egui::Event>| egui::RawInput {
            screen_rect: Some(screen),
            time: Some(time),
            events,
            ..Default::default()
        };
        let show_section = |ui: &mut egui::Ui, shown: &mut bool| {
            env_collapsing_header(ui, "docker_env_probe", Cow::Borrowed("Section"), |ui| {
                ui.label("body");
                *shown = true;
            });
        };

        // Frame 1: layout pass — collapsed by default, body not rendered;
        // capture the header rect for the synthesized click.
        let mut body_shown = false;
        let mut header_rect = None;
        let mut output = ctx.run_ui(mk_input(0.0, Vec::new()), |ui| {
            let response =
                env_collapsing_header(ui, "docker_env_probe", Cow::Borrowed("Section"), |ui| {
                    ui.label("body");
                    body_shown = true;
                });
            header_rect = Some(response.header_response.rect);
        });
        output.textures_delta.clear();
        assert!(!body_shown, "section starts collapsed");
        let Some(header_rect) = header_rect else {
            return assert!(header_rect.is_some(), "header rendered");
        };
        let click_pos = header_rect.center();

        // Frames 2+3: press and release on the header (a click), which
        // toggles the section open and stores the state in egui memory.
        let click = |pressed: bool| {
            vec![
                egui::Event::PointerMoved(click_pos),
                egui::Event::PointerButton {
                    pos: click_pos,
                    pressed,
                    button: egui::PointerButton::Primary,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        for (time, pressed) in [(0.1, true), (0.2, false)] {
            let mut output = ctx.run_ui(mk_input(time, click(pressed)), |ui| {
                let mut shown = false;
                show_section(ui, &mut shown);
            });
            output.textures_delta.clear();
        }

        // Frame 4 (time advanced past the open animation): the body must be
        // visible with no further interaction — the toggle persisted.
        let mut body_shown = false;
        let mut output = ctx.run_ui(mk_input(1.0, Vec::new()), |ui| {
            show_section(ui, &mut body_shown);
        });
        output.textures_delta.clear();
        assert!(body_shown, "open state persists across frames");
    }

    // ---- Multi-delete modal/toast helpers ----

    #[test]
    fn modal_name_head_caps_at_eight() {
        let few: Vec<String> = (0..3).map(|i| format!("name-{i}")).collect();
        let (head, more) = modal_name_head(&few);
        assert_eq!(head.len(), 3);
        assert_eq!(more, 0);

        let many: Vec<String> = (0..12).map(|i| format!("name-{i}")).collect();
        let (head, more) = modal_name_head(&many);
        assert_eq!(head.len(), MODAL_NAME_CAP);
        assert_eq!(head[0], "name-0");
        assert_eq!(more, 4);

        let exact: Vec<String> = (0..MODAL_NAME_CAP).map(|i| format!("name-{i}")).collect();
        let (head, more) = modal_name_head(&exact);
        assert_eq!(head.len(), MODAL_NAME_CAP);
        assert_eq!(more, 0);
    }

    #[test]
    fn cap_error_text_truncates_long_messages() {
        let short = "image is being used by stopped container abc123";
        assert_eq!(cap_error_text(short, TOAST_ERROR_CAP), short);

        let long = "x".repeat(TOAST_ERROR_CAP + 50);
        let capped = cap_error_text(&long, TOAST_ERROR_CAP);
        assert_eq!(capped.chars().count(), TOAST_ERROR_CAP + 3);
        assert!(capped.ends_with("..."));
        // Multibyte input is cut on char boundaries, not bytes.
        let wide = "é".repeat(TOAST_ERROR_CAP + 10);
        let capped = cap_error_text(&wide, TOAST_ERROR_CAP);
        assert_eq!(capped.chars().count(), TOAST_ERROR_CAP + 3);
    }
}
