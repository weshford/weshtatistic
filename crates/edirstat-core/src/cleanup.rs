//! "Common cleanups" heuristics: a small rulepack evaluated over an
//! already-scanned tree that flags well-known reclaimable space.
//!
//! Two detection styles:
//!
//! - **Fingerprint rules** match a directory anywhere in the tree by its
//!   shape (e.g. a `target` directory whose parent holds a `Cargo.toml`), so
//!   they work on any disk layout, including external drives.
//! - **Path rules** match well-known absolute locations (e.g.
//!   `~/.cache/pip`), resolved against the scanned tree.
//!
//! Every finding is tiered by reversibility, not just size:
//!
//! - [`CleanupTier::Regenerable`] findings cost nothing to delete but future
//!   recompute or re-download; they may be acted on directly.
//! - [`CleanupTier::ToolMediated`] findings are owned by a system tool
//!   (package manager, journald, ...). They are strictly display-only: the
//!   rule carries the blessed command instead of a deletion path.
//!
//! Findings are ranked by `score = size * staleness * tier_weight`, where
//! staleness grows linearly with the directory's age (from its scanned
//! `modified_timestamp`) up to a cap — a cache untouched for a year is a
//! better reclaim candidate than a hot one.

use std::{collections::HashSet, path::Path, path::PathBuf, time::SystemTime};

use crate::{
    arena::{FileArenaSnapshot, FileNode, NO_INDEX},
    time_utils::system_time_to_unix_timestamp,
};

/// Findings smaller than this are noise in a "reclaim space" UI.
const MIN_FINDING_SIZE: u64 = 1_048_576; // 1 MiB

/// Staleness multiplier reaches its cap (+3.0) at this many days of age.
const STALENESS_CAP_DAYS: u32 = 270;

/// How reversible deleting a finding is. Drives both ranking and what action
/// the UI is allowed to offer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupTier {
    /// Rebuilt on demand by the owning tool (build artifacts, caches).
    Regenerable,
    /// Owned by a system tool; the user should run the rule's `command`
    /// instead of deleting files directly.
    ToolMediated,
}

impl CleanupTier {
    /// Regenerable space ranks slightly above tool-mediated space of equal
    /// size and age, because acting on it is one click instead of a terminal.
    #[must_use]
    const fn weight(self) -> f64 {
        match self {
            Self::Regenerable => 1.0,
            Self::ToolMediated => 0.8,
        }
    }
}

/// How a rule decides whether a directory is a cleanup target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuleMatcher {
    /// Well-known absolute path; a leading `~/` expands against
    /// [`CleanupContext::home_dir`].
    Path(&'static str),
    /// A directory with this base name whose *parent* contains `marker`
    /// (e.g. `target` next to `Cargo.toml`).
    DirWithParentMarker {
        dir: &'static str,
        marker: &'static str,
    },
    /// A directory that itself contains `marker` (e.g. `pyvenv.cfg`).
    DirWithChildMarker { marker: &'static str },
}

/// One entry in the cleanup rulepack. The corpus is static; rules reference
/// display strings directly so the data can later move to an external
/// rulepack format without changing the engine.
#[derive(Debug)]
pub struct CleanupRule {
    /// Stable kebab-case identifier.
    pub id: &'static str,
    pub tier: CleanupTier,
    matcher: RuleMatcher,
    /// Short human-readable name of the cleanup category.
    pub title: &'static str,
    /// What it costs to get the data back (displayed to the user).
    pub restore_hint: &'static str,
    /// The owning tool's cleanup command, for [`CleanupTier::ToolMediated`].
    pub command: Option<&'static str>,
}

macro_rules! tier1 {
    ($id:literal, $matcher:expr, $title:literal, $hint:literal) => {
        CleanupRule {
            id: $id,
            tier: CleanupTier::Regenerable,
            matcher: $matcher,
            title: $title,
            restore_hint: $hint,
            command: None,
        }
    };
}

macro_rules! tier2 {
    ($id:literal, $path:literal, $title:literal, $hint:literal, $command:literal) => {
        CleanupRule {
            id: $id,
            tier: CleanupTier::ToolMediated,
            matcher: RuleMatcher::Path($path),
            title: $title,
            restore_hint: $hint,
            command: Some($command),
        }
    };
}

static RULES: &[CleanupRule] = &[
    // ---- Regenerable: fingerprint rules (match anywhere in the tree) ----
    tier1!(
        "rust-target",
        RuleMatcher::DirWithParentMarker {
            dir: "target",
            marker: "Cargo.toml",
        },
        "Rust build artifacts",
        "Rebuilt with `cargo build`."
    ),
    tier1!(
        "node-modules",
        RuleMatcher::DirWithParentMarker {
            dir: "node_modules",
            marker: "package.json",
        },
        "Node.js dependencies",
        "Restored with `npm install` (or yarn/pnpm install)."
    ),
    tier1!(
        "python-venv",
        RuleMatcher::DirWithChildMarker {
            marker: "pyvenv.cfg",
        },
        "Python virtual environment",
        "Recreated from your project's requirements."
    ),
    // ---- Regenerable: path rules (well-known cache locations) ----
    tier1!(
        "cargo-registry",
        RuleMatcher::Path("~/.cargo/registry"),
        "Cargo package cache",
        "Crate sources are re-downloaded on demand."
    ),
    tier1!(
        "pip-cache",
        RuleMatcher::Path("~/.cache/pip"),
        "pip cache",
        "Wheels are re-downloaded on demand."
    ),
    tier1!(
        "uv-cache",
        RuleMatcher::Path("~/.cache/uv"),
        "uv cache",
        "Re-downloaded on demand, or clear with `uv cache clean`."
    ),
    tier1!(
        "go-build-cache",
        RuleMatcher::Path("~/.cache/go-build"),
        "Go build cache",
        "Rebuilt on the next `go build`."
    ),
    tier1!(
        "go-module-cache",
        RuleMatcher::Path("~/go/pkg/mod"),
        "Go module cache",
        "Restored with `go mod download`."
    ),
    tier1!(
        "npm-cache",
        RuleMatcher::Path("~/.npm/_cacache"),
        "npm cache",
        "Re-downloaded on demand, or clear with `npm cache verify`."
    ),
    tier1!(
        "yarn-cache",
        RuleMatcher::Path("~/.cache/yarn"),
        "Yarn cache",
        "Packages are re-downloaded on demand."
    ),
    tier1!(
        "pnpm-store",
        RuleMatcher::Path("~/.local/share/pnpm/store"),
        "pnpm store",
        "Pruned with `pnpm store prune`; packages re-download on demand."
    ),
    tier1!(
        "gradle-cache",
        RuleMatcher::Path("~/.gradle/caches"),
        "Gradle cache",
        "Re-downloaded on the next build."
    ),
    tier1!(
        "maven-cache",
        RuleMatcher::Path("~/.m2/repository"),
        "Maven repository cache",
        "Artifacts are re-downloaded on the next build."
    ),
    tier1!(
        "ccache",
        RuleMatcher::Path("~/.cache/ccache"),
        "ccache compiler cache",
        "Rebuilt on subsequent compiles."
    ),
    tier1!(
        "thumbnail-cache",
        RuleMatcher::Path("~/.cache/thumbnails"),
        "Thumbnail cache",
        "Regenerated by your file manager."
    ),
    tier1!(
        "xcode-derived-data",
        RuleMatcher::Path("~/Library/Developer/Xcode/DerivedData"),
        "Xcode DerivedData",
        "Regenerated on the next build."
    ),
    // ---- Tool-mediated: display-only, run the owning tool ----
    tier2!(
        "pacman-cache",
        "/var/cache/pacman/pkg",
        "pacman package cache",
        "Keeps only currently-installed package versions.",
        "sudo pacman -Sc"
    ),
    tier2!(
        "paru-cache",
        "~/.cache/paru",
        "paru AUR cache",
        "Cleans built AUR packages no longer installed.",
        "paru -Sc"
    ),
    tier2!(
        "yay-cache",
        "~/.cache/yay",
        "yay AUR cache",
        "Cleans built AUR packages no longer installed.",
        "yay -Sc"
    ),
    tier2!(
        "apt-cache",
        "/var/cache/apt/archives",
        "APT package cache",
        "Downloaded .deb files are refetched when needed.",
        "sudo apt clean"
    ),
    tier2!(
        "dnf-cache",
        "/var/cache/dnf",
        "DNF package cache",
        "Downloaded packages are refetched when needed.",
        "sudo dnf clean all"
    ),
    tier2!(
        "systemd-journal",
        "/var/log/journal",
        "systemd journal logs",
        "Vacuums journal files down to 100 MB.",
        "sudo journalctl --vacuum-size=100M"
    ),
];

/// The full cleanup rulepack.
#[must_use]
pub fn rules() -> &'static [CleanupRule] {
    RULES
}

/// Inputs to the analysis that would otherwise come from the environment,
/// injected so tests are deterministic.
#[derive(Debug, Clone)]
pub struct CleanupContext {
    /// Current time as seconds since the Unix epoch.
    pub now_unix: u32,
    /// The user's home directory, used to expand `~` in path rules. `None`
    /// disables all home-relative rules.
    pub home_dir: Option<PathBuf>,
}

impl CleanupContext {
    /// Context from the live environment (wall clock + `$HOME` /
    /// `%USERPROFILE%`).
    #[must_use]
    pub fn current() -> Self {
        Self {
            now_unix: system_time_to_unix_timestamp(SystemTime::now()),
            home_dir: detect_home_dir(),
        }
    }
}

#[cfg(target_os = "windows")]
fn detect_home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

#[cfg(not(target_os = "windows"))]
fn detect_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// A single matched cleanup target.
#[derive(Debug, Clone)]
pub struct CleanupFinding {
    /// Arena index of the flagged directory.
    pub node_index: u32,
    /// The rule that matched.
    pub rule: &'static CleanupRule,
    /// Directory size in bytes, from the scan.
    pub size: u64,
    /// `size * staleness * tier weight`; findings are sorted by it.
    pub score: f64,
    /// Days since the directory was last modified, if the scan recorded a
    /// timestamp. `None` is treated as "hot" for scoring.
    pub age_days: Option<u32>,
}

/// The result of running the rulepack over a snapshot.
#[derive(Debug, Default, Clone)]
pub struct CleanupReport {
    /// All findings, sorted by descending score.
    pub findings: Vec<CleanupFinding>,
    /// Sum of [`CleanupTier::Regenerable`] finding sizes.
    pub regenerable_bytes: u64,
    /// Sum of [`CleanupTier::ToolMediated`] finding sizes.
    pub tool_mediated_bytes: u64,
}

impl CleanupReport {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.findings.is_empty()
    }
}

/// Run the cleanup rulepack over a finished scan.
#[allow(clippy::cast_precision_loss)] // ranking score; sub-byte precision is irrelevant
#[must_use]
pub fn analyze(snapshot: &FileArenaSnapshot, ctx: &CleanupContext) -> CleanupReport {
    if snapshot.nodes.is_empty() {
        return CleanupReport::default();
    }

    let nodes: &[FileNode] = &snapshot.nodes;
    let pool = &snapshot.string_pool;
    let mut matches: Vec<(u32, usize)> = Vec::new();

    // Path rules: resolve each well-known location against the scanned tree.
    for (rule_idx, rule) in RULES.iter().enumerate() {
        if let RuleMatcher::Path(raw) = rule.matcher
            && let Some(path) = expand_rule_path(raw, ctx.home_dir.as_deref())
            && let Some(node_idx) = snapshot.resolve_path_index(&path)
            && nodes
                .get(node_idx as usize)
                .is_some_and(FileNode::is_directory)
        {
            matches.push((node_idx, rule_idx));
        }
    }

    // Fingerprint rules: one top-down sweep from the root. Walking the child
    // chains (rather than the flat arena) skips orphaned nodes left behind by
    // in-place rescans. Matched directories are not descended into: nested
    // copies (e.g. `node_modules` inside `node_modules`) belong to the outer
    // finding, and pruning keeps huge artifact trees cheap to skip.
    let parent_marker_rules: Vec<(usize, &'static str, &'static str)> = RULES
        .iter()
        .enumerate()
        .filter_map(|(idx, rule)| match rule.matcher {
            RuleMatcher::DirWithParentMarker { dir, marker } => Some((idx, dir, marker)),
            _ => None,
        })
        .collect();
    let child_marker_rules: Vec<(usize, &'static str)> = RULES
        .iter()
        .enumerate()
        .filter_map(|(idx, rule)| match rule.matcher {
            RuleMatcher::DirWithChildMarker { marker } => Some((idx, marker)),
            _ => None,
        })
        .collect();

    let mut stack: Vec<u32> = vec![0];
    while let Some(idx) = stack.pop() {
        let Some(node) = nodes.get(idx as usize) else {
            continue;
        };
        if !node.is_directory() || node.is_symlink() || node.is_special() {
            continue;
        }
        let Some(name) = pool.get(node.name_id) else {
            continue;
        };

        // One finding per directory: parent-marker rules win over child-marker
        // rules, first match wins.
        let mut matched_rule = parent_marker_rules
            .iter()
            .find(|(_, dir, _)| *dir == name)
            .and_then(|(rule_idx, _, marker)| {
                let parent = node.parent_opt()?;
                has_child_named(nodes, pool, parent, marker).then_some(*rule_idx)
            });

        if matched_rule.is_none() && !child_marker_rules.is_empty() {
            let mut child = node.first_child;
            while child != NO_INDEX {
                let Some(child_node) = nodes.get(child as usize) else {
                    break;
                };
                if let Some(child_name) = pool.get(child_node.name_id)
                    && let Some((rule_idx, _)) = child_marker_rules
                        .iter()
                        .find(|(_, marker)| *marker == child_name)
                {
                    matched_rule = Some(*rule_idx);
                    break;
                }
                child = child_node.next_sibling;
            }
        }

        if let Some(rule_idx) = matched_rule {
            matches.push((idx, rule_idx));
            continue; // prune: do not descend into a matched directory
        }

        let mut child = node.first_child;
        while child != NO_INDEX {
            let Some(child_node) = nodes.get(child as usize) else {
                break;
            };
            if child_node.is_directory() {
                stack.push(child);
            }
            child = child_node.next_sibling;
        }
    }

    // Drop findings nested inside another finding (the outer one already
    // accounts for those bytes).
    let matched_nodes: HashSet<u32> = matches.iter().map(|(idx, _)| *idx).collect();
    matches.retain(|(idx, _)| !has_matched_ancestor(nodes, *idx, &matched_nodes));

    let mut findings: Vec<CleanupFinding> = matches
        .into_iter()
        .filter_map(|(node_idx, rule_idx)| {
            let node = nodes.get(node_idx as usize)?;
            if node.size < MIN_FINDING_SIZE {
                return None;
            }
            let rule = &RULES[rule_idx];
            let age_days = age_days(node.modified_timestamp, ctx.now_unix);
            let score = node.size as f64 * staleness_factor(age_days) * rule.tier.weight();
            Some(CleanupFinding {
                node_index: node_idx,
                rule,
                size: node.size,
                score,
                age_days,
            })
        })
        .collect();
    findings.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.rule.id.cmp(b.rule.id))
    });

    let mut report = CleanupReport::default();
    for finding in &findings {
        match finding.rule.tier {
            CleanupTier::Regenerable => report.regenerable_bytes += finding.size,
            CleanupTier::ToolMediated => report.tool_mediated_bytes += finding.size,
        }
    }
    report.findings = findings;
    report
}

/// True if any strict ancestor of `idx` is itself a matched finding.
fn has_matched_ancestor(nodes: &[FileNode], idx: u32, matched: &HashSet<u32>) -> bool {
    let mut curr = nodes.get(idx as usize).and_then(FileNode::parent_opt);
    while let Some(ancestor) = curr {
        if matched.contains(&ancestor) {
            return true;
        }
        curr = nodes.get(ancestor as usize).and_then(FileNode::parent_opt);
    }
    false
}

/// True if `parent` has an immediate child named `name`.
fn has_child_named(
    nodes: &[FileNode],
    pool: &crate::arena::StringPool,
    parent: u32,
    name: &str,
) -> bool {
    let Some(parent_node) = nodes.get(parent as usize) else {
        return false;
    };
    let mut child = parent_node.first_child;
    while child != NO_INDEX {
        let Some(child_node) = nodes.get(child as usize) else {
            break;
        };
        if pool.get(child_node.name_id).is_some_and(|n| n == name) {
            return true;
        }
        child = child_node.next_sibling;
    }
    false
}

/// Expand a path rule against the home directory. The result uses `/`
/// separators; `resolve_path_index` accepts both separators on all platforms.
fn expand_rule_path(raw: &str, home: Option<&Path>) -> Option<String> {
    if let Some(rest) = raw.strip_prefix("~/") {
        let home = home?.to_string_lossy();
        let home = home.trim_end_matches(['/', '\\']);
        Some(format!("{home}/{rest}"))
    } else {
        Some(raw.to_string())
    }
}

const fn age_days(modified_timestamp: u32, now_unix: u32) -> Option<u32> {
    if modified_timestamp == 0 || modified_timestamp >= now_unix {
        return None;
    }
    Some((now_unix - modified_timestamp) / 86_400)
}

/// Linear growth from 1.0 (fresh) to 4.0 at [`STALENESS_CAP_DAYS`] of age.
fn staleness_factor(age_days: Option<u32>) -> f64 {
    age_days.map_or(1.0, |days| {
        1.0 + f64::from(days.min(STALENESS_CAP_DAYS)) / 90.0
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::{NodeStorage, StringPool, precompute_dir_counts};
    use std::sync::Arc;

    const NOW: u32 = 1_800_000_000;
    const MIB: u64 = 1_048_576;

    /// Minimal linked-tree builder; sizes are cumulative, so tests set
    /// directory sizes explicitly via `set_size`.
    struct TreeBuilder {
        pool: StringPool,
        nodes: Vec<FileNode>,
    }

    impl TreeBuilder {
        fn new(root: &str) -> Self {
            let mut pool = StringPool::new();
            let root_id = pool.get_or_insert(root.as_bytes());
            Self {
                pool,
                nodes: vec![FileNode::new(root_id, None, true, false, NOW, NOW)],
            }
        }

        fn link(&mut self, parent: u32, child: u32) {
            if self.nodes[parent as usize].first_child == NO_INDEX {
                self.nodes[parent as usize].first_child = child;
                return;
            }
            let mut curr = self.nodes[parent as usize].first_child;
            while self.nodes[curr as usize].next_sibling != NO_INDEX {
                curr = self.nodes[curr as usize].next_sibling;
            }
            self.nodes[curr as usize].next_sibling = child;
        }

        fn dir(&mut self, parent: u32, name: &str, mtime: u32) -> u32 {
            let name_id = self.pool.get_or_insert(name.as_bytes());
            let idx = self.nodes.len() as u32;
            self.nodes.push(FileNode::new(
                name_id,
                Some(parent),
                true,
                false,
                mtime,
                mtime,
            ));
            self.link(parent, idx);
            idx
        }

        fn file(&mut self, parent: u32, name: &str, size: u64, mtime: u32) -> u32 {
            let name_id = self.pool.get_or_insert(name.as_bytes());
            let idx = self.nodes.len() as u32;
            let mut node = FileNode::new(name_id, Some(parent), false, false, mtime, mtime);
            node.size = size;
            self.nodes.push(node);
            self.link(parent, idx);
            idx
        }

        fn set_size(&mut self, idx: u32, size: u64) {
            self.nodes[idx as usize].size = size;
        }

        fn build(self) -> FileArenaSnapshot {
            FileArenaSnapshot {
                dir_counts: Arc::new(precompute_dir_counts(&self.nodes)),
                nodes: Arc::new(NodeStorage::Owned(self.nodes)),
                string_pool: Arc::new(self.pool),
                extensions: crate::extensions::ExtensionStore::default(),
            }
        }
    }

    fn ctx() -> CleanupContext {
        CleanupContext {
            now_unix: NOW,
            home_dir: Some(PathBuf::from("/home/user")),
        }
    }

    fn days_ago(days: u32) -> u32 {
        NOW - days * 86_400
    }

    fn rule_index(id: &str) -> usize {
        let Some(idx) = RULES.iter().position(|r| r.id == id) else {
            unreachable!("rule {id} must exist");
        };
        idx
    }

    #[test]
    fn target_with_cargo_toml_matches() {
        let mut tree = TreeBuilder::new("/home/user");
        let project = tree.dir(0, "project", days_ago(10));
        tree.file(project, "Cargo.toml", 500, days_ago(10));
        let target = tree.dir(project, "target", days_ago(400));
        tree.file(target, "binary", 50 * MIB, days_ago(400));
        tree.set_size(target, 50 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule.id, "rust-target");
        assert_eq!(report.findings[0].size, 50 * MIB);
        assert_eq!(report.regenerable_bytes, 50 * MIB);
        assert_eq!(report.tool_mediated_bytes, 0);
    }

    #[test]
    fn target_without_marker_does_not_match() {
        let mut tree = TreeBuilder::new("/home/user");
        let misc = tree.dir(0, "misc", days_ago(10));
        let target = tree.dir(misc, "target", days_ago(10));
        tree.set_size(target, 50 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert!(report.is_empty());
    }

    #[test]
    fn nested_node_modules_reported_once() {
        let mut tree = TreeBuilder::new("/home/user");
        let project = tree.dir(0, "app", days_ago(10));
        tree.file(project, "package.json", 300, days_ago(10));
        let outer = tree.dir(project, "node_modules", days_ago(200));
        let pkg = tree.dir(outer, "leftpad", days_ago(200));
        tree.file(pkg, "package.json", 200, days_ago(200));
        let inner = tree.dir(pkg, "node_modules", days_ago(200));
        tree.set_size(inner, 10 * MIB);
        tree.set_size(outer, 100 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].node_index, outer);
        assert_eq!(report.findings[0].size, 100 * MIB);
    }

    #[test]
    fn venv_matches_via_pyvenv_cfg() {
        let mut tree = TreeBuilder::new("/home/user");
        let project = tree.dir(0, "tool", days_ago(30));
        let venv = tree.dir(project, ".venv", days_ago(90));
        tree.file(venv, "pyvenv.cfg", 100, days_ago(90));
        tree.set_size(venv, 20 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule.id, "python-venv");
    }

    #[test]
    fn path_rule_resolves_against_home() {
        let mut tree = TreeBuilder::new("/home/user");
        let cache = tree.dir(0, ".cache", days_ago(5));
        let pip = tree.dir(cache, "pip", days_ago(60));
        tree.set_size(pip, 8 * MIB);
        tree.set_size(cache, 8 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert_eq!(report.findings.len(), 1);
        assert_eq!(report.findings[0].rule.id, "pip-cache");
    }

    #[test]
    fn home_relative_rules_skip_without_home() {
        let mut tree = TreeBuilder::new("/home/user");
        let cache = tree.dir(0, ".cache", days_ago(5));
        let pip = tree.dir(cache, "pip", days_ago(60));
        tree.set_size(pip, 8 * MIB);

        let no_home = CleanupContext {
            now_unix: NOW,
            home_dir: None,
        };
        let report = analyze(&tree.build(), &no_home);
        assert!(report.is_empty());
    }

    #[test]
    fn tiny_findings_are_filtered() {
        let mut tree = TreeBuilder::new("/home/user");
        let project = tree.dir(0, "project", days_ago(10));
        tree.file(project, "Cargo.toml", 500, days_ago(10));
        let target = tree.dir(project, "target", days_ago(10));
        tree.file(target, "empty", 100, days_ago(10));
        tree.set_size(target, 100);

        let report = analyze(&tree.build(), &ctx());
        assert!(report.is_empty());
    }

    #[test]
    fn stale_equal_sized_findings_rank_higher() {
        let mut tree = TreeBuilder::new("/home/user");
        let fresh_project = tree.dir(0, "fresh", days_ago(2));
        tree.file(fresh_project, "Cargo.toml", 500, days_ago(2));
        let fresh_target = tree.dir(fresh_project, "target", days_ago(2));
        tree.set_size(fresh_target, 50 * MIB);

        let stale_project = tree.dir(0, "stale", days_ago(400));
        tree.file(stale_project, "Cargo.toml", 500, days_ago(400));
        let stale_target = tree.dir(stale_project, "target", days_ago(400));
        tree.set_size(stale_target, 50 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert_eq!(report.findings.len(), 2);
        assert_eq!(report.findings[0].node_index, stale_target);
        assert_eq!(report.findings[1].node_index, fresh_target);
        assert!(report.findings[0].score > report.findings[1].score);
    }

    #[test]
    fn tool_mediated_rules_have_no_delete_path() {
        let mut tree = TreeBuilder::new("/");
        let var = tree.dir(0, "var", days_ago(1));
        let cache = tree.dir(var, "cache", days_ago(1));
        let pacman = tree.dir(cache, "pacman", days_ago(1));
        let pkg = tree.dir(pacman, "pkg", days_ago(30));
        tree.set_size(pkg, 4 * MIB);

        let report = analyze(&tree.build(), &ctx());
        assert_eq!(report.findings.len(), 1);
        let finding = &report.findings[0];
        assert_eq!(finding.rule.id, "pacman-cache");
        assert_eq!(finding.rule.tier, CleanupTier::ToolMediated);
        assert_eq!(finding.rule.command, Some("sudo pacman -Sc"));
        assert_eq!(report.tool_mediated_bytes, 4 * MIB);
        assert_eq!(report.regenerable_bytes, 0);
    }

    #[test]
    fn orphaned_nodes_are_not_reported() {
        let mut tree = TreeBuilder::new("/home/user");
        // Orphan: parented at the root but never linked into its child chain,
        // mimicking what an in-place rescan leaves behind.
        let name_id = tree.pool.get_or_insert(b"target");
        let mut orphan = FileNode::new(name_id, Some(0), true, false, days_ago(400), days_ago(400));
        orphan.size = 50 * MIB;
        tree.nodes.push(orphan);

        let report = analyze(&tree.build(), &ctx());
        assert!(report.is_empty());
    }

    #[test]
    fn every_rule_has_display_data_and_tier_consistency() {
        for rule in rules() {
            assert!(!rule.id.is_empty());
            assert!(!rule.title.is_empty());
            assert!(!rule.restore_hint.is_empty());
            match rule.tier {
                CleanupTier::Regenerable => assert!(rule.command.is_none()),
                CleanupTier::ToolMediated => {
                    assert!(rule.command.is_some_and(|c| !c.is_empty()));
                    assert!(matches!(rule.matcher, RuleMatcher::Path(_)));
                }
            }
        }
        assert_eq!(rule_index("rust-target"), 0);
    }
}
