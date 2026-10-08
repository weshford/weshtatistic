//! Overlay filesystem mountpoint detection.
//!
//! Every running Docker container exposes an `overlay`-type mount (e.g.
//! `/var/lib/docker/overlay2/<id>/merged`) whose bytes are already counted
//! under the per-layer `diff/` directories. Full-filesystem scans that descend
//! into them double-count. Same-filesystem scans already avoid them (overlay
//! mounts report a synthetic `st_dev`); default scans need this path-based
//! guard.
//!
//! On Linux the mount table is read once from `/proc/self/mountinfo`; other
//! platforms return an empty set (overlay mounts only exist on Linux).

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Load the set of overlay mountpoint paths visible in this process.
///
/// Cheap: a single read of `/proc/self/mountinfo`. Returns an empty set on
/// non-Linux platforms.
#[must_use]
pub(crate) fn load_overlay_mounts() -> Arc<HashSet<PathBuf>> {
    #[cfg(target_os = "linux")]
    {
        let text = std::fs::read_to_string("/proc/self/mountinfo").unwrap_or_default();
        Arc::new(parse_mountinfo(&text))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Arc::new(HashSet::new())
    }
}

/// Parse `/proc/self/mountinfo` content, returning the mountpoints of
/// `overlay`-filesystem mounts.
///
/// Each line is: `id parent major:minor root mountpoint options… - fstype
/// source super-options`. The mountpoint is field 5 and the filesystem type
/// is the field following the `-` separator; both may contain octal escapes
/// such as `\040` for space.
#[cfg(target_os = "linux")]
fn parse_mountinfo(text: &str) -> HashSet<PathBuf> {
    let mut mounts = HashSet::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        // Fields 1-4: id, parent, major:minor, root.
        if fields.next().is_none()
            || fields.next().is_none()
            || fields.next().is_none()
            || fields.next().is_none()
        {
            continue;
        }
        // Field 5: mountpoint.
        let Some(mountpoint) = fields.next() else {
            continue;
        };
        // Optional fields (e.g. `shared:`, `master:`) run until the `-` separator.
        let mut fstype = None;
        for field in fields.by_ref() {
            if field == "-" {
                fstype = fields.next();
                break;
            }
        }
        if fstype == Some("overlay") {
            mounts.insert(decode_mountinfo_path(mountpoint));
        }
    }
    mounts
}

/// Decode the octal escapes (`\040`, `\134`, …) used for special characters
/// in `mountinfo` path fields. Malformed escapes are kept literally.
#[cfg(target_os = "linux")]
fn decode_mountinfo_path(field: &str) -> PathBuf {
    use std::os::unix::ffi::OsStringExt as _;

    let bytes = field.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\'
            && i + 3 < bytes.len()
            && bytes[i + 1].is_ascii_digit()
            && bytes[i + 2].is_ascii_digit()
            && bytes[i + 3].is_ascii_digit()
            && bytes[i + 1] < b'8'
            && bytes[i + 2] < b'8'
            && bytes[i + 3] < b'8'
        {
            let value = (u32::from(bytes[i + 1]) - u32::from(b'0')) * 64
                + (u32::from(bytes[i + 2]) - u32::from(b'0')) * 8
                + (u32::from(bytes[i + 3]) - u32::from(b'0'));
            decoded.push(value as u8);
            i += 4;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    PathBuf::from(std::ffi::OsString::from_vec(decoded))
}

/// True when `path` is an overlay mountpoint that traversal must not descend
/// into. Kept branch-cheap so the common (non-Docker) scan pays almost
/// nothing: callers short-circuit on `mounts.is_empty()` first. Always false
/// on non-Linux platforms, where the set is empty.
pub(crate) fn is_overlay_mountpoint(mounts: &HashSet<PathBuf>, path: &Path) -> bool {
    mounts.contains(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_mountinfo_extracts_overlay_mountpoints() {
        let fixture = "\
24 0 8:2 / / rw,relatime shared:1 - ext4 /dev/sda2 rw
36 24 0:31 / /var/lib/docker/overlay2/abc123/merged rw,relatime - overlay overlay rw,lowerdir=...,upperdir=...,workdir=...
37 24 0:32 / /var/lib/docker/overlay2/def\\040456/merged rw,relatime master:5 - overlay overlay rw
38 24 0:33 / /mnt/data\\040disk rw,noatime - ext4 /dev/sdb1 rw
39 24 0:34 / /proc proc rw,nosuid,nodev,noexec,relatime - proc proc rw
";
        let mounts = parse_mountinfo(fixture);
        assert_eq!(mounts.len(), 2);
        assert!(mounts.contains(Path::new("/var/lib/docker/overlay2/abc123/merged")));
        assert!(
            mounts.contains(Path::new("/var/lib/docker/overlay2/def 456/merged")),
            "octal-escaped overlay mountpoint must be decoded"
        );
        assert!(!mounts.contains(Path::new("/")));
        assert!(!mounts.contains(Path::new("/proc")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_decode_mountinfo_path_octal_escapes() {
        assert_eq!(
            decode_mountinfo_path("/mnt/my\\040disk/merged"),
            PathBuf::from("/mnt/my disk/merged")
        );
        assert_eq!(
            decode_mountinfo_path("/a\\134b\\011c\\012"),
            PathBuf::from("/a\\b\tc\n")
        );
        assert_eq!(
            decode_mountinfo_path("/plain/path"),
            PathBuf::from("/plain/path")
        );
        // Malformed escapes are preserved literally rather than dropped.
        assert_eq!(
            decode_mountinfo_path("/bad\\4x\\99esc"),
            PathBuf::from("/bad\\4x\\99esc")
        );
        assert_eq!(
            decode_mountinfo_path("/trunc\\04"),
            PathBuf::from("/trunc\\04")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_load_overlay_mounts_returns_absolute_paths() {
        // Environment-tolerant smoke test against the live mount table.
        let mounts = load_overlay_mounts();
        for path in mounts.iter() {
            assert!(
                path.is_absolute(),
                "mountinfo mountpoints must be absolute: {path:?}"
            );
        }
    }

    #[test]
    fn test_non_linux_empty_set() {
        #[cfg(not(target_os = "linux"))]
        {
            assert!(load_overlay_mounts().is_empty());
        }
    }
}
