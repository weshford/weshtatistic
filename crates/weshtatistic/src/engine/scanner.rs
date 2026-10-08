use std::{path::PathBuf, sync::Arc, sync::atomic::Ordering};

use weshtatistic_core::docker::DockerCollector;
use weshtatistic_core::state::SharedState;
use weshtatistic_gui::ScanController;

use super::{
    coordinator::Coordinator,
    docker::{NativeDockerCollector, attach_docker_inventory},
    traversal::TraversalEngine,
};

/// Native scanner backend: runs directory scans on background threads via the
/// traversal engine and coordinator, publishing snapshots and progress into
/// the shared state for the GUI to render.
pub struct EngineScanController {
    traversal_engine: Arc<TraversalEngine>,
    shared_state: Arc<SharedState>,
}

impl EngineScanController {
    pub const fn new(
        traversal_engine: Arc<TraversalEngine>,
        shared_state: Arc<SharedState>,
    ) -> Self {
        Self {
            traversal_engine,
            shared_state,
        }
    }
}

impl ScanController for EngineScanController {
    fn start_scan(&self, path: PathBuf, same_filesystem: bool) {
        // Start traversal and coordinator
        let (tx, rx) = crossbeam::channel::unbounded();

        // Launch Traversal Engine in background
        match self.traversal_engine.start_traversal(
            path.clone(),
            same_filesystem,
            self.shared_state.scan_cancel.clone(),
            tx,
        ) {
            Ok(_) => {
                // Launch Coordinator in background
                let mut coordinator = Coordinator::new(rx, self.shared_state.clone());
                let shared_state = self.shared_state.clone();
                std::thread::spawn(move || {
                    coordinator.run_coordinator_loop(&path.to_string_lossy());

                    // The scan is complete and its final snapshot published;
                    // collect the Docker inventory off the hot path and attach
                    // it as the EXT_DOCKER_INVENTORY extension.
                    if !weshtatistic_gui::gui::operations::is_macos_sandbox() {
                        attach_docker_after_scan(&shared_state);
                    }
                });
            }
            Err(e) => {
                println!("Failed to start traversal: {e}");
            }
        }
    }

    fn num_threads(&self) -> usize {
        self.traversal_engine.num_threads()
    }

    fn docker_collector(&self) -> Option<Arc<dyn weshtatistic_gui::docker::DockerCollector>> {
        // The App Sandbox blocks every Docker access path (the VM disk lives
        // in another app's container; unix sockets are off-limits), so live
        // collection is impossible in sandboxed builds — report no collector
        // and let the UI fall back to reviewing snapshot-carried data.
        if weshtatistic_gui::gui::operations::is_macos_sandbox() {
            return None;
        }
        Some(Arc::new(super::docker::NativeDockerCollector::new()))
    }
}

/// Post-scan Docker collection: runs on the coordinator's background thread
/// after `is_scanning` flips false, so scan completion is never blocked.
/// Silently skips when the scan was cancelled or no Docker presence was
/// detected; collection/attach failures are logged but never disturb the
/// scan results.
fn attach_docker_after_scan(shared_state: &Arc<SharedState>) {
    if shared_state.scan_cancel.load(Ordering::Relaxed) {
        return;
    }
    let collector = NativeDockerCollector::new();
    if !collector.detect_environment().has_docker() {
        return;
    }
    match collector.collect_preferred(&shared_state.scan_cancel) {
        Ok(inventory) => {
            if let Err(e) = attach_docker_inventory(shared_state, &inventory) {
                eprintln!("Failed to attach Docker inventory: {e}");
            }
        }
        Err(e) => eprintln!("Docker inventory collection failed: {e}"),
    }
}
