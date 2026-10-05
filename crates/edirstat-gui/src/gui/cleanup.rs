//! Cleanup tab.
//!
//! After a scan, a static rulepack ([`crate::cleanup`]) flags well-known
//! reclaimable directories — build artifacts and language caches, plus a few
//! tool-owned locations (package-manager caches, the systemd journal). The
//! analysis is pure snapshot logic: it runs on a background thread against
//! the current snapshot and publishes a [`CleanupReport`] whose findings are
//! displayed in an `egui-table-kit` table (the same framework as the Docker
//! tab), one row per finding with a per-row action and no selection or
//! bulk-action UI of any kind.
//!
//! Findings carry arena node indices, so a report is only ever rendered for
//! the exact snapshot it was computed from (`analyzed_ptr`); any snapshot
//! change (rescan, trash-induced arena splice, docker-merge republish, a
//! loaded `.edst`) puts the tab back into an "analyzing…" placeholder and
//! re-runs the analysis. Two tiers drive the allowed actions:
//!
//! - [`CleanupTier::Regenerable`] rows offer the same two icon actions as
//!   the primary table's trash/delete `TableOperation`s (♻ move-to-trash,
//!   🗑 permanently-delete), dispatched through the shared
//!   [`crate::gui::operations::AppCommand::ShowTrashModal`] /
//!   [`crate::gui::operations::AppCommand::ShowDeleteModal`] pipeline
//!   (confirmation modal + deletion + arena splice).
//! - [`CleanupTier::ToolMediated`] rows are strictly display-only: the
//!   owning tool's cleanup command plus a clipboard Copy icon button. There
//!   is deliberately no delete/trash affordance anywhere on these rows.
//!
//! The location column is a click-to-jump link: it selects the directory in
//! the primary explorer table, expands its ancestor chain, and scrolls it
//! into view (via [`crate::gui::operations::AppCommand::RevealInExplorer`])
//! without leaving this tab.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use eframe::egui;
use egui_table_kit::{
    error::TableError,
    operations::{BorrowedRow, HeaderIter, RowCallback, TableCell, TableProvider},
    state::TableState,
    table::TableKit,
};
use fluent_zero::t;

use crate::arena::FileArenaSnapshot;
use crate::cleanup::{CleanupContext, CleanupReport, CleanupTier};

/// `TableState::id` of the findings table.
pub(crate) const CLEANUP_TABLE_ID: &str = "cleanup_table";

/// Number of columns the findings table renders.
const CLEANUP_COLUMN_COUNT: usize = 6;

/// Scan-completion toast only fires when at least this much space is
/// reclaimable (100 MiB): below that it is noise.
const TOAST_MIN_BYTES: u64 = 100 * 1_048_576;

/// Directories younger than this render as "Recent".
const FRESH_AGE_DAYS: u32 = 7;

/// GUI state for the Cleanup tab, owned by `GuiApp` (mirrors the docker
/// tab's "state lives on the app" pattern).
pub(crate) struct CleanupViewState {
    /// The published report; `Some` only together with a matching
    /// `analyzed_ptr`. Findings reference arena indices, so this must never
    /// be rendered against a different snapshot.
    report: Option<Arc<CleanupReport>>,
    /// Latest finished analysis not yet published: the `Arc` data pointer of
    /// the snapshot it was computed from, plus the report itself.
    pending: Arc<parking_lot::RwLock<Option<(usize, CleanupReport)>>>,
    /// True while the background analysis thread is running.
    running: Arc<AtomicBool>,
    /// `Arc::as_ptr(&snapshot.nodes)` of the snapshot `report` was computed
    /// from; 0 marks "no report".
    analyzed_ptr: usize,
    /// Set when the analysis was kicked by scan completion; consumed (either
    /// way) when the report is published.
    toast_pending: bool,
    /// egui-table-kit state of the findings table (column layout; no
    /// selection is kept — actions are per-row buttons).
    table_state: TableState,
    /// Row count the table state was last synced against; a change marks the
    /// view dirty and drops the (index-based) selection.
    row_count: usize,
}

impl Default for CleanupViewState {
    fn default() -> Self {
        Self {
            report: None,
            pending: Arc::new(parking_lot::RwLock::new(None)),
            running: Arc::new(AtomicBool::new(false)),
            analyzed_ptr: 0,
            toast_pending: false,
            table_state: TableState::new(CLEANUP_TABLE_ID, 0),
            row_count: 0,
        }
    }
}

impl CleanupViewState {
    /// Drops the report and all analysis bookkeeping. Called from
    /// `GuiApp::reset_state` (cancelled scans, closed snapshots, new scans).
    /// A background thread already in flight still finishes, but its report
    /// is never rendered: the snapshot pointer it records no longer matches.
    pub(crate) fn reset(&mut self) {
        self.report = None;
        *self.pending.write() = None;
        self.running.store(false, Ordering::SeqCst);
        self.analyzed_ptr = 0;
        self.toast_pending = false;
        self.row_count = 0;
    }
}

/// Per-finding display data, precomputed once per report so the table cells
/// never touch the snapshot: findings are only rendered against the analyzed
/// snapshot anyway, and precomputing keeps the cell closures borrow-free.
struct CleanupRow {
    /// Arena index of the flagged directory (valid only for `analyzed_ptr`'s
    /// snapshot — enforced by the render path before any row is built).
    node_index: u32,
    tier: CleanupTier,
    /// Rule title; rules carry English `&'static str` display data by design
    /// (same precedent as the hardcoded-English deletion toasts).
    title: &'static str,
    /// What it costs to get the data back (button/tooltip text).
    hint: &'static str,
    /// Full path of the flagged directory.
    path: String,
    size_str: String,
    age_str: String,
    /// Raw size / age for numeric column sorting.
    sort_size: u64,
    sort_age_days: u32,
    /// The owning tool's cleanup command (tool-mediated rows only).
    command: Option<&'static str>,
}

/// egui-table-kit [`TableProvider`] over the cleanup findings.
///
/// Rows are flat and indexed directly into the precomputed row vec; findings
/// arrive sorted by descending score and the provider adds numeric-aware
/// sorting per column (the kit's default sort is string-based, which would
/// order "9 MB" above "10 MB").
struct CleanupTableProvider<'a> {
    rows: &'a [CleanupRow],
}

impl<'a> CleanupTableProvider<'a> {
    #[must_use]
    const fn new(rows: &'a [CleanupRow]) -> Self {
        Self { rows }
    }

    /// Sort ranking for the tier column: regenerable first.
    const fn tier_rank(row: &CleanupRow) -> u8 {
        match row.tier {
            CleanupTier::Regenerable => 0,
            CleanupTier::ToolMediated => 1,
        }
    }

    /// Orders `rows` with `cmp`, honoring the ascending/descending direction.
    fn sort_rows(
        &self,
        rows: &mut [usize],
        ascending: bool,
        cmp: impl Fn(&CleanupRow, &CleanupRow) -> std::cmp::Ordering,
    ) {
        rows.sort_by(|a, b| {
            let ord = cmp(&self.rows[*a], &self.rows[*b]);
            if ascending { ord } else { ord.reverse() }
        });
    }
}

impl TableProvider for CleanupTableProvider<'_> {
    fn column_count(&self) -> usize {
        CLEANUP_COLUMN_COUNT
    }

    fn header(&self, index: usize) -> Option<Cow<'_, str>> {
        let key = match index {
            0 => "cleanup-hdr-tier",
            1 => "cleanup-hdr-title",
            2 => "cleanup-hdr-path",
            3 => "explorer-hdr-size",
            4 => "cleanup-hdr-age",
            5 => "cleanup-hdr-action",
            _ => return None,
        };
        Some(t!(key))
    }

    fn headers(&self) -> HeaderIter<'_> {
        HeaderIter::new(self)
    }

    fn row_count(&self) -> usize {
        self.rows.len()
    }

    fn cell_at(
        &self,
        row_index: usize,
        col_index: usize,
    ) -> Result<Option<TableCell<'_>>, TableError> {
        let Some(row) = self.rows.get(row_index) else {
            return Ok(None);
        };
        let cell: TableCell<'_> = match col_index {
            0 => (Cow::Owned(tier_label(row.tier).into_owned()), None),
            // Hover carries the restore hint; the title itself is static English.
            1 => (Cow::Borrowed(row.title), Some(Cow::Borrowed(row.hint))),
            2 => (
                Cow::Borrowed(row.path.as_str()),
                Some(Cow::Borrowed(row.path.as_str())),
            ),
            3 => (Cow::Borrowed(row.size_str.as_str()), None),
            4 => (Cow::Borrowed(row.age_str.as_str()), None),
            // Actions render through the custom cell callback.
            5 => (Cow::Borrowed(""), None),
            _ => return Ok(None),
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
        match col_index {
            0 => self.sort_rows(active_rows, ascending, |a, b| {
                Self::tier_rank(a).cmp(&Self::tier_rank(b))
            }),
            1 => self.sort_rows(active_rows, ascending, |a, b| a.title.cmp(b.title)),
            2 => self.sort_rows(active_rows, ascending, |a, b| a.path.cmp(&b.path)),
            3 => self.sort_rows(active_rows, ascending, |a, b| a.sort_size.cmp(&b.sort_size)),
            4 => self.sort_rows(active_rows, ascending, |a, b| {
                a.sort_age_days.cmp(&b.sort_age_days)
            }),
            _ => {}
        }
        Ok(())
    }
}

#[must_use]
fn tier_label(tier: CleanupTier) -> Cow<'static, str> {
    match tier {
        CleanupTier::Regenerable => t!("cleanup-tier-regenerable"),
        CleanupTier::ToolMediated => t!("cleanup-tier-tool"),
    }
}

#[must_use]
fn format_size(bytes: u64) -> String {
    prettier_bytes::ByteFormatter::new()
        .format(bytes)
        .to_string()
}

/// Age label: fresh (or unknown) timestamps render as "Recent".
fn age_label(age_days: Option<u32>) -> (String, u32) {
    match age_days {
        Some(days) if days >= FRESH_AGE_DAYS => (
            t!("cleanup-age-days", { "days" => days }).into_owned(),
            days,
        ),
        _ => (t!("cleanup-age-fresh").into_owned(), 0),
    }
}

/// Render scale for the action cell's trash/delete icon buttons. The default
/// toolbar size measures ~25px tall against the app font stack — too tall
/// for the 28px table rows — while 0.7x lands on egui's 18px minimum button
/// height (an 18.8px-wide pair fits the action column with room to spare).
const OP_BUTTON_SCALE: f32 = 0.7;

/// Column layout of the findings table (resizable).
fn cleanup_table_columns() -> Vec<egui_table_kit::layout::Column> {
    #[allow(clippy::shadow_unrelated)]
    let mk = |w: f32, min: f32, max: f32| {
        egui_table_kit::layout::Column::new(w)
            .range(min..=max)
            .resizable(true)
    };
    vec![
        mk(90.0, 70.0, 150.0),   // Tier
        mk(200.0, 100.0, 400.0), // What
        mk(320.0, 120.0, 800.0), // Location
        mk(90.0, 50.0, 200.0),   // Size
        mk(90.0, 60.0, 160.0),   // Age
        mk(280.0, 120.0, 600.0), // Action
    ]
}

/// Cell customization layered on the kit's default label rendering: the tier
/// badge column gets its tier color, the location column becomes a
/// click-to-jump link into the explorer tree, and the action column renders
/// the per-row controls — the primary table's ♻/🗑 trash/delete icon buttons
/// for regenerable findings, or the owning tool's command with a clipboard
/// Copy icon button for tool-mediated findings. Every other column renders
/// with the kit default.
fn cleanup_custom_cell(
    ui: &mut egui::Ui,
    cell_info: &egui_table_kit::layout::CellInfo,
    row_data: &dyn egui_table_kit::operations::Row,
    _text_color: egui::Color32,
    rows: &[CleanupRow],
    command_tx: &std::sync::mpsc::Sender<crate::gui::operations::AppCommand>,
    is_native: bool,
) -> Option<egui::Response> {
    let row_idx = row_data.row_index()?;
    let row = rows.get(row_idx)?;

    match cell_info.col_nr {
        0 => {
            let (label, color) = match row.tier {
                CleanupTier::Regenerable => {
                    (tier_label(row.tier), crate::colors::COLOR_SCAN_COMPLETE)
                }
                CleanupTier::ToolMediated => {
                    (tier_label(row.tier), crate::colors::COLOR_DUPLICATE_ORANGE)
                }
            };
            Some(ui.colored_label(color, egui::RichText::new(label).strong()))
        }
        2 => {
            // Click-to-jump link (same hyperlink styling as the treemap
            // breadcrumbs): reveals the directory in the primary explorer
            // table without leaving this tab. Hover shows the untruncated
            // path.
            let response = ui
                .add(
                    egui::Label::new(
                        egui::RichText::new(&row.path).color(ui.visuals().hyperlink_color),
                    )
                    .sense(egui::Sense::click())
                    .wrap_mode(egui::TextWrapMode::Truncate),
                )
                .on_hover_text(&row.path);
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.clicked() {
                let _ = command_tx.send(crate::gui::operations::AppCommand::RevealInExplorer(
                    row.node_index,
                ));
            }
            Some(response)
        }
        col if col == CLEANUP_COLUMN_COUNT - 1 => {
            // Action column: exactly the tier's controls, nothing else.
            // Tier 2 rows deliberately have no deletion affordance of any
            // kind.
            match row.tier {
                CleanupTier::Regenerable => {
                    // The same two icon buttons — and the same renderer — as
                    // the primary table's trash/delete TableOperations
                    // (♻ "Move to Trash", 🗑 "Permanently Delete"), wired to
                    // the same confirmation-modal pipeline. The icons mirror
                    // TrashSelectedOp/DeleteSelectedOp::icon(). Rendered at
                    // OP_BUTTON_SCALE: the default toolbar size overflows
                    // the 28px rows.
                    let response = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            let trash = super::render_custom_op_button_scaled(
                                ui,
                                "♻",
                                t!("op-move-trash").as_ref(),
                                is_native,
                                t!("web-not-available").as_ref(),
                                OP_BUTTON_SCALE,
                            );
                            if trash.clicked() {
                                let _ = command_tx.send(
                                    crate::gui::operations::AppCommand::ShowTrashModal(vec![
                                        row.node_index,
                                    ]),
                                );
                            }
                            let delete = super::render_custom_op_button_scaled(
                                ui,
                                "🗑",
                                t!("op-permanently-delete").as_ref(),
                                is_native,
                                t!("web-not-available").as_ref(),
                                OP_BUTTON_SCALE,
                            );
                            if delete.clicked() {
                                let _ = command_tx.send(
                                    crate::gui::operations::AppCommand::ShowDeleteModal(vec![
                                        row.node_index,
                                    ]),
                                );
                            }
                        })
                        .response;
                    Some(response)
                }
                CleanupTier::ToolMediated => {
                    let cmd = row.command?;
                    let hover = format!("{}\n{cmd}", t!("cleanup-run-command"));
                    let response = ui
                        .horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            // Clipboard icon button first — the same glyph the
                            // copy TableOperations use (CopyNameOp/CopyPathOp),
                            // at the same renderer/scale as the tier-1
                            // action icons — so the command can use the
                            // remaining (possibly narrow) width. No deletion
                            // affordance on this row, by design.
                            let copy = super::render_custom_op_button_scaled(
                                ui,
                                "📋",
                                t!("cleanup-copy").as_ref(),
                                true,
                                "",
                                OP_BUTTON_SCALE,
                            );
                            if copy.clicked() {
                                ui.ctx().copy_text(cmd.to_string());
                                crate::gui::toast_info(t!("cleanup-copied"));
                            }
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            ui.add(
                                egui::Label::new(egui::RichText::new(cmd).monospace())
                                    .selectable(false)
                                    .wrap_mode(egui::TextWrapMode::Truncate),
                            )
                            .on_hover_text(hover);
                        })
                        .response;
                    Some(response)
                }
            }
        }
        _ => None,
    }
}

impl super::GuiApp {
    /// Starts the background cleanup analysis over the current snapshot,
    /// unless one is already running or there is nothing to analyze. When
    /// `toast` is set (scan-completion kick), a finished non-trivial report
    /// raises the one-shot reclaimable-space notification on publish.
    pub(crate) fn start_cleanup_analysis(&mut self, toast: bool) {
        if self.cleanup_view.running.load(Ordering::Acquire) {
            return;
        }
        let snapshot = self.shared_state.current_snapshot.load();
        if snapshot.nodes.is_empty() {
            return;
        }
        if toast {
            self.cleanup_view.toast_pending = true;
        }

        let slot = self.cleanup_view.pending.clone();
        let running = self.cleanup_view.running.clone();
        running.store(true, Ordering::SeqCst);

        #[cfg(not(target_family = "wasm"))]
        std::thread::spawn(move || {
            let ptr = Arc::as_ptr(&snapshot.nodes) as usize;
            let report = crate::cleanup::analyze(&snapshot, &CleanupContext::current());
            *slot.write() = Some((ptr, report));
            running.store(false, Ordering::Release);
        });

        #[cfg(target_family = "wasm")]
        {
            // No threads on wasm; the analysis is a cheap in-memory sweep, so
            // run it inline and let the next publish pick it up.
            let ptr = Arc::as_ptr(&snapshot.nodes) as usize;
            let report = crate::cleanup::analyze(&snapshot, &CleanupContext::current());
            *slot.write() = Some((ptr, report));
            running.store(false, Ordering::SeqCst);
        }
    }

    /// Drains a finished analysis into `report`, recording which snapshot it
    /// belongs to. An empty pending slot is a cheap no-op on every frame.
    /// Consumes `toast_pending` either way; the toast itself only fires for
    /// a non-empty report reclaiming at least [`TOAST_MIN_BYTES`].
    ///
    /// Called every frame from the update loop (so the notification lands
    /// without the tab being open) and idempotently from the tab render.
    pub(crate) fn publish_pending_report(&mut self) -> bool {
        let Some((ptr, report)) = self.cleanup_view.pending.write().take() else {
            return false;
        };
        self.cleanup_view.analyzed_ptr = ptr;
        let toast = self.cleanup_view.toast_pending;
        self.cleanup_view.toast_pending = false;

        let total = report
            .regenerable_bytes
            .saturating_add(report.tool_mediated_bytes);
        let fire_toast = toast && !report.is_empty() && total >= TOAST_MIN_BYTES;
        let size_str = format_size(total);
        self.cleanup_view.report = Some(Arc::new(report));
        if fire_toast {
            crate::gui::toast_info(t!("cleanup-toast", { "size" => size_str }));
        }
        true
    }

    pub(crate) fn render_cleanup_tab(&mut self, ui: &mut egui::Ui, snapshot: &FileArenaSnapshot) {
        if self.cleanup_view.running.load(Ordering::Acquire) {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }

        ui.label(t!("cleanup-desc"));
        ui.separator();

        // The update loop already publishes every frame; keep the tab honest
        // when rendered outside it (e.g. a host embedding the panel).
        self.publish_pending_report();

        if snapshot.nodes.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(t!("cleanup-empty"));
            });
            return;
        }

        // A running scan re-indexes the arena continuously; wait for it to
        // finish — the scan-completion hook kicks the analysis once.
        if self.shared_state.is_scanning.load(Ordering::SeqCst) {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!("cleanup-analyzing"));
            });
            return;
        }

        // Findings store arena node indices, which shift whenever the arena
        // is spliced or rebuilt. Only a report computed for *this* snapshot
        // may render; anything else shows a placeholder and re-analyzes.
        let current_ptr = Arc::as_ptr(&snapshot.nodes) as usize;
        if current_ptr != self.cleanup_view.analyzed_ptr {
            if !self.cleanup_view.running.load(Ordering::Acquire) {
                self.start_cleanup_analysis(false);
            }
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(t!("cleanup-analyzing"));
            });
            return;
        }

        let report = match self.cleanup_view.report.clone() {
            Some(report) if !report.is_empty() => report,
            _ => {
                ui.centered_and_justified(|ui| {
                    ui.label(t!("cleanup-empty"));
                });
                return;
            }
        };

        let total = report
            .regenerable_bytes
            .saturating_add(report.tool_mediated_bytes);
        ui.horizontal_wrapped(|ui| {
            ui.label(t!("cleanup-summary", {
                "count" => report.findings.len(),
                "size" => format_size(total),
            }));
            ui.weak(t!("cleanup-regenerable-total", {
                "size" => format_size(report.regenerable_bytes),
            }));
            ui.weak(t!("cleanup-tool-total", {
                "size" => format_size(report.tool_mediated_bytes),
            }));
        });
        ui.add_space(4.0);

        let rows: Vec<CleanupRow> = report
            .findings
            .iter()
            .map(|finding| {
                let (age_str, sort_age_days) = age_label(finding.age_days);
                CleanupRow {
                    node_index: finding.node_index,
                    tier: finding.rule.tier,
                    title: finding.rule.title,
                    hint: finding.rule.restore_hint,
                    path: snapshot.get_full_path(finding.node_index),
                    size_str: format_size(finding.size),
                    age_str,
                    sort_size: finding.size,
                    sort_age_days,
                    command: finding.rule.command,
                }
            })
            .collect();
        self.draw_cleanup_table(ui, &rows);
    }

    /// Keeps the findings table state consistent with the rendered rows:
    /// establishes the column layout and drops the (index-based) selection
    /// whenever the row count changes underneath it.
    fn sync_cleanup_table_state(&mut self, rows: usize) {
        let count_changed = self.cleanup_view.row_count != rows;
        if count_changed {
            self.cleanup_view.row_count = rows;
        }
        let state = &mut self.cleanup_view.table_state;
        if state.columns.len() < CLEANUP_COLUMN_COUNT {
            state.columns.resize_with(
                CLEANUP_COLUMN_COUNT,
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
    }

    /// Renders the findings table on the egui-table-kit framework (same as
    /// the Docker tab's tables): per-row action buttons via the custom cell
    /// callback, no context menu, no selection-dependent operations.
    fn draw_cleanup_table(&mut self, ui: &mut egui::Ui, rows: &[CleanupRow]) {
        self.sync_cleanup_table_state(rows.len());
        let provider = CleanupTableProvider::new(rows);
        let state = &mut self.cleanup_view.table_state;
        let command_tx = self.command_tx.clone();
        let is_native = crate::IS_NATIVE;
        let output = TableKit::new(CLEANUP_TABLE_ID, &provider, state)
            .with_columns(cleanup_table_columns())
            // 28px rows: the trash/delete action cells render scaled-down
            // icon-operation buttons (OP_BUTTON_SCALE) with a comfortable
            // margin, matching the explorer's row height.
            .with_row_height(28.0)
            .with_max_height(Some(ui.available_height()))
            .with_striped(true)
            .show_with_output(ui, |ui, cell_info, row_data, text_color| {
                cleanup_custom_cell(
                    ui,
                    cell_info,
                    row_data,
                    text_color,
                    rows,
                    &command_tx,
                    is_native,
                )
            });
        // Row clicks carry no action in this table; ignore the event stream.
        let _ = output;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_rows() -> Vec<CleanupRow> {
        vec![
            CleanupRow {
                node_index: 0,
                tier: CleanupTier::ToolMediated,
                title: "systemd journal logs",
                hint: "Vacuums journal files down to 100 MB.",
                path: "/var/log/journal".to_string(),
                size_str: "4 MB".to_string(),
                age_str: "Recent".to_string(),
                sort_size: 4 * 1_048_576,
                sort_age_days: 0,
                command: Some("sudo journalctl --vacuum-size=100M"),
            },
            CleanupRow {
                node_index: 1,
                tier: CleanupTier::Regenerable,
                title: "Rust build artifacts",
                hint: "Rebuilt with `cargo build`.",
                path: "/home/user/project/target".to_string(),
                size_str: "50 MB".to_string(),
                age_str: "400 days".to_string(),
                sort_size: 50 * 1_048_576,
                sort_age_days: 400,
                command: None,
            },
        ]
    }

    /// Extracts a cell as owned strings for assertion (no unwrap/expect —
    /// the crate denies them).
    fn cell_strings(
        cell: Result<Option<TableCell<'_>>, TableError>,
    ) -> Option<(String, Option<String>)> {
        cell.ok()
            .flatten()
            .map(|(value, hover)| (value.into_owned(), hover.map(std::borrow::Cow::into_owned)))
    }

    #[test]
    fn provider_reports_flat_row_count() {
        let rows = sample_rows();
        let provider = CleanupTableProvider::new(&rows);
        assert_eq!(provider.row_count(), 2);
        assert_eq!(provider.column_count(), CLEANUP_COLUMN_COUNT);
        // Static-English title with the restore hint as hover.
        assert_eq!(
            cell_strings(provider.cell_at(1, 1)),
            Some((
                "Rust build artifacts".to_string(),
                Some("Rebuilt with `cargo build`.".to_string())
            ))
        );
    }

    #[test]
    fn sort_by_size_is_numeric() {
        let rows = sample_rows();
        let provider = CleanupTableProvider::new(&rows);
        let mut active = vec![0_usize, 1];
        assert!(provider.sort_active_rows(&mut active, 3, false).is_ok());
        // Descending: 50 MB row before the 4 MB row (string sort flips these).
        assert_eq!(active, vec![1, 0]);
    }

    #[test]
    fn sort_by_tier_puts_regenerable_first_ascending() {
        let rows = sample_rows();
        let provider = CleanupTableProvider::new(&rows);
        let mut active = vec![0_usize, 1];
        assert!(provider.sort_active_rows(&mut active, 0, true).is_ok());
        assert_eq!(active, vec![1, 0]);
    }

    #[test]
    fn age_label_buckets_fresh_and_unknown() {
        assert_eq!(age_label(None).1, 0);
        assert_eq!(age_label(Some(2)).1, 0);
        assert_eq!(age_label(Some(30)).1, 30);
    }

    #[test]
    fn action_column_is_placeholder_and_tool_rows_carry_command() {
        // Mirrors the core engine invariant the action column relies on.
        let rows = sample_rows();
        let provider = CleanupTableProvider::new(&rows);
        // The action column renders through the custom cell callback; the
        // provider returns an empty placeholder value there.
        assert_eq!(
            cell_strings(provider.cell_at(0, 5)),
            Some((String::new(), None))
        );
        assert!(rows[0].command.is_some());
        assert!(rows[1].command.is_none());
    }
}
