use std::{
    borrow::Cow,
    sync::{Arc, mpsc::Sender},
};

#[cfg(not(target_family = "wasm"))]
use std::path::Path;

use egui_table_kit::{
    error::TableError,
    operations::{OperationContext, TableOperation, TableOperationEnablement},
};
use fluent_zero::t;

use crate::{arena::FileArenaSnapshot, state::SharedState};

#[derive(Debug, Clone)]
pub enum BackgroundOpResult {
    Deletion {
        successfully_deleted: Vec<u32>,
        failures: Vec<(String, String, bool)>, // (path, error_msg, is_permission_denied)
        to_trash: bool,
        snapshot: Arc<FileArenaSnapshot>,
    },
    Hardlinking {
        successfully_linked: Vec<u32>,
        failures: Vec<(String, String, bool)>, // (path, error_msg, is_permission_denied)
        snapshot: Arc<FileArenaSnapshot>,
    },
    Softlinking {
        successfully_linked: Vec<u32>,
        failures: Vec<(String, String, bool)>, // (path, error_msg, is_permission_denied)
        snapshot: Arc<FileArenaSnapshot>,
    },
}

/// Decoupled actions sent from the `TableOperations` directly to the main `GuiApp` loop.
#[derive(Debug, Clone)]
pub enum AppCommand {
    RefreshSubtrees(Vec<u32>),
    ScrollToSelected,
    ShowTrashModal(Vec<u32>),
    ShowDeleteModal(Vec<u32>),
    /// Re-collect the docker inventory through the (preferred-source)
    /// collector (`DockerRefreshOp`).
    RefreshDockerInventory,
    /// Open the shared Docker resource deletion confirmation modal (dispatched
    /// by the `DockerDelete*Op`s; the existing modal + background deletion
    /// state machine execute the actual deletion). Carries every selected row
    /// of the op's table.
    ShowDockerDeleteResourceModal(Vec<crate::gui::docker::DockerDeleteTarget>),
    BackgroundOpCompleted(BackgroundOpResult),
    ZoomTreemap(u32),
    /// A snapshot file picked in the browser, delivered as raw bytes
    /// (used by the wasm frontend's async file picker).
    LoadSnapshotBytes {
        name: String,
        bytes: Vec<u8>,
    },
}

// Helper to retrieve the current snapshot safely
fn get_snapshot(shared_state: &Arc<SharedState>) -> Arc<FileArenaSnapshot> {
    shared_state.current_snapshot.load().clone()
}

// --- Focus in Treemap ---
#[derive(Debug)]
pub struct ZoomTreemapOp {
    shared_state: Arc<SharedState>,
    command_tx: Sender<AppCommand>,
}

impl ZoomTreemapOp {
    pub const fn new(shared_state: Arc<SharedState>, command_tx: Sender<AppCommand>) -> Self {
        Self {
            shared_state,
            command_tx,
        }
    }
}

impl TableOperation for ZoomTreemapOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-zoom-treemap")
    }

    fn icon(&self) -> &'static str {
        "🔍"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::OneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if state.selected_rows.len() == 1
            && let Some(idx) = state.selected_rows.iter().next()
        {
            let snapshot = get_snapshot(&self.shared_state);
            if (idx as usize) < snapshot.nodes.len() {
                let target = if snapshot.nodes[idx as usize].is_directory() {
                    idx
                } else {
                    snapshot.nodes[idx as usize].parent
                };
                if target != crate::arena::NO_INDEX {
                    return (true, Cow::Borrowed(""));
                }
            }
        }
        (false, t!("operation-one-selected"))
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let snapshot = get_snapshot(&self.shared_state);

        if let Some(idx) = ctx.data.selected_rows.iter().next()
            && (idx as usize) < snapshot.nodes.len()
        {
            let target = if snapshot.nodes[idx as usize].is_directory() {
                idx
            } else {
                snapshot.nodes[idx as usize].parent
            };

            if target != crate::arena::NO_INDEX {
                let _ = self.command_tx.send(AppCommand::ZoomTreemap(target));
                crate::gui::toast_info(t!("toast-zoomed-treemap"));
            }
        }
        Ok(())
    }
}

// --- Up One Level ---
#[derive(Debug)]
pub struct UpOneLevelOp {
    shared_state: Arc<SharedState>,
    command_tx: Sender<AppCommand>,
}

impl UpOneLevelOp {
    pub const fn new(shared_state: Arc<SharedState>, command_tx: Sender<AppCommand>) -> Self {
        Self {
            shared_state,
            command_tx,
        }
    }
}

impl TableOperation for UpOneLevelOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-up-one-level")
    }

    fn icon(&self) -> &'static str {
        "⏶"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::OneSelected
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let snapshot = get_snapshot(&self.shared_state);

        if let Some(idx) = ctx.data.selected_rows.iter().next()
            && (idx as usize) < snapshot.nodes.len()
        {
            let parent = snapshot.nodes[idx as usize].parent;
            if parent == crate::arena::NO_INDEX {
                crate::gui::toast_warning(t!("toast-already-root"));
            } else {
                ctx.data.selected_rows.clear();
                ctx.data.selected_rows.insert(parent);
                let _ = self.command_tx.send(AppCommand::ScrollToSelected);
                crate::gui::toast_info(t!("toast-navigated-up"));
            }
        }
        Ok(())
    }
}

// --- Refresh Entire Scan (Root) ---
#[derive(Debug)]
pub struct RefreshRootOp {
    shared_state: Arc<SharedState>,
    command_tx: Sender<AppCommand>,
}

impl RefreshRootOp {
    pub const fn new(shared_state: Arc<SharedState>, command_tx: Sender<AppCommand>) -> Self {
        Self {
            shared_state,
            command_tx,
        }
    }
}

impl TableOperation for RefreshRootOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-refresh-entire-scan")
    }

    fn icon(&self) -> &'static str {
        "🔁"
    }

    // Always enabled, regardless of whether a row is selected
    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::Always
    }

    fn evaluate_enablement(
        &self,
        _state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if crate::IS_NATIVE {
            (true, Cow::Borrowed(""))
        } else {
            (false, t!("web-not-available"))
        }
    }

    fn exec(&mut self, _ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let snapshot = get_snapshot(&self.shared_state);

        // Safety check to ensure we only refresh if a tree is actually loaded
        if !snapshot.nodes.is_empty() {
            // The root node is always strictly at index 0 in the arena
            let _ = self.command_tx.send(AppCommand::RefreshSubtrees(vec![0]));
            crate::gui::toast_info(t!("toast-refreshing-scan"));
        }

        Ok(())
    }
}

// --- Refresh Directory ---
#[derive(Debug)]
pub struct RefreshDirectoryOp {
    shared_state: Arc<SharedState>,
    command_tx: Sender<AppCommand>,
}

impl RefreshDirectoryOp {
    pub const fn new(shared_state: Arc<SharedState>, command_tx: Sender<AppCommand>) -> Self {
        Self {
            shared_state,
            command_tx,
        }
    }
}

impl TableOperation for RefreshDirectoryOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-refresh-directory")
    }

    fn icon(&self) -> &'static str {
        "🔄"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::AtLeastOneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if crate::IS_NATIVE {
            (
                !state.selected_rows.is_empty(),
                t!("operation-at-least-one"),
            )
        } else {
            (false, t!("web-not-available"))
        }
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let snapshot = get_snapshot(&self.shared_state);

        let dirs: Vec<u32> = ctx
            .data
            .selected_rows
            .iter()
            .filter(|&idx| {
                (idx as usize) < snapshot.nodes.len() && snapshot.nodes[idx as usize].is_directory()
            })
            .collect();

        if !dirs.is_empty() {
            let _ = self.command_tx.send(AppCommand::RefreshSubtrees(dirs));
            crate::gui::toast_info(t!("toast-refreshing-dir"));
        }
        Ok(())
    }
}

// --- Open File ---
#[derive(Debug)]
pub struct OpenFileOp {
    #[allow(dead_code)]
    shared_state: Arc<SharedState>,
}

impl OpenFileOp {
    #[must_use]
    pub const fn new(shared_state: Arc<SharedState>) -> Self {
        Self { shared_state }
    }
}

impl TableOperation for OpenFileOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-open-file")
    }

    fn icon(&self) -> &'static str {
        "↗"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::OneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if crate::IS_NATIVE {
            (state.selected_rows.len() == 1, t!("operation-one"))
        } else {
            (false, t!("web-not-available"))
        }
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        #[cfg(not(target_family = "wasm"))]
        {
            let snapshot = get_snapshot(&self.shared_state);

            if let Some(idx) = ctx.data.selected_rows.iter().next() {
                let path_str = snapshot.get_full_path(idx);
                let path = Path::new(&path_str);
                match crate::gui::reveal::open_file(path) {
                    Ok(()) => {
                        let path_lossy = path.to_string_lossy();
                        let cleaned_path = crate::arena::clean_unc_path(&path_lossy);
                        crate::gui::toast_info(
                            t!("toast-opened-file", { "path" => cleaned_path.as_ref() }),
                        );
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        crate::gui::toast_error(
                            t!("toast-failed-open-file", { "error" => err_msg.as_str() }),
                        );
                    }
                }
            }
        }
        let _ = ctx;
        Ok(())
    }
}

// --- Open in File Manager ---
#[derive(Debug)]
pub struct OpenFileManagerOp {
    #[allow(dead_code)]
    shared_state: Arc<SharedState>,
}

impl OpenFileManagerOp {
    #[must_use]
    pub const fn new(shared_state: Arc<SharedState>) -> Self {
        Self { shared_state }
    }
}

impl TableOperation for OpenFileManagerOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-open-file-manager")
    }

    fn icon(&self) -> &'static str {
        "🗁"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::OneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if crate::IS_NATIVE {
            (state.selected_rows.len() == 1, t!("operation-one"))
        } else {
            (false, t!("web-not-available"))
        }
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        #[cfg(not(target_family = "wasm"))]
        {
            let snapshot = get_snapshot(&self.shared_state);

            if let Some(idx) = ctx.data.selected_rows.iter().next() {
                let path_str = snapshot.get_full_path(idx);
                let path = Path::new(&path_str);
                match crate::gui::reveal::reveal_in_file_manager(path) {
                    Ok(()) => {
                        let path_lossy = path.to_string_lossy();
                        let cleaned_path = crate::arena::clean_unc_path(&path_lossy);
                        crate::gui::toast_info(
                            t!("toast-opened-manager", { "path" => cleaned_path.as_ref() }),
                        );
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        crate::gui::toast_error(
                            t!("toast-failed-open-manager", { "error" => err_msg.as_str() }),
                        );
                    }
                }
            }
        }
        let _ = ctx;
        Ok(())
    }
}

/// Compile-time check: was this binary built for the Mac App Store / Sandboxed channel?
pub const IS_MACOS_APPSTORE: bool = option_env!("EDIRSTAT_MACOS_APPSTORE").is_some()
    || option_env!("EDIRSTAT_APP_SANDBOX").is_some();

/// Returns true if running within a sandboxed macOS environment (detected at compile-time or runtime).
#[must_use]
#[allow(clippy::missing_const_for_fn)]
pub fn is_macos_sandbox() -> bool {
    if IS_MACOS_APPSTORE {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("APP_SANDBOX_CONTAINER_ID").is_some()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Returns true if terminal launching should be disabled (e.g., in macOS Sandbox or Web/WASM).
#[must_use]
pub fn is_terminal_disabled() -> bool {
    !crate::IS_NATIVE || is_macos_sandbox()
}

// --- Open Terminal Here ---
#[derive(Debug)]
pub struct OpenTerminalOp {
    #[allow(dead_code)]
    shared_state: Arc<SharedState>,
}

impl OpenTerminalOp {
    pub const fn new(shared_state: Arc<SharedState>) -> Self {
        Self { shared_state }
    }
}

impl TableOperation for OpenTerminalOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-open-terminal")
    }

    fn icon(&self) -> &'static str {
        "💻"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::OneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if is_terminal_disabled() {
            (false, t!("web-not-available"))
        } else {
            (state.selected_rows.len() == 1, t!("operation-one"))
        }
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        #[cfg(not(target_family = "wasm"))]
        {
            if is_terminal_disabled() {
                let _ = ctx;
                return Ok(());
            }

            let snapshot = get_snapshot(&self.shared_state);

            if let Some(idx) = ctx.data.selected_rows.iter().next()
                && (idx as usize) < snapshot.nodes.len()
            {
                let is_dir = snapshot.nodes[idx as usize].is_directory();
                let path_str = snapshot.get_full_path(idx);
                let path = Path::new(&path_str);
                let dir_to_open = if is_dir {
                    path
                } else {
                    path.parent().unwrap_or(path)
                };
                match super::open_terminal_at(dir_to_open) {
                    Ok(()) => {
                        let path_lossy = dir_to_open.to_string_lossy();
                        let cleaned_path = crate::arena::clean_unc_path(&path_lossy);
                        crate::gui::toast_info(
                            t!("toast-opened-terminal", { "path" => cleaned_path.as_ref() }),
                        );
                    }
                    Err(e) => {
                        let err_msg = e.to_string();
                        crate::gui::toast_error(
                            t!("toast-failed-open-terminal", { "error" => err_msg.as_str() }),
                        );
                    }
                }
            }
        }
        let _ = ctx;
        Ok(())
    }
}

// --- Copy Full Path ---
#[derive(Debug)]
pub struct CopyPathOp {
    shared_state: Arc<SharedState>,
}

impl CopyPathOp {
    pub const fn new(shared_state: Arc<SharedState>) -> Self {
        Self { shared_state }
    }
}

impl TableOperation for CopyPathOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-copy-path")
    }

    fn icon(&self) -> &'static str {
        "📎"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::AtLeastOneSelected
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let snapshot = get_snapshot(&self.shared_state);

        let mut paths = Vec::new();
        let mut selected: Vec<u32> = ctx.data.selected_rows.iter().collect();
        selected.sort_unstable();
        for idx in selected {
            let path_str = snapshot.get_full_path(idx);
            paths.push(crate::arena::clean_unc_path(&path_str).into_owned());
        }

        let num_paths = paths.len();
        ctx.ui.ctx().copy_text(paths.join("\n"));
        crate::gui::toast_success(t!("toast-copied-paths", { "count" => num_paths }));
        Ok(())
    }
}

// --- Copy Name Only ---
#[derive(Debug)]
pub struct CopyNameOp {
    shared_state: Arc<SharedState>,
}

impl CopyNameOp {
    pub const fn new(shared_state: Arc<SharedState>) -> Self {
        Self { shared_state }
    }
}

impl TableOperation for CopyNameOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-copy-name")
    }

    fn icon(&self) -> &'static str {
        "📋"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::AtLeastOneSelected
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let snapshot = get_snapshot(&self.shared_state);

        let mut names = Vec::new();
        let mut selected: Vec<u32> = ctx.data.selected_rows.iter().collect();
        selected.sort_unstable();
        for idx in selected {
            if (idx as usize) < snapshot.nodes.len() {
                let node = &snapshot.nodes[idx as usize];
                let name = snapshot.string_pool.get(node.name_id).unwrap_or("unknown");
                let cleaned_name = if node.parent_opt().is_none() {
                    crate::arena::clean_unc_path(name).into_owned()
                } else {
                    name.to_string()
                };
                names.push(cleaned_name);
            }
        }

        let num_names = names.len();
        ctx.ui.ctx().copy_text(names.join("\n"));
        crate::gui::toast_success(t!("toast-copied-names", { "count" => num_names }));
        Ok(())
    }
}

// --- Trash Selected ---
#[derive(Debug)]
pub struct TrashSelectedOp {
    command_tx: Sender<AppCommand>,
}

impl TrashSelectedOp {
    #[must_use]
    pub const fn new(command_tx: Sender<AppCommand>) -> Self {
        Self { command_tx }
    }
}

impl TableOperation for TrashSelectedOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-move-trash")
    }

    fn icon(&self) -> &'static str {
        "♻"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::AtLeastOneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if crate::IS_NATIVE {
            (
                !state.selected_rows.is_empty(),
                t!("operation-at-least-one"),
            )
        } else {
            (false, t!("web-not-available"))
        }
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let targets: Vec<u32> = ctx.data.selected_rows.iter().collect();
        let _ = self.command_tx.send(AppCommand::ShowTrashModal(targets));
        Ok(())
    }
}

// --- Permanently Delete Selected ---
#[derive(Debug)]
pub struct DeleteSelectedOp {
    command_tx: Sender<AppCommand>,
}

impl DeleteSelectedOp {
    #[must_use]
    pub const fn new(command_tx: Sender<AppCommand>) -> Self {
        Self { command_tx }
    }
}

impl TableOperation for DeleteSelectedOp {
    fn name(&self) -> Cow<'_, str> {
        t!("op-permanently-delete")
    }

    fn icon(&self) -> &'static str {
        "🗑"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::AtLeastOneSelected
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if crate::IS_NATIVE {
            (
                !state.selected_rows.is_empty(),
                t!("operation-at-least-one"),
            )
        } else {
            (false, t!("web-not-available"))
        }
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        let targets: Vec<u32> = ctx.data.selected_rows.iter().collect();
        let _ = self.command_tx.send(AppCommand::ShowDeleteModal(targets));
        Ok(())
    }
}

// --- Refresh Docker inventory (Docker tab toolbar) ---
/// Re-collects the Docker inventory through the collector's preferred source
/// (daemon API first, disk fallback).
///
/// The tab's Refresh/Analyze action, rendered as a toolbar op like the
/// explorer's refresh ops. Scoped to the docker tables by `TableState::id`
/// (any of the three sub-tables; the op is section-agnostic), native-only,
/// blocked while a collection runs, and requiring a live collector.
pub struct DockerRefreshOp {
    command_tx: Sender<AppCommand>,
    scanner: Option<Arc<dyn crate::ScanController>>,
    collection_running: Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for DockerRefreshOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // `dyn ScanController` is not Debug; the flag is the interesting part.
        f.debug_struct("DockerRefreshOp")
            .field(
                "collection_running",
                &self
                    .collection_running
                    .load(std::sync::atomic::Ordering::Acquire),
            )
            .finish_non_exhaustive()
    }
}

impl DockerRefreshOp {
    #[must_use]
    pub const fn new(
        command_tx: Sender<AppCommand>,
        scanner: Option<Arc<dyn crate::ScanController>>,
        collection_running: Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        Self {
            command_tx,
            scanner,
            collection_running,
        }
    }

    /// True when `state` is one of the docker sub-table states (refresh is
    /// section-agnostic, but must never light up on the explorer table).
    fn is_docker_table(state: &egui_table_kit::state::TableState) -> bool {
        matches!(
            state.id.as_str(),
            crate::gui::docker::DOCKER_IMAGES_TABLE_ID
                | crate::gui::docker::DOCKER_CONTAINERS_TABLE_ID
                | crate::gui::docker::DOCKER_VOLUMES_TABLE_ID
        )
    }
}

impl TableOperation for DockerRefreshOp {
    fn name(&self) -> Cow<'_, str> {
        t!("docker-refresh")
    }

    fn icon(&self) -> &'static str {
        // Same glyph as `RefreshDirectoryOp`, for consistency.
        "🔄"
    }

    fn enabled(&self) -> TableOperationEnablement {
        TableOperationEnablement::Always
    }

    fn evaluate_enablement(
        &self,
        state: &egui_table_kit::state::TableState,
    ) -> (bool, Cow<'static, str>) {
        if !Self::is_docker_table(state) {
            return (false, t!("operation-at-least-one-filtered"));
        }
        if !crate::IS_NATIVE {
            return (false, t!("web-not-available"));
        }
        if self
            .collection_running
            .load(std::sync::atomic::Ordering::Acquire)
        {
            return (false, t!("docker-analyzing"));
        }
        if self
            .scanner
            .as_ref()
            .is_none_or(|scanner| scanner.docker_collector().is_none())
        {
            return (false, t!("docker-no-install"));
        }
        (true, Cow::Borrowed(""))
    }

    fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
        if !Self::is_docker_table(ctx.data) {
            return Ok(());
        }
        let _ = self.command_tx.send(AppCommand::RefreshDockerInventory);
        Ok(())
    }
}

// --- Delete Docker resources (Docker tab > sub-tables) ---
/// Shared enablement of the docker delete ops: hard-scoped to the resource's
/// sub-table by `TableState::id`, native-only, blocked while any deletion
/// runs, and requiring at least one selected row.
fn docker_delete_enablement(
    kind: crate::gui::docker::DockerResourceKind,
    state: &egui_table_kit::state::TableState,
    deletion_running: &std::sync::atomic::AtomicBool,
    scanner: Option<&Arc<dyn crate::ScanController>>,
) -> (bool, Cow<'static, str>) {
    if state.id != kind.table_id() {
        return (false, t!("operation-one"));
    }
    if !crate::IS_NATIVE {
        return (false, t!("web-not-available"));
    }
    if deletion_running.load(std::sync::atomic::Ordering::Acquire) {
        return (false, t!(kind.busy_key()));
    }
    if scanner.is_none_or(|scanner| scanner.docker_collector().is_none()) {
        return (false, t!("docker-no-install"));
    }
    (
        !state.selected_rows.is_empty(),
        t!("operation-at-least-one"),
    )
}

/// Shared exec tail of the docker delete ops: resolves the display data of
/// every selected row from the snapshot-carried inventory and asks the app
/// to open the shared deletion confirmation modal. `keys` are the daemon
/// keys extracted from the selected rows (image/container IDs, or volume
/// names).
fn docker_delete_request_modal(
    shared_state: &Arc<SharedState>,
    command_tx: &Sender<AppCommand>,
    kind: crate::gui::docker::DockerResourceKind,
    keys: &[String],
) {
    let Some(inventory) = get_snapshot(shared_state).docker_inventory() else {
        return;
    };
    let targets: Vec<crate::gui::docker::DockerDeleteTarget> = keys
        .iter()
        .filter_map(|key| crate::gui::docker::DockerDeleteTarget::resolve(kind, key, &inventory))
        .collect();
    if !targets.is_empty() {
        let _ = command_tx.send(AppCommand::ShowDockerDeleteResourceModal(targets));
    }
}

macro_rules! docker_delete_op {
    ($op:ident, $kind:ident, $doc:literal) => {
        #[doc = $doc]
        ///
        /// Scoped to its sub-table twice over: it lives in the docker-scoped
        /// `TableOperations` registry (rendered only from the docker toolbar
        /// and context menu), and its enablement/exec gate on the table's
        /// `TableState::id`. Execution asks the app to open the shared
        /// `ActiveModal::DockerDeleteResource` confirmation modal; the
        /// background `DockerDeletionState` machine performs the deletion.
        pub struct $op {
            shared_state: Arc<SharedState>,
            command_tx: Sender<AppCommand>,
            scanner: Option<Arc<dyn crate::ScanController>>,
            deletion_running: Arc<std::sync::atomic::AtomicBool>,
        }

        impl std::fmt::Debug for $op {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // `dyn ScanController` is not Debug; the flag is the
                // interesting part.
                f.debug_struct(stringify!($op))
                    .field(
                        "deletion_running",
                        &self
                            .deletion_running
                            .load(std::sync::atomic::Ordering::Acquire),
                    )
                    .finish_non_exhaustive()
            }
        }

        impl $op {
            #[must_use]
            pub const fn new(
                shared_state: Arc<SharedState>,
                command_tx: Sender<AppCommand>,
                scanner: Option<Arc<dyn crate::ScanController>>,
                deletion_running: Arc<std::sync::atomic::AtomicBool>,
            ) -> Self {
                Self {
                    shared_state,
                    command_tx,
                    scanner,
                    deletion_running,
                }
            }
        }

        impl TableOperation for $op {
            fn name(&self) -> Cow<'_, str> {
                t!(crate::gui::docker::DockerResourceKind::$kind.title_key())
            }

            fn icon(&self) -> &'static str {
                "🗑"
            }

            fn enabled(&self) -> TableOperationEnablement {
                TableOperationEnablement::AtLeastOneSelected
            }

            fn evaluate_enablement(
                &self,
                state: &egui_table_kit::state::TableState,
            ) -> (bool, Cow<'static, str>) {
                docker_delete_enablement(
                    crate::gui::docker::DockerResourceKind::$kind,
                    state,
                    &self.deletion_running,
                    self.scanner.as_ref(),
                )
            }

            fn exec(&mut self, ctx: &mut OperationContext<'_, '_>) -> Result<(), TableError> {
                Self::exec_kind(&self.shared_state, &self.command_tx, ctx)
            }
        }

        impl $op {
            /// Resource-specific exec: extracts the daemon key from the first
            /// selected row, then shares the modal request with the siblings.
            fn exec_kind(
                shared_state: &Arc<SharedState>,
                command_tx: &Sender<AppCommand>,
                ctx: &mut OperationContext<'_, '_>,
            ) -> Result<(), TableError> {
                let kind = crate::gui::docker::DockerResourceKind::$kind;
                if ctx.data.id != kind.table_id() {
                    return Ok(());
                }

                // Multi-select delete: every selected row joins the batch.
                let keys = ctx.provider.map_selected_rows(ctx.data, Self::row_key)?;
                docker_delete_request_modal(shared_state, command_tx, kind, &keys);
                Ok(())
            }
        }
    };
}

docker_delete_op!(
    DockerDeleteImageOp,
    Image,
    "Deletes the selected image of the Docker tab's Images table."
);

impl DockerDeleteImageOp {
    /// The full image ID rides in the ID column's hover text.
    fn row_key(row: &dyn egui_table_kit::operations::Row) -> Result<String, TableError> {
        use egui_table_kit::operations::RowSliceExt as _;
        row.get_hover(crate::gui::docker::DOCKER_COL_ID)
            .map(std::borrow::Cow::into_owned)
    }
}

docker_delete_op!(
    DockerDeleteContainerOp,
    Container,
    "Deletes the selected container of the Docker tab's Containers table."
);

impl DockerDeleteContainerOp {
    /// The full container ID rides in the ID column's hover text.
    fn row_key(row: &dyn egui_table_kit::operations::Row) -> Result<String, TableError> {
        use egui_table_kit::operations::RowSliceExt as _;
        row.get_hover(crate::gui::docker::DOCKER_COL_ID)
            .map(std::borrow::Cow::into_owned)
    }
}

docker_delete_op!(
    DockerDeleteVolumeOp,
    Volume,
    "Deletes the selected volume of the Docker tab's Volumes table."
);

impl DockerDeleteVolumeOp {
    /// The volume name (the daemon key) is the name column's primary text.
    fn row_key(row: &dyn egui_table_kit::operations::Row) -> Result<String, TableError> {
        use egui_table_kit::operations::RowSliceExt as _;
        row.get_primary(crate::gui::docker::DOCKER_VOLUME_COL_NAME)
            .map(std::borrow::Cow::into_owned)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use eframe::egui;

    use super::*;

    struct NoopCollector;
    impl crate::docker::DockerCollector for NoopCollector {
        fn detect_environment(&self) -> crate::docker::DockerEnvironment {
            crate::docker::DockerEnvironment::default()
        }

        fn collect_inventory(
            &self,
            _cancel: &AtomicBool,
        ) -> Result<crate::docker::DockerInventory, crate::EdirstatError> {
            Err(crate::EdirstatError::Io(std::io::Error::from(
                std::io::ErrorKind::Unsupported,
            )))
        }
    }

    struct DockerScanner;
    impl crate::ScanController for DockerScanner {
        fn start_scan(&self, _path: std::path::PathBuf, _same_filesystem: bool) {}
        fn num_threads(&self) -> usize {
            1
        }
        fn docker_collector(&self) -> Option<Arc<dyn crate::docker::DockerCollector>> {
            Some(Arc::new(NoopCollector))
        }
    }

    fn docker_table_with_selection(
        table_id: &str,
        selected: &[u32],
    ) -> egui_table_kit::state::TableState {
        let mut state = egui_table_kit::state::TableState::new(table_id, 2);
        for &row in selected {
            state.selected_rows.insert(row);
        }
        state
    }

    /// Publishes a snapshot carrying `inventory` as the docker extension.
    fn shared_with_inventory(
        inventory: &crate::docker::DockerInventory,
    ) -> Result<Arc<SharedState>, TableError> {
        let payload = inventory
            .to_extension_payload()
            .map_err(|err| TableError::Generic(err.to_string()))?;
        let mut snapshot = crate::arena::FileArenaSnapshot {
            nodes: Arc::new(crate::arena::NodeStorage::Owned(Vec::new())),
            string_pool: Arc::new(crate::arena::StringPool::new()),
            dir_counts: Arc::new(Vec::new()),
            extensions: crate::extensions::ExtensionStore::default(),
        };
        snapshot.extensions.insert(
            crate::extensions::EXT_DOCKER_INVENTORY,
            crate::extensions::EXT_DOCKER_INVENTORY_VERSION,
            payload.into(),
        );
        let shared = Arc::new(SharedState::new());
        shared.store_snapshot(snapshot);
        Ok(shared)
    }

    /// Runs `op.exec` headlessly against the given table/provider.
    fn exec_op(
        op: &mut impl TableOperation,
        provider: &crate::gui::docker::DockerImagesProvider<'_>,
        state: &mut egui_table_kit::state::TableState,
    ) {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            let mut op_ctx = OperationContext {
                ui,
                data: state,
                provider,
            };
            let _ = op.exec(&mut op_ctx);
        });
        output.textures_delta.clear();
    }

    /// The inventory the docker op exec tests resolve against: two entries
    /// per section so multi-select payloads can be asserted.
    fn op_test_inventory() -> crate::docker::DockerInventory {
        crate::docker::DockerInventory {
            images: vec![
                crate::docker::ImageInfo {
                    id: "img1".to_string(),
                    tags: vec!["repo:tag".to_string()],
                    size_bytes: 42,
                    ..crate::docker::ImageInfo::default()
                },
                crate::docker::ImageInfo {
                    id: "img2".to_string(),
                    tags: vec!["repo:tag2".to_string()],
                    size_bytes: 43,
                    ..crate::docker::ImageInfo::default()
                },
            ],
            containers: vec![
                crate::docker::ContainerInfo {
                    id: "ctr1".to_string(),
                    name: "ctr-one".to_string(),
                    image_id: None,
                    rw_size_bytes: 25,
                    log_bytes: 15,
                    created: 0,
                },
                crate::docker::ContainerInfo {
                    id: "ctr2".to_string(),
                    name: "ctr-two".to_string(),
                    image_id: None,
                    rw_size_bytes: 35,
                    log_bytes: 5,
                    created: 0,
                },
            ],
            volumes: vec![
                crate::docker::VolumeInfo {
                    name: "vol1".to_string(),
                    size_bytes: 30,
                    ref_count: 0,
                    created: 0,
                },
                crate::docker::VolumeInfo {
                    name: "vol2".to_string(),
                    size_bytes: 40,
                    ref_count: 1,
                    created: 0,
                },
            ],
            ..crate::docker::DockerInventory::default()
        }
    }

    #[test]
    fn docker_refresh_op_enablement_is_scoped() {
        use crate::gui::docker::{
            DOCKER_CONTAINERS_TABLE_ID, DOCKER_IMAGES_TABLE_ID, DOCKER_VOLUMES_TABLE_ID,
        };

        let (tx, _rx) = std::sync::mpsc::channel();
        let idle = Arc::new(AtomicBool::new(false));
        let op = DockerRefreshOp::new(tx, Some(Arc::new(DockerScanner)), idle);

        // Section-agnostic: every docker sub-table state enables it.
        for table in [
            DOCKER_IMAGES_TABLE_ID,
            DOCKER_CONTAINERS_TABLE_ID,
            DOCKER_VOLUMES_TABLE_ID,
        ] {
            assert!(
                op.evaluate_enablement(&docker_table_with_selection(table, &[]))
                    .0,
                "refresh must be enabled on {table} without a selection"
            );
        }

        // The explorer table must never enable it.
        let explorer = egui_table_kit::state::TableState::new("edirstat_hierarchical_table", 0);
        assert!(!op.evaluate_enablement(&explorer).0);

        // No collector (native snapshot viewer): disabled.
        let no_collector = DockerRefreshOp::new(
            std::sync::mpsc::channel().0,
            None,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(
            !no_collector
                .evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[]))
                .0
        );

        // A running collection disables it with the busy reason.
        let busy = DockerRefreshOp::new(
            std::sync::mpsc::channel().0,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(true)),
        );
        let (enabled, reason) =
            busy.evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[]));
        assert!(!enabled);
        assert_eq!(reason, t!("docker-analyzing"));
    }

    #[test]
    fn docker_refresh_op_exec_issues_refresh_command() -> Result<(), TableError> {
        let inventory = op_test_inventory();
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerRefreshOp::new(
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let mut state =
            docker_table_with_selection(crate::gui::docker::DOCKER_IMAGES_TABLE_ID, &[]);
        exec_op(&mut op, &provider, &mut state);

        let command = rx
            .try_recv()
            .map_err(|err| TableError::Generic(err.to_string()))?;
        assert!(
            matches!(command, AppCommand::RefreshDockerInventory),
            "unexpected command: {command:?}"
        );
        Ok(())
    }

    #[test]
    fn docker_refresh_op_exec_ignores_foreign_table() {
        let inventory = op_test_inventory();
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerRefreshOp::new(
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let mut foreign = docker_table_with_selection("edirstat_hierarchical_table", &[]);
        exec_op(&mut op, &provider, &mut foreign);
        assert!(
            rx.try_recv().is_err(),
            "exec against a foreign table must not emit a refresh command"
        );
    }

    #[test]
    fn docker_delete_image_op_enablement_is_scoped() {
        use crate::gui::docker::{
            DOCKER_CONTAINERS_TABLE_ID, DOCKER_IMAGES_TABLE_ID, DOCKER_VOLUMES_TABLE_ID,
        };

        let (tx, _rx) = std::sync::mpsc::channel();
        let idle = Arc::new(AtomicBool::new(false));
        let op = DockerDeleteImageOp::new(
            Arc::new(SharedState::new()),
            tx.clone(),
            Some(Arc::new(DockerScanner)),
            idle,
        );

        // Wrong table: disabled even with a valid selection (the op must
        // never light up on the primary explorer table nor the sibling
        // docker tables).
        let mut foreign = egui_table_kit::state::TableState::new("edirstat_hierarchical_table", 2);
        foreign.selected_rows.insert(0);
        assert!(!op.evaluate_enablement(&foreign).0);
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(
                DOCKER_CONTAINERS_TABLE_ID,
                &[0]
            ))
            .0
        );
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(DOCKER_VOLUMES_TABLE_ID, &[0]))
                .0
        );

        // Own table, exactly one selected: enabled.
        assert!(
            op.evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[0]))
                .0
        );

        // Multi-select delete: several selected rows stay enabled.
        assert!(
            op.evaluate_enablement(&docker_table_with_selection(
                DOCKER_IMAGES_TABLE_ID,
                &[0, 1]
            ))
            .0
        );

        // Empty selection: disabled.
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[]))
                .0
        );

        // No collector (native snapshot viewer): disabled.
        let no_collector = DockerDeleteImageOp::new(
            Arc::new(SharedState::new()),
            tx,
            None,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(
            !no_collector
                .evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[0]))
                .0
        );

        // A running deletion disables the op with the busy reason.
        let busy = DockerDeleteImageOp::new(
            Arc::new(SharedState::new()),
            std::sync::mpsc::channel().0,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(true)),
        );
        let (enabled, reason) =
            busy.evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[0]));
        assert!(!enabled);
        assert_eq!(reason, t!("docker-deleting"));
    }

    #[test]
    fn docker_delete_container_op_enablement_is_scoped() {
        use crate::gui::docker::{
            DOCKER_CONTAINERS_TABLE_ID, DOCKER_IMAGES_TABLE_ID, DOCKER_VOLUMES_TABLE_ID,
        };

        let (tx, _rx) = std::sync::mpsc::channel();
        let op = DockerDeleteContainerOp::new(
            Arc::new(SharedState::new()),
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );

        // Own table, exactly one selected: enabled.
        assert!(
            op.evaluate_enablement(&docker_table_with_selection(
                DOCKER_CONTAINERS_TABLE_ID,
                &[0]
            ))
            .0
        );

        // Sibling docker tables and the explorer table: disabled.
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[0]))
                .0
        );
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(DOCKER_VOLUMES_TABLE_ID, &[0]))
                .0
        );
        let mut foreign = egui_table_kit::state::TableState::new("edirstat_hierarchical_table", 2);
        foreign.selected_rows.insert(0);
        assert!(!op.evaluate_enablement(&foreign).0);

        // Selection count and busy flag behave like the image op's.
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(
                DOCKER_CONTAINERS_TABLE_ID,
                &[]
            ))
            .0
        );
        assert!(
            op.evaluate_enablement(&docker_table_with_selection(
                DOCKER_CONTAINERS_TABLE_ID,
                &[0, 1]
            ))
            .0
        );
        let busy = DockerDeleteContainerOp::new(
            Arc::new(SharedState::new()),
            std::sync::mpsc::channel().0,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(true)),
        );
        let (enabled, reason) = busy.evaluate_enablement(&docker_table_with_selection(
            DOCKER_CONTAINERS_TABLE_ID,
            &[0],
        ));
        assert!(!enabled);
        assert_eq!(reason, t!("docker-deleting-container"));
    }

    #[test]
    fn docker_delete_volume_op_enablement_is_scoped() {
        use crate::gui::docker::{
            DOCKER_CONTAINERS_TABLE_ID, DOCKER_IMAGES_TABLE_ID, DOCKER_VOLUMES_TABLE_ID,
        };

        let (tx, _rx) = std::sync::mpsc::channel();
        let op = DockerDeleteVolumeOp::new(
            Arc::new(SharedState::new()),
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );

        // Own table, exactly one selected: enabled.
        assert!(
            op.evaluate_enablement(&docker_table_with_selection(DOCKER_VOLUMES_TABLE_ID, &[0]))
                .0
        );

        // Sibling docker tables: disabled.
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(DOCKER_IMAGES_TABLE_ID, &[0]))
                .0
        );
        assert!(
            !op.evaluate_enablement(&docker_table_with_selection(
                DOCKER_CONTAINERS_TABLE_ID,
                &[0]
            ))
            .0
        );

        let busy = DockerDeleteVolumeOp::new(
            Arc::new(SharedState::new()),
            std::sync::mpsc::channel().0,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(true)),
        );
        let (enabled, reason) =
            busy.evaluate_enablement(&docker_table_with_selection(DOCKER_VOLUMES_TABLE_ID, &[0]));
        assert!(!enabled);
        assert_eq!(reason, t!("docker-deleting-volume"));
    }

    /// Asserts that `op.exec` against the given table emits exactly one
    /// `ShowDockerDeleteResourceModal` carrying the expected targets.
    fn assert_exec_requests_modal(
        op: &mut impl TableOperation,
        provider: &crate::gui::docker::DockerImagesProvider<'_>,
        table_id: &str,
        selected: &[u32],
        rx: &std::sync::mpsc::Receiver<AppCommand>,
        expect: &[crate::gui::docker::DockerDeleteTarget],
    ) -> Result<(), TableError> {
        let mut state = docker_table_with_selection(table_id, selected);
        exec_op(op, provider, &mut state);

        let command = rx
            .try_recv()
            .map_err(|err| TableError::Generic(err.to_string()))?;
        let debug = format!("{command:?}");
        let AppCommand::ShowDockerDeleteResourceModal(targets) = command else {
            return Err(TableError::Generic(debug));
        };
        assert_eq!(targets, expect);
        Ok(())
    }

    #[test]
    fn docker_delete_image_op_exec_requests_modal() -> Result<(), TableError> {
        let inventory = op_test_inventory();
        let shared = shared_with_inventory(&inventory)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerDeleteImageOp::new(
            shared,
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );

        assert_exec_requests_modal(
            &mut op,
            &provider,
            crate::gui::docker::DOCKER_IMAGES_TABLE_ID,
            &[0],
            &rx,
            &[crate::gui::docker::DockerDeleteTarget {
                kind: crate::gui::docker::DockerResourceKind::Image,
                id: "img1".to_string(),
                name: "repo:tag".to_string(),
                size_bytes: 42,
                extra_bytes: 0,
            }],
        )
    }

    #[test]
    fn docker_delete_image_op_exec_carries_all_selected_rows() -> Result<(), TableError> {
        let inventory = op_test_inventory();
        let shared = shared_with_inventory(&inventory)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerDeleteImageOp::new(
            shared,
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );

        // Multi-select: both rows join the batch in selection order.
        assert_exec_requests_modal(
            &mut op,
            &provider,
            crate::gui::docker::DOCKER_IMAGES_TABLE_ID,
            &[0, 1],
            &rx,
            &[
                crate::gui::docker::DockerDeleteTarget {
                    kind: crate::gui::docker::DockerResourceKind::Image,
                    id: "img1".to_string(),
                    name: "repo:tag".to_string(),
                    size_bytes: 42,
                    extra_bytes: 0,
                },
                crate::gui::docker::DockerDeleteTarget {
                    kind: crate::gui::docker::DockerResourceKind::Image,
                    id: "img2".to_string(),
                    name: "repo:tag2".to_string(),
                    size_bytes: 43,
                    extra_bytes: 0,
                },
            ],
        )
    }

    #[test]
    fn docker_delete_container_op_exec_requests_modal() -> Result<(), TableError> {
        let inventory = op_test_inventory();
        let shared = shared_with_inventory(&inventory)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerDeleteContainerOp::new(
            shared,
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Containers,
            crate::time_utils::TimeFormat::default(),
        );

        // The modal carries the container name plus rw/log sizes.
        assert_exec_requests_modal(
            &mut op,
            &provider,
            crate::gui::docker::DOCKER_CONTAINERS_TABLE_ID,
            &[0],
            &rx,
            &[crate::gui::docker::DockerDeleteTarget {
                kind: crate::gui::docker::DockerResourceKind::Container,
                id: "ctr1".to_string(),
                name: "ctr-one".to_string(),
                size_bytes: 25,
                extra_bytes: 15,
            }],
        )
    }

    #[test]
    fn docker_delete_volume_op_exec_requests_modal() -> Result<(), TableError> {
        let inventory = op_test_inventory();
        let shared = shared_with_inventory(&inventory)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerDeleteVolumeOp::new(
            shared,
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Volumes,
            crate::time_utils::TimeFormat::default(),
        );

        assert_exec_requests_modal(
            &mut op,
            &provider,
            crate::gui::docker::DOCKER_VOLUMES_TABLE_ID,
            &[0],
            &rx,
            &[crate::gui::docker::DockerDeleteTarget {
                kind: crate::gui::docker::DockerResourceKind::Volume,
                id: "vol1".to_string(),
                name: "vol1".to_string(),
                size_bytes: 30,
                extra_bytes: 0,
            }],
        )
    }

    #[test]
    fn docker_delete_image_op_exec_ignores_foreign_table() -> Result<(), TableError> {
        let inventory = op_test_inventory();
        let shared = shared_with_inventory(&inventory)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let mut op = DockerDeleteImageOp::new(
            shared,
            tx,
            Some(Arc::new(DockerScanner)),
            Arc::new(AtomicBool::new(false)),
        );
        let provider = crate::gui::docker::DockerImagesProvider::new(
            &inventory,
            crate::gui::docker::DockerSection::Images,
            crate::time_utils::TimeFormat::default(),
        );
        let mut foreign =
            docker_table_with_selection(crate::gui::docker::DOCKER_CONTAINERS_TABLE_ID, &[0]);
        exec_op(&mut op, &provider, &mut foreign);
        assert!(
            rx.try_recv().is_err(),
            "exec against a foreign table must not emit a modal command"
        );
        Ok(())
    }

    #[test]
    fn test_macos_appstore_detection() {
        assert_eq!(
            IS_MACOS_APPSTORE,
            option_env!("EDIRSTAT_MACOS_APPSTORE").is_some()
                || option_env!("EDIRSTAT_APP_SANDBOX").is_some()
        );
    }

    #[test]
    fn test_is_terminal_disabled() {
        if !crate::IS_NATIVE || IS_MACOS_APPSTORE {
            assert!(is_terminal_disabled());
        }
    }
}
