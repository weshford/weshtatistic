//! Docker daemon API client: inventory reads (`GET /system/df`) and image
//! removal (`DELETE /images/{id}`), both over the local socket.
//!
//! The disk parser in [`super::docker`] needs no daemon, but cannot attribute
//! containerd-store images, and walking `layerdb` is slow on huge roots. When
//! the daemon socket is reachable we prefer its `system/df` summary and enrich
//! it from disk with the two things the API does not report: per-container
//! json log sizes and volume sizes the daemon left unreported. Image deletion
//! has no disk equivalent at all: the daemon is the only safe authority for
//! removing image state, so that endpoint lives here too.
//!
//! # HTTP over `UnixStream`
//!
//! Minimal HTTP/1.1 client: one pipelined request with `Connection: close`,
//! then read the response to EOF. The daemon answers either with a
//! `Content-Length`-delimited body or `Transfer-Encoding: chunked`; both are
//! handled (simple chunked framing: hex size line, data, CRLF, terminated by
//! a zero-size chunk whose trailers we ignore). Read/write timeouts (~5s)
//! keep a wedged daemon from stalling a scan; note the *connect* itself needs
//! no timeout because a Unix connect completes once the socket file exists —
//! daemon liveness is covered by the first read timing out.
//!
//! Failure semantics: every failure mode is a typed [`DockerApiError`]; the
//! caller (`NativeDockerCollector::collect_preferred`) falls back to the disk
//! parse. Nothing here panics.

use std::{
    ffi::OsString,
    fmt,
    io::{ErrorKind, Read, Write},
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use weshtatistic_core::WeshtatisticError;
use weshtatistic_core::docker::{
    ContainerInfo, DockerInventory, ImageDeletion, ImageInfo, InventorySource, InventoryTotals,
    VolumeInfo,
};
use serde_json::Value;

/// `GET /system/df` on API v1.44 (bundled with Docker 25.0; daemons negotiate
/// down to their own version for this read-only endpoint).
const SYSTEM_DF_REQUEST: &str =
    "GET /v1.44/system/df HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n";

/// Idle timeout for socket reads/writes. `system/df` is small and local;
/// anything slower means the daemon is wedged.
const SOCKET_TIMEOUT: Duration = Duration::from_secs(5);

/// Sanity cap on the raw response so a malicious or broken peer cannot make us
/// buffer unbounded memory. Real `system/df` payloads are well under 1 MiB
/// even with thousands of images.
const MAX_RESPONSE_BYTES: usize = 64 * 1024 * 1024;

/// Why a daemon-API exchange failed; the enum keeps the reason precise for
/// logs/tests. Collection failures let the caller fall back to the disk
/// parse on any variant; deletion failures convert to [`WeshtatisticError`] with
/// the daemon's message preserved verbatim.
#[derive(Debug)]
pub(crate) enum DockerApiError {
    /// `cancel` was set before or during the exchange.
    Cancelled,
    /// The Unix socket could not be connected (missing path, permissions).
    Connect(std::io::Error),
    /// Read or write timed out (daemon wedged).
    Timeout,
    /// I/O failure while sending the request or reading the response.
    Request(std::io::Error),
    /// Bytes came back, but not a well-formed HTTP response.
    InvalidResponse(&'static str),
    /// HTTP status other than 200.
    Status(u16),
    /// 404/409-style rejection whose body carried a daemon error message
    /// (e.g. "image is being used by stopped container abc"), surfaced
    /// verbatim so the UI can show the daemon's own explanation.
    Daemon { status: u16, message: String },
    /// 200 response whose body is not the expected JSON document.
    Decode(serde_json::Error),
}

impl fmt::Display for DockerApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => write!(f, "Docker API collection cancelled"),
            Self::Connect(e) => write!(f, "cannot connect to Docker socket: {e}"),
            Self::Timeout => write!(f, "timed out talking to the Docker daemon"),
            Self::Request(e) => write!(f, "I/O error talking to the Docker daemon: {e}"),
            Self::InvalidResponse(detail) => {
                write!(
                    f,
                    "malformed HTTP response from the Docker daemon: {detail}"
                )
            }
            Self::Status(code) => write!(f, "Docker daemon returned HTTP status {code}"),
            Self::Daemon { status, message } => {
                write!(f, "Docker daemon returned HTTP status {status}: {message}")
            }
            Self::Decode(e) => write!(f, "invalid JSON from the Docker daemon: {e}"),
        }
    }
}

impl std::error::Error for DockerApiError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Connect(e) | Self::Request(e) => Some(e),
            Self::Decode(e) => Some(e),
            Self::Cancelled
            | Self::Timeout
            | Self::InvalidResponse(_)
            | Self::Status(_)
            | Self::Daemon { .. } => None,
        }
    }
}

/// Convert an API failure into the shared error type. I/O errors keep their
/// original `ErrorKind`; daemon rejections (404/409) map to `NotFound` /
/// `ResourceBusy` so callers can react on kind, with the daemon's message
/// preserved verbatim for display.
impl From<DockerApiError> for WeshtatisticError {
    fn from(error: DockerApiError) -> Self {
        match error {
            DockerApiError::Connect(e) | DockerApiError::Request(e) => Self::Io(e),
            DockerApiError::Decode(e) => Self::Io(std::io::Error::new(ErrorKind::InvalidData, e)),
            DockerApiError::Cancelled => Self::Io(std::io::Error::from(ErrorKind::Interrupted)),
            DockerApiError::Timeout => Self::Io(std::io::Error::from(ErrorKind::TimedOut)),
            DockerApiError::InvalidResponse(detail) => Self::Io(std::io::Error::new(
                ErrorKind::InvalidData,
                format!("malformed HTTP response from the Docker daemon: {detail}"),
            )),
            DockerApiError::Status(code) => Self::Io(std::io::Error::other(format!(
                "Docker daemon returned HTTP status {code}"
            ))),
            DockerApiError::Daemon { status, message } => {
                let kind = match status {
                    404 => ErrorKind::NotFound,
                    409 => ErrorKind::ResourceBusy,
                    _ => ErrorKind::Other,
                };
                Self::Io(std::io::Error::new(
                    kind,
                    format!("Docker daemon returned HTTP status {status}: {message}"),
                ))
            }
        }
    }
}

/// Discover the daemon socket: `$DOCKER_HOST` (when it is a `unix://` path),
/// then `/var/run/docker.sock`, then the rootless `$XDG_RUNTIME_DIR` socket.
/// First existing path wins; `None` when no candidate exists. Environment
/// variables are read through `var` (normally [`std::env::var_os`]).
#[must_use]
pub(crate) fn discover_socket(var: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if let Some(host) = var("DOCKER_HOST") {
        let host = host.to_string_lossy();
        // An explicit `unix://` override wins when the path exists.
        if let Some(path) = host
            .strip_prefix("unix://")
            .filter(|path| !path.is_empty() && Path::new(path).exists())
        {
            return Some(PathBuf::from(path));
        }
    }
    let system = PathBuf::from("/var/run/docker.sock");
    if system.exists() {
        return Some(system);
    }
    if let Some(runtime_dir) = var("XDG_RUNTIME_DIR") {
        let candidate = PathBuf::from(runtime_dir).join("docker.sock");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    // Docker Desktop (macOS) and some rootless setups live under the home dir.
    if let Some(home) = var("HOME") {
        let candidate = PathBuf::from(home).join(".docker/run/docker.sock");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Collect `GET /system/df` from the daemon at `socket_path` and map it to a
/// [`DockerInventory`] (`source = DockerApi`, `collected_at = now`, totals
/// computed). Disk enrichment and the `root` field are the caller's job.
///
/// # Errors
/// Returns [`DockerApiError`] on connect failure, timeout, non-200 status,
/// malformed HTTP framing, or a non-JSON body. Never panics.
pub(crate) fn collect_system_df(
    socket_path: &Path,
    cancel: &AtomicBool,
) -> Result<DockerInventory, DockerApiError> {
    let response = perform_request(socket_path, SYSTEM_DF_REQUEST.as_bytes(), cancel)?;
    if response.status != 200 {
        return Err(DockerApiError::Status(response.status));
    }
    let df: Value = serde_json::from_slice(&response.body).map_err(DockerApiError::Decode)?;
    let mut inventory = inventory_from_df(&df);
    fill_layer_counts(&mut inventory, socket_path, cancel);
    Ok(inventory)
}

/// Fill `ImageInfo.layer_count` on an API-built inventory. `/system/df`
/// omits per-image layer lists (the UI's Layers column reads 0 otherwise),
/// so each image gets a cheap sequential `GET /images/{id}/json` over the
/// local socket. A failed inspect leaves the count at 0 rather than failing
/// the collection; `cancel` stops the loop between requests.
pub(crate) fn fill_layer_counts(
    inventory: &mut DockerInventory,
    socket_path: &Path,
    cancel: &AtomicBool,
) {
    for image in &mut inventory.images {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        if let Ok(count) = image_layer_count(socket_path, &image.id, cancel) {
            image.layer_count = count;
        }
    }
}

/// `GET /v1.44/images/{id}/json` → the `RootFS.Layers` array length.
///
/// # Errors
/// Same transport errors as the other endpoints; a non-200 status is a
/// `Status` error (callers treat it as "count unknown"). Never panics.
fn image_layer_count(
    socket_path: &Path,
    image_id: &str,
    cancel: &AtomicBool,
) -> Result<u32, DockerApiError> {
    let request = format!(
        "GET /v1.44/images/{image_id}/json HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n"
    );
    let response = perform_request(socket_path, request.as_bytes(), cancel)?;
    if response.status != 200 {
        return Err(DockerApiError::Status(response.status));
    }
    let json: Value = serde_json::from_slice(&response.body).map_err(DockerApiError::Decode)?;
    Ok(json
        .pointer("/RootFS/Layers")
        .and_then(Value::as_array)
        .map_or(0, |layers| u32::try_from(layers.len()).unwrap_or(u32::MAX)))
}

/// Delete an image by ID (`DELETE /v1.44/images/{id}`) on the daemon at
/// `socket_path` and map the deletion report into an [`ImageDeletion`].
///
/// The request always carries `noprune=false` so deleting one image does not
/// sweep unrelated dangling parents. On success (HTTP 200) the body is an
/// array of `{"Untagged": "repo:tag"}` / `{"Deleted": "sha256:…"}` entries;
/// each key's values are collected in order. Docker's failure bodies are
/// `{"message": "…"}`: 404 (no such image) and 409 (in use by a container)
/// surface that message verbatim via [`DockerApiError::Daemon`]; any other
/// non-200 is a plain [`DockerApiError::Status`].
///
/// # Errors
/// Same failure modes as [`collect_system_df`]. Never panics.
pub(crate) fn delete_image(
    socket_path: &Path,
    image_id: &str,
    force: bool,
    cancel: &AtomicBool,
) -> Result<ImageDeletion, DockerApiError> {
    let request = format!(
        "DELETE /v1.44/images/{image_id}?force={force}&noprune=false HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n"
    );
    let response = perform_request(socket_path, request.as_bytes(), cancel)?;
    match response.status {
        200 => deletion_from_body(&response.body),
        404 | 409 => match daemon_message(&response.body) {
            Some(message) => Err(DockerApiError::Daemon {
                status: response.status,
                message,
            }),
            None => Err(DockerApiError::Status(response.status)),
        },
        other => Err(DockerApiError::Status(other)),
    }
}

/// Map a 200 `DELETE /images/{id}` body — an array of `Untagged`/`Deleted`
/// single-key objects — into an [`ImageDeletion`]; unknown keys are ignored,
/// non-object entries are a malformed response.
fn deletion_from_body(body: &[u8]) -> Result<ImageDeletion, DockerApiError> {
    let entries: Vec<Value> = serde_json::from_slice(body).map_err(DockerApiError::Decode)?;
    let mut deletion = ImageDeletion::default();
    for entry in entries {
        let Some(object) = entry.as_object() else {
            return Err(DockerApiError::InvalidResponse(
                "DELETE response entry is not a JSON object",
            ));
        };
        if let Some(tag) = object.get("Untagged").and_then(Value::as_str) {
            deletion.untagged.push(tag.to_owned());
        }
        if let Some(id) = object.get("Deleted").and_then(Value::as_str) {
            deletion.deleted.push(id.to_owned());
        }
    }
    Ok(deletion)
}

/// Extract the daemon's `{"message": "…"}` error text; `None` when the body
/// is not that shape (callers degrade to a bare status error).
fn daemon_message(body: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(body)
        .ok()?
        .get("message")?
        .as_str()
        .map(str::to_owned)
}

/// Delete a container by ID (`DELETE /v1.44/containers/{id}`) on the daemon
/// at `socket_path`. Success is an empty `204 No Content` (a bare 200 is
/// tolerated); 404 (no such container) and 409 (running without `force`)
/// surface the daemon's `message` verbatim via [`DockerApiError::Daemon`].
///
/// # Errors
/// Same failure modes as [`delete_image`]. Never panics.
pub(crate) fn delete_container(
    socket_path: &Path,
    container_id: &str,
    force: bool,
    cancel: &AtomicBool,
) -> Result<(), DockerApiError> {
    delete_resource(
        socket_path,
        &format!("/v1.44/containers/{container_id}?force={force}&v=false"),
        cancel,
    )
}

/// Delete a named volume (`DELETE /v1.44/volumes/{name}`) on the daemon at
/// `socket_path`. Success is an empty `204 No Content` (a bare 200 is
/// tolerated); 404 (no such volume) and 409 (in use without `force`) surface
/// the daemon's `message` verbatim via [`DockerApiError::Daemon`]. Volume
/// names come from the daemon's own listing and are passed through
/// unchanged (the API accepts `[a-zA-Z0-9_.-]` unencoded).
///
/// # Errors
/// Same failure modes as [`delete_image`]. Never panics.
pub(crate) fn delete_volume(
    socket_path: &Path,
    name: &str,
    force: bool,
    cancel: &AtomicBool,
) -> Result<(), DockerApiError> {
    delete_resource(
        socket_path,
        &format!("/v1.44/volumes/{name}?force={force}"),
        cancel,
    )
}

/// Shared `DELETE` plumbing for endpoints whose success is an empty body
/// (`204 No Content`, tolerating a bare 200): containers and volumes. Error
/// mapping is identical to [`delete_image`], without the success-body decode.
fn delete_resource(
    socket_path: &Path,
    path_and_query: &str,
    cancel: &AtomicBool,
) -> Result<(), DockerApiError> {
    let request =
        format!("DELETE {path_and_query} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n");
    let response = perform_request(socket_path, request.as_bytes(), cancel)?;
    match response.status {
        200 | 204 => Ok(()),
        404 | 409 => match daemon_message(&response.body) {
            Some(message) => Err(DockerApiError::Daemon {
                status: response.status,
                message,
            }),
            None => Err(DockerApiError::Status(response.status)),
        },
        other => Err(DockerApiError::Status(other)),
    }
}

/// Open the socket at `socket_path`, send one pipelined HTTP/1.1 request
/// (`Connection: close`), and read the response to EOF. Shared transport for
/// [`collect_system_df`] and [`delete_image`]; interpreting the status code
/// is the caller's job. The connect itself needs no timeout (a Unix connect
/// completes once the socket file exists); socket reads/writes are bounded by
/// [`SOCKET_TIMEOUT`], and a set `cancel` flag aborts the exchange.
///
/// # Errors
/// Returns [`DockerApiError`] on connect failure, timeout, I/O failure, or
/// malformed HTTP framing. Never panics.
fn perform_request(
    socket_path: &Path,
    request: &[u8],
    cancel: &AtomicBool,
) -> Result<HttpResponse, DockerApiError> {
    if cancel.load(Ordering::Relaxed) {
        return Err(DockerApiError::Cancelled);
    }

    let mut stream = UnixStream::connect(socket_path).map_err(DockerApiError::Connect)?;
    stream
        .set_read_timeout(Some(SOCKET_TIMEOUT))
        .map_err(DockerApiError::Connect)?;
    stream
        .set_write_timeout(Some(SOCKET_TIMEOUT))
        .map_err(DockerApiError::Connect)?;
    stream.write_all(request).map_err(DockerApiError::Request)?;

    // `Connection: close` means the daemon closes the stream after the body;
    // read until EOF. The timeout converts a silent hang into `Timeout`.
    let mut raw = Vec::new();
    let mut buf = [0u8; 16 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(DockerApiError::Cancelled);
        }
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                if raw.len() > MAX_RESPONSE_BYTES {
                    return Err(DockerApiError::InvalidResponse(
                        "response exceeds the size cap",
                    ));
                }
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                return Err(DockerApiError::Timeout);
            }
            Err(e) => return Err(DockerApiError::Request(e)),
        }
    }
    drop(stream);

    parse_http_response(&raw)
}

/// Enrich an API-built inventory from an accessible data root: per-container
/// json log sizes (the API never reports logs) and volume sizes the daemon
/// left at zero/missing. Totals are recomputed afterwards.
pub(crate) fn enrich_from_disk(inventory: &mut DockerInventory, root: &Path, cancel: &AtomicBool) {
    for container in &mut inventory.containers {
        if cancel.load(Ordering::Relaxed) {
            return;
        }
        container.log_bytes = super::docker::container_log_bytes(root, &container.id);
    }
    for volume in &mut inventory.volumes {
        if volume.size_bytes == 0
            && let Some(size) = super::docker::volume_size_from_disk(root, &volume.name, cancel)
        {
            volume.size_bytes = size;
        }
    }
    inventory.compute_totals();
}

/// A parsed HTTP response: status code plus the (dechunked/length-trimmed) body.
#[derive(Debug)]
struct HttpResponse {
    status: u16,
    body: Vec<u8>,
}

/// Split a raw HTTP/1.x response into status + body, handling
/// `Content-Length`, read-until-EOF, and simple chunked framing.
fn parse_http_response(raw: &[u8]) -> Result<HttpResponse, DockerApiError> {
    let header_end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or(DockerApiError::InvalidResponse("missing header terminator"))?;
    let head = &raw[..header_end];
    let body = &raw[header_end + 4..];

    let mut lines = head.split(|&b| b == b'\n');
    let status_line = lines
        .next()
        .ok_or(DockerApiError::InvalidResponse("empty response"))?;
    // `HTTP/1.1 200 OK` — the code is the second whitespace-separated token.
    let mut tokens = status_line.split(|&b| b == b' ');
    if !tokens.next().is_some_and(|t| t.starts_with(b"HTTP/")) {
        return Err(DockerApiError::InvalidResponse("missing HTTP status line"));
    }
    let status = tokens
        .next()
        .and_then(|t| std::str::from_utf8(t).ok())
        .and_then(|t| t.trim().parse::<u16>().ok())
        .ok_or(DockerApiError::InvalidResponse("unparsable status code"))?;

    let mut content_length: Option<usize> = None;
    let mut chunked = false;
    for line in lines {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(colon) = line.iter().position(|&b| b == b':') else {
            continue;
        };
        let name = &line[..colon];
        let value = &line[colon + 1..];
        if name.eq_ignore_ascii_case(b"content-length") {
            content_length = std::str::from_utf8(value)
                .ok()
                .and_then(|v| v.trim().parse::<usize>().ok());
        } else if name.eq_ignore_ascii_case(b"transfer-encoding")
            && String::from_utf8_lossy(value)
                .to_ascii_lowercase()
                .contains("chunked")
        {
            chunked = true;
        }
    }

    let body = if chunked {
        decode_chunked(body)?
    } else if let Some(len) = content_length {
        if body.len() < len {
            return Err(DockerApiError::InvalidResponse(
                "body shorter than Content-Length",
            ));
        }
        body[..len].to_vec()
    } else {
        // No framing header: `Connection: close` delimits the body.
        body.to_vec()
    };
    Ok(HttpResponse { status, body })
}

/// Decode simple HTTP chunked framing: hex size lines (ignoring chunk
/// extensions after `;`), CRLF after each chunk, terminated by a zero-size
/// chunk. Trailer headers after the zero chunk are ignored.
fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, DockerApiError> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        let line_end = find_crlf(body, pos)
            .ok_or(DockerApiError::InvalidResponse("truncated chunk size line"))?;
        let size_text = std::str::from_utf8(&body[pos..line_end])
            .map_err(|_| DockerApiError::InvalidResponse("chunk size is not ASCII"))?
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        let size = usize::from_str_radix(size_text, 16)
            .map_err(|_| DockerApiError::InvalidResponse("invalid chunk size"))?;
        pos = line_end + 2;
        if size == 0 {
            break;
        }
        let end = pos
            .checked_add(size)
            .filter(|&end| end <= body.len())
            .ok_or(DockerApiError::InvalidResponse("truncated chunk data"))?;
        out.extend_from_slice(&body[pos..end]);
        if body.get(end..end + 2) != Some(b"\r\n") {
            return Err(DockerApiError::InvalidResponse("missing chunk terminator"));
        }
        pos = end + 2;
    }
    Ok(out)
}

fn find_crlf(haystack: &[u8], from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|rel| from + rel)
}

// =============================================================================
// `/system/df` JSON → DockerInventory mapping
// =============================================================================

/// JSON object member, treating explicit `null` as absent (the daemon emits
/// `"RepoTags": null` / `"UsageData": null` for dangling images and unused volumes).
fn member<'a>(obj: &'a Value, key: &str) -> Option<&'a Value> {
    obj.get(key).filter(|v| !v.is_null())
}

/// Non-negative integer field; negatives and non-integers clamp to 0.
fn json_u64(obj: &Value, key: &str) -> u64 {
    member(obj, key)
        .and_then(Value::as_i64)
        .map_or(0, |v| u64::try_from(v.max(0)).unwrap_or(0))
}

/// Non-negative integer field as `u32` (refcounts/layer counts); negatives
/// and non-integers clamp to 0, overflow saturates.
fn json_u32(obj: &Value, key: &str) -> u32 {
    member(obj, key)
        .and_then(Value::as_i64)
        .map_or(0, |v| u32::try_from(v.max(0)).unwrap_or(u32::MAX))
}

fn json_str<'a>(obj: &'a Value, key: &str) -> &'a str {
    member(obj, key).and_then(Value::as_str).unwrap_or_default()
}

fn json_array<'a>(obj: &'a Value, key: &str) -> &'a [Value] {
    member(obj, key)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn strip_sha256(s: &str) -> &str {
    s.strip_prefix("sha256:").unwrap_or(s)
}

/// Map a decoded `/system/df` document into a [`DockerInventory`]. Per-image
/// layer cache IDs are not exposed by this endpoint, so `layers` stays empty
/// and `layer_cache_ids` are unpopulated; `SharedSize` still feeds
/// `shared_bytes`, and build cache is the Σ of `BuildCache[].Size`.
fn inventory_from_df(df: &Value) -> DockerInventory {
    let mut images: Vec<ImageInfo> = json_array(df, "Images")
        .iter()
        .map(|img| {
            let id = json_str(img, "Id");
            let mut tags: Vec<String> = json_array(img, "RepoTags")
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect();
            // Dangling images may carry `<none>:<none>` placeholders.
            tags.retain(|tag| !tag.starts_with("<none>"));
            tags.sort();
            ImageInfo {
                id: strip_sha256(id).to_owned(),
                tags,
                size_bytes: json_u64(img, "Size"),
                shared_bytes: json_u64(img, "SharedSize"),
                // `/system/df` has no per-image layer list: filled per image
                // by `fill_layer_counts` after this mapping.
                layer_count: 0,
                // df reports how many containers use each image.
                ref_count: json_u32(img, "Containers"),
                layer_cache_ids: Vec::new(),
                created: json_u64(img, "Created"),
            }
        })
        .collect();
    images.sort_by(|a, b| a.id.cmp(&b.id));

    let mut containers: Vec<ContainerInfo> = json_array(df, "Containers")
        .iter()
        .map(|container| {
            let id = json_str(container, "Id");
            let trimmed = json_array(container, "Names")
                .first()
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim_start_matches('/');
            let name = if trimmed.is_empty() { id } else { trimmed };
            let image_id = strip_sha256(json_str(container, "ImageID"));
            ContainerInfo {
                id: id.to_owned(),
                name: name.to_owned(),
                image_id: if image_id.is_empty() {
                    None
                } else {
                    Some(image_id.to_owned())
                },
                rw_size_bytes: json_u64(container, "SizeRw"),
                log_bytes: 0,
                created: json_u64(container, "Created"),
            }
        })
        .collect();
    containers.sort_by(|a, b| a.id.cmp(&b.id));

    let mut volumes: Vec<VolumeInfo> = json_array(df, "Volumes")
        .iter()
        .map(|volume| VolumeInfo {
            name: json_str(volume, "Name").to_owned(),
            size_bytes: member(volume, "UsageData").map_or(0, |usage| json_u64(usage, "Size")),
            ref_count: member(volume, "UsageData").map_or(0, |usage| json_u32(usage, "RefCount")),
            // `CreatedAt` is RFC 3339 and absent on older daemons → 0.
            created: member(volume, "CreatedAt")
                .and_then(Value::as_str)
                .and_then(super::docker::parse_rfc3339_epoch)
                .unwrap_or(0),
        })
        .collect();
    volumes.sort_by(|a, b| a.name.cmp(&b.name));

    let build_cache_bytes = json_array(df, "BuildCache")
        .iter()
        .fold(0u64, |total, entry| {
            total.saturating_add(json_u64(entry, "Size"))
        });

    let mut inventory = DockerInventory {
        root: None,
        images,
        layers: Vec::new(),
        containers,
        volumes,
        build_cache_bytes,
        collected_at: super::docker::now_epoch(),
        source: InventorySource::DockerApi,
        totals: InventoryTotals::default(),
        warnings: Vec::new(),
    };
    inventory.compute_totals();
    inventory
}

/// Test-only path for a fake-daemon socket. `sun_path` holds just 104
/// (macOS) to 108 (Linux) bytes, so sockets never go under the checkout's
/// `target/` (arbitrarily deep); `$TMPDIR` is used when short enough, else
/// `/tmp`. The pid keeps concurrent runs from clobbering each other.
#[cfg(test)]
pub(crate) fn test_socket_path(name: &str) -> PathBuf {
    let file = format!("weshtatistic-{}-{name}.sock", std::process::id());
    let in_temp = std::env::temp_dir().join(&file);
    if in_temp.as_os_str().len() < 100 {
        in_temp
    } else {
        Path::new("/tmp").join(file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weshtatistic_core::WeshtatisticError;
    use weshtatistic_core::docker::DockerCollector;
    use std::sync::{Arc, Mutex};
    use std::{fs, os::unix::net::UnixListener};

    const IMAGE_ONE: &str = "aaaa1111bbbb2222cccc3333dddd4444eeee5555ffff6666aaaa1111bbbb2222";
    const IMAGE_TWO: &str = "9999aaaa8888bbbb7777cccc6666dddd5555eeee4444ffff3333222211110000";
    const CONTAINER_ONE: &str = "c0ffee42abc01234deadbeeff00dc0de";
    const VOLUME_ONE: &str = "weshtatistic-test-vol";

    /// Canned `/system/df` body: two images sharing 200 B, two containers with
    /// `SizeRw`, a sized volume and one with `UsageData: null`, two build cache
    /// entries (400 shared + 32 exclusive). The images carry df's `Containers`
    /// refcount, the sized volume a `RefCount` and RFC 3339 `CreatedAt`, so
    /// the mapping assertions cover the new fields; the per-image layer
    /// counts come from follow-up inspects (see the routed fake daemon).
    fn df_json() -> String {
        concat!(
            r#"{"LayersSize": 123456,"#,
            r#""Images": ["#,
            r#"{"Id": "sha256:IMAGE_TWO", "RepoTags": null, "Created": 1704207845, "Size": 250, "SharedSize": 200, "Containers": 0},"#,
            r#"{"Id": "sha256:IMAGE_ONE", "RepoTags": ["repo/one:latest", "repo/one:1.0"], "Created": 1686125350, "Size": 300, "SharedSize": 200, "Containers": 1}"#,
            r#"],"Containers": ["#,
            r#"{"Id": "c0ffee42abc", "Names": ["/api-container"], "ImageID": "sha256:IMAGE_ONE", "SizeRw": 30, "Created": 1704240000},"#,
            r#"{"Id": "deadbeef99", "Names": [], "ImageID": "sha256:IMAGE_TWO", "SizeRw": 5, "Created": 1704240001}"#,
            r#"],"Volumes": ["#,
            r#"{"Name": "vol-null", "UsageData": null},"#,
            r#"{"Name": "vol-sized", "CreatedAt": "2024-01-02T15:04:05Z", "UsageData": {"Size": 321, "RefCount": 2}}"#,
            r#"],"BuildCache": ["#,
            r#"{"Size": 400, "Shared": true}, {"Size": 32, "Shared": false}]}"#
        )
        .replace("IMAGE_ONE", IMAGE_ONE)
        .replace("IMAGE_TWO", IMAGE_TWO)
    }

    fn sock_path(name: &str) -> PathBuf {
        test_socket_path(name)
    }

    fn http_response(status_line: &str, body: &str) -> Vec<u8> {
        format!(
            "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .into_bytes()
    }

    fn chunked_response(body: &[u8]) -> Vec<u8> {
        let mid = body.len() / 2;
        let (a, b) = body.split_at(mid);
        let mut resp = Vec::new();
        resp.extend_from_slice(
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
        );
        // First chunk carries an extension to prove we strip it.
        resp.extend_from_slice(format!("{:x};ext=1\r\n", a.len()).as_bytes());
        resp.extend_from_slice(a);
        resp.extend_from_slice(b"\r\n");
        resp.extend_from_slice(format!("{:x}\r\n", b.len()).as_bytes());
        resp.extend_from_slice(b);
        resp.extend_from_slice(b"\r\n0\r\nX-Trailer: done\r\n\r\n");
        resp
    }

    /// A fake-daemon fixture: the bound socket path plus the raw request head
    /// the server recorded, shared with the test thread.
    type CapturedRequest = (PathBuf, Arc<Mutex<Vec<u8>>>);

    /// Bind a one-shot Unix socket server that reads the request headers,
    /// replies with `response`, and closes (dropping the listener with it).
    /// Returns the socket path and the raw request head the server received,
    /// so tests can assert the exact wire format.
    fn serve_once_capturing(
        name: &str,
        response: Vec<u8>,
    ) -> Result<CapturedRequest, WeshtatisticError> {
        let path = sock_path(name);
        let _ = fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let captured = Arc::new(Mutex::new(Vec::new()));
        let captured_server = Arc::clone(&captured);
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                // Consume the request head so the client is never blocked on write.
                let mut req = Vec::new();
                let mut buf = [0u8; 512];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => req.extend_from_slice(&buf[..n]),
                    }
                }
                if let Ok(mut guard) = captured_server.lock() {
                    *guard = req;
                }
                let _ = stream.write_all(&response);
                let _ = stream.flush();
            }
        });
        Ok((path, captured))
    }

    /// The raw request head the fake daemon received (empty if recording
    /// failed; the assertions then report the miss).
    fn captured_request(captured: &Arc<Mutex<Vec<u8>>>) -> String {
        captured
            .lock()
            .map(|guard| String::from_utf8_lossy(&guard).into_owned())
            .unwrap_or_default()
    }

    /// `serve_once_capturing` when the request bytes are not needed.
    fn serve_once(name: &str, response: Vec<u8>) -> Result<PathBuf, WeshtatisticError> {
        serve_once_capturing(name, response).map(|(path, _)| path)
    }

    /// Bind a fake daemon that answers `expected_requests` sequential
    /// connections by routing on the request line: the first route whose
    /// path key appears in the request line wins; unmatched requests get a
    /// bare 500. Used when one test exercises several endpoints (df plus
    /// per-image inspects).
    fn serve_routed(
        name: &str,
        routes: Vec<(&str, Vec<u8>)>,
        expected_requests: usize,
    ) -> Result<PathBuf, WeshtatisticError> {
        let path = sock_path(name);
        let _ = fs::remove_file(&path);
        let listener = UnixListener::bind(&path)?;
        let routes: Vec<(String, Vec<u8>)> = routes
            .into_iter()
            .map(|(key, response)| (key.to_owned(), response))
            .collect();
        std::thread::spawn(move || {
            for _ in 0..expected_requests {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                // Consume the request head so the client is never blocked.
                let mut req = Vec::new();
                let mut buf = [0u8; 512];
                while !req.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => req.extend_from_slice(&buf[..n]),
                    }
                }
                let request_line = req.split(|&b| b == b'\n').next().unwrap_or_default();
                let response = routes
                    .iter()
                    .find(|(key, _)| request_line.windows(key.len()).any(|w| w == key.as_bytes()))
                    .map_or_else(
                        || {
                            b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                                .to_vec()
                        },
                        |(_, response)| response.clone(),
                    );
                let _ = stream.write_all(&response);
                let _ = stream.flush();
            }
        });
        Ok(path)
    }

    fn image<'a>(inventory: &'a DockerInventory, id: &str) -> Option<&'a ImageInfo> {
        inventory.images.iter().find(|img| img.id == id)
    }

    #[test]
    fn test_collect_system_df_via_fake_daemon() -> Result<(), Box<dyn std::error::Error>> {
        let path = serve_once("df", http_response("200 OK", &df_json()))?;
        let inventory = collect_system_df(&path, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        assert_eq!(inventory.source, InventorySource::DockerApi);
        assert!(inventory.collected_at > 0);
        assert!(inventory.root.is_none());

        // Images: ids stripped of `sha256:`, sorted by id; tags sorted;
        // dangling image (RepoTags null) has none.
        assert_eq!(inventory.images.len(), 2);
        let img_one = image(&inventory, IMAGE_ONE).ok_or_else(|| missing("image one"))?;
        assert_eq!(img_one.tags, vec!["repo/one:1.0", "repo/one:latest"]);
        assert_eq!(img_one.size_bytes, 300);
        assert_eq!(img_one.shared_bytes, 200);
        assert_eq!(img_one.created, 1_686_125_350);
        assert_eq!(img_one.layer_cache_ids, Vec::<String>::new());
        // The one-shot server has no inspect route: df lacks layers, so the
        // count stays 0; df's `Containers` field feeds ref_count.
        assert_eq!(img_one.layer_count, 0);
        assert_eq!(img_one.ref_count, 1);
        let img_two = image(&inventory, IMAGE_TWO).ok_or_else(|| missing("image two"))?;
        assert_eq!(img_two.tags, Vec::<String>::new());
        assert_eq!(img_two.size_bytes, 250);
        assert_eq!(img_two.shared_bytes, 200);
        assert_eq!(img_two.layer_count, 0);
        assert_eq!(img_two.ref_count, 0);

        // Containers: sorted by id; `/` trimmed from Names; empty Names fall
        // back to the id; image_id stripped.
        assert_eq!(inventory.containers.len(), 2);
        let c0ffee = &inventory.containers[0];
        assert_eq!(c0ffee.id, "c0ffee42abc");
        assert_eq!(c0ffee.name, "api-container");
        assert_eq!(c0ffee.image_id.as_deref(), Some(IMAGE_ONE));
        assert_eq!(c0ffee.rw_size_bytes, 30);
        assert_eq!(c0ffee.log_bytes, 0);
        let deadbeef = &inventory.containers[1];
        assert_eq!(deadbeef.name, "deadbeef99");
        assert_eq!(deadbeef.rw_size_bytes, 5);
        assert_eq!(deadbeef.created, 1_704_240_001);

        // Volumes sorted by name; null UsageData maps to 0 (disk fallback is
        // a separate concern tested below); RefCount/CreatedAt come from df.
        assert_eq!(inventory.volumes.len(), 2);
        assert_eq!(inventory.volumes[0].name, "vol-null");
        assert_eq!(inventory.volumes[0].size_bytes, 0);
        assert_eq!(inventory.volumes[0].ref_count, 0);
        assert_eq!(inventory.volumes[0].created, 0);
        assert_eq!(inventory.volumes[1].name, "vol-sized");
        assert_eq!(inventory.volumes[1].size_bytes, 321);
        assert_eq!(inventory.volumes[1].ref_count, 2);
        assert_eq!(
            inventory.volumes[1].created, 1_704_207_845,
            "CreatedAt is RFC 3339 → epoch"
        );

        assert_eq!(inventory.build_cache_bytes, 432);

        let totals = &inventory.totals;
        assert_eq!(totals.images_bytes, 550);
        assert_eq!(totals.shared_bytes, 400);
        assert_eq!(totals.container_rw_bytes, 35);
        assert_eq!(totals.volumes_bytes, 321);
        assert_eq!(totals.build_cache_bytes, 432);
        assert_eq!(totals.log_bytes, 0);
        Ok(())
    }

    #[test]
    fn test_collect_system_df_chunked_via_fake_daemon() -> Result<(), Box<dyn std::error::Error>> {
        let path = serve_once("chunked", chunked_response(df_json().as_bytes()))?;
        let inventory = collect_system_df(&path, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        assert_eq!(inventory.images.len(), 2);
        assert_eq!(inventory.containers.len(), 2);
        assert_eq!(inventory.volumes.len(), 2);
        assert_eq!(inventory.build_cache_bytes, 432);
        Ok(())
    }

    #[test]
    fn test_collect_system_df_layer_counts_via_routed_daemon()
    -> Result<(), Box<dyn std::error::Error>> {
        // df names two images; collection must follow up with one inspect
        // per image. IMAGE_ONE resolves to a 3-layer RootFS; IMAGE_TWO's
        // inspect 404s and must fall back to 0 without failing collection.
        let inspect_one = http_response(
            "200 OK",
            concat!(
                r#"{"Id": "sha256:IMAGE_ONE","#,
                r#""RootFS": {"Type": "layers","#,
                r#""Layers": ["sha256:l1", "sha256:l2", "sha256:l3"]}}"#
            )
            .replace("IMAGE_ONE", IMAGE_ONE)
            .as_str(),
        );
        let inspect_missing = http_response(
            "404 Not Found",
            r#"{"message":"No such image: sha256:IMAGE_TWO"}"#
                .replace("IMAGE_TWO", IMAGE_TWO)
                .as_str(),
        );
        let inspect_one_key = format!(" /v1.44/images/{IMAGE_ONE}/json HTTP/");
        let inspect_two_key = format!(" /v1.44/images/{IMAGE_TWO}/json HTTP/");
        let path = serve_routed(
            "routed_df",
            vec![
                (
                    " /v1.44/system/df HTTP/",
                    http_response("200 OK", &df_json()),
                ),
                (inspect_one_key.as_str(), inspect_one),
                (inspect_two_key.as_str(), inspect_missing),
            ],
            3,
        )?;
        let inventory = collect_system_df(&path, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        let img_one = image(&inventory, IMAGE_ONE).ok_or_else(|| missing("image one"))?;
        assert_eq!(img_one.layer_count, 3, "inspect RootFS.Layers length");
        assert_eq!(img_one.ref_count, 1, "df's Containers refcount");
        let img_two = image(&inventory, IMAGE_TWO).ok_or_else(|| missing("image two"))?;
        assert_eq!(
            img_two.layer_count, 0,
            "a failed inspect leaves the count at 0"
        );
        assert_eq!(img_two.ref_count, 0);
        Ok(())
    }

    #[test]
    fn test_collect_system_df_non_200() -> Result<(), Box<dyn std::error::Error>> {
        let path = serve_once(
            "non200",
            http_response("500 Internal Server Error", "{\"message\":\"boom\"}"),
        )?;
        let result = collect_system_df(&path, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Status(500))),
            "expected Status(500), got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_collect_system_df_garbage_body() -> Result<(), Box<dyn std::error::Error>> {
        let path = serve_once("garbage", http_response("200 OK", "this is not json"))?;
        let result = collect_system_df(&path, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Decode(_))),
            "expected Decode, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_collect_system_df_socket_missing() {
        let path = sock_path("does_not_exist");
        let _ = fs::remove_file(&path);
        let result = collect_system_df(&path, &AtomicBool::new(false));
        assert!(
            matches!(result, Err(DockerApiError::Connect(_))),
            "expected Connect, got {result:?}"
        );
    }

    #[test]
    fn test_collect_system_df_cancelled() {
        let path = sock_path("cancelled");
        let cancel = AtomicBool::new(true);
        let result = collect_system_df(&path, &cancel);
        assert!(
            matches!(result, Err(DockerApiError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
    }

    #[test]
    fn test_delete_image_via_fake_daemon() -> Result<(), Box<dyn std::error::Error>> {
        let body = concat!(
            r#"[{"Untagged":"repo/one:1.0"},"#,
            r#"{"Untagged":"repo/one:latest"},"#,
            r#"{"Deleted":"sha256:aaaa1111bbbb"},"#,
            r#"{"Deleted":"sha256:cccc3333dddd"}]"#
        );
        let (path, captured) = serve_once_capturing("delete_ok", http_response("200 OK", body))?;
        let deletion = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        // The wire format: DELETE on the bare image id, force/noprune query.
        let head = captured_request(&captured);
        assert!(
            head.contains(&format!(
                "DELETE /v1.44/images/{IMAGE_ONE}?force=false&noprune=false HTTP/1.1\r\n"
            )) && head.contains("Host: docker\r\n"),
            "unexpected request head: {head:?}"
        );

        // Mixed Untagged/Deleted entries land in their own lists, in order.
        assert_eq!(
            deletion.untagged,
            vec!["repo/one:1.0".to_owned(), "repo/one:latest".to_owned()]
        );
        assert_eq!(
            deletion.deleted,
            vec![
                "sha256:aaaa1111bbbb".to_owned(),
                "sha256:cccc3333dddd".to_owned()
            ]
        );
        Ok(())
    }

    #[test]
    fn test_delete_image_force_true_query() -> Result<(), Box<dyn std::error::Error>> {
        let (path, captured) = serve_once_capturing(
            "delete_force",
            http_response("200 OK", r#"[{"Deleted":"sha256:beef"}]"#),
        )?;
        let deletion = delete_image(&path, IMAGE_ONE, true, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        let head = captured_request(&captured);
        assert!(
            head.contains(&format!(
                "DELETE /v1.44/images/{IMAGE_ONE}?force=true&noprune=false HTTP/1.1\r\n"
            )),
            "unexpected request head: {head:?}"
        );
        assert_eq!(deletion.untagged, Vec::<String>::new());
        assert_eq!(deletion.deleted, vec!["sha256:beef".to_owned()]);
        Ok(())
    }

    #[test]
    fn test_delete_image_conflict_carries_daemon_message() -> Result<(), Box<dyn std::error::Error>>
    {
        let body = r#"{"message":"image is being used by stopped container xyz"}"#;
        let path = serve_once("delete_409", http_response("409 Conflict", body))?;
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        match result {
            Err(DockerApiError::Daemon {
                status: 409,
                message,
            }) => {
                assert_eq!(message, "image is being used by stopped container xyz");
            }
            other => return Err(missing(&format!("expected Daemon 409, got {other:?}")).into()),
        }
        Ok(())
    }

    #[test]
    fn test_delete_image_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let body = r#"{"message":"No such image: weshtatistic-nonexistent"}"#;
        let path = serve_once("delete_404", http_response("404 Not Found", body))?;
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Daemon { status: 404, .. })),
            "expected Daemon 404, got {result:?}"
        );

        // A 404 body without a parseable `message` degrades to a bare status.
        let path = serve_once("delete_404_bare", http_response("404 Not Found", "nope"))?;
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Status(404))),
            "expected Status(404), got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_delete_image_malformed_success_body() -> Result<(), Box<dyn std::error::Error>> {
        // Garbage body: not JSON at all.
        let path = serve_once(
            "delete_garbage",
            http_response("200 OK", "this is not json"),
        )?;
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Decode(_))),
            "expected Decode, got {result:?}"
        );

        // Well-formed JSON, wrong shape: an object is not the expected array
        // (serde rejects it → Decode); an array whose entry is not an object
        // parses but has the wrong shape (→ InvalidResponse).
        let path = serve_once(
            "delete_obj",
            http_response("200 OK", r#"{"Deleted":"sha256:x"}"#),
        )?;
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Decode(_))),
            "expected Decode, got {result:?}"
        );

        let path = serve_once("delete_arr", http_response("200 OK", r"[42]"))?;
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::InvalidResponse(_))),
            "expected InvalidResponse, got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_delete_image_socket_missing() {
        let path = sock_path("delete_missing");
        let _ = fs::remove_file(&path);
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(false));
        assert!(
            matches!(result, Err(DockerApiError::Connect(_))),
            "expected Connect, got {result:?}"
        );
    }

    #[test]
    fn test_delete_image_cancelled() {
        let path = sock_path("delete_cancelled");
        let result = delete_image(&path, IMAGE_ONE, false, &AtomicBool::new(true));
        assert!(
            matches!(result, Err(DockerApiError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
    }

    #[test]
    fn test_delete_container_via_fake_daemon() -> Result<(), Box<dyn std::error::Error>> {
        // Success is an empty 204 No Content; a bare 200 is tolerated too.
        // Neither carries a body to decode.
        for (name, status) in [
            ("container_204", "204 No Content"),
            ("container_200", "200 OK"),
        ] {
            let (path, captured) = serve_once_capturing(name, http_response(status, ""))?;
            delete_container(&path, CONTAINER_ONE, false, &AtomicBool::new(false))?;
            let _ = fs::remove_file(&path);

            let head = captured_request(&captured);
            assert!(
                head.contains(&format!(
                    "DELETE /v1.44/containers/{CONTAINER_ONE}?force=false&v=false HTTP/1.1\r\n"
                )) && head.contains("Host: docker\r\n"),
                "{name}: unexpected request head: {head:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_delete_container_force_true_query() -> Result<(), Box<dyn std::error::Error>> {
        let (path, captured) =
            serve_once_capturing("container_force", http_response("204 No Content", ""))?;
        delete_container(&path, CONTAINER_ONE, true, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        let head = captured_request(&captured);
        assert!(
            head.contains(&format!(
                "DELETE /v1.44/containers/{CONTAINER_ONE}?force=true&v=false HTTP/1.1\r\n"
            )),
            "unexpected request head: {head:?}"
        );
        Ok(())
    }

    #[test]
    fn test_delete_container_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let body = r#"{"message":"No such container: weshtatistic-nonexistent"}"#;
        let path = serve_once("container_404", http_response("404 Not Found", body))?;
        let result = delete_container(&path, CONTAINER_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Daemon { status: 404, .. })),
            "expected Daemon 404, got {result:?}"
        );

        // A 404 body without a parseable `message` degrades to a bare status.
        let path = serve_once("container_404_bare", http_response("404 Not Found", ""))?;
        let result = delete_container(&path, CONTAINER_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Status(404))),
            "expected Status(404), got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_delete_container_socket_missing() {
        let path = sock_path("container_missing");
        let _ = fs::remove_file(&path);
        let result = delete_container(&path, CONTAINER_ONE, false, &AtomicBool::new(false));
        assert!(
            matches!(result, Err(DockerApiError::Connect(_))),
            "expected Connect, got {result:?}"
        );
    }

    #[test]
    fn test_delete_container_cancelled() {
        let path = sock_path("container_cancelled");
        let result = delete_container(&path, CONTAINER_ONE, false, &AtomicBool::new(true));
        assert!(
            matches!(result, Err(DockerApiError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
    }

    #[test]
    fn test_delete_volume_via_fake_daemon() -> Result<(), Box<dyn std::error::Error>> {
        // Success is an empty 204 No Content; a bare 200 is tolerated too.
        for (name, status) in [("volume_204", "204 No Content"), ("volume_200", "200 OK")] {
            let (path, captured) = serve_once_capturing(name, http_response(status, ""))?;
            delete_volume(&path, VOLUME_ONE, false, &AtomicBool::new(false))?;
            let _ = fs::remove_file(&path);

            let head = captured_request(&captured);
            assert!(
                head.contains(&format!(
                    "DELETE /v1.44/volumes/{VOLUME_ONE}?force=false HTTP/1.1\r\n"
                )) && head.contains("Host: docker\r\n"),
                "{name}: unexpected request head: {head:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn test_delete_volume_force_true_query() -> Result<(), Box<dyn std::error::Error>> {
        let (path, captured) =
            serve_once_capturing("volume_force", http_response("204 No Content", ""))?;
        delete_volume(&path, VOLUME_ONE, true, &AtomicBool::new(false))?;
        let _ = fs::remove_file(&path);

        let head = captured_request(&captured);
        assert!(
            head.contains(&format!(
                "DELETE /v1.44/volumes/{VOLUME_ONE}?force=true HTTP/1.1\r\n"
            )),
            "unexpected request head: {head:?}"
        );
        Ok(())
    }

    #[test]
    fn test_delete_volume_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let body = r#"{"message":"get weshtatistic-nonexistent: no such volume"}"#;
        let path = serve_once("volume_404", http_response("404 Not Found", body))?;
        let result = delete_volume(&path, VOLUME_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Daemon { status: 404, .. })),
            "expected Daemon 404, got {result:?}"
        );

        // A 404 body without a parseable `message` degrades to a bare status.
        let path = serve_once("volume_404_bare", http_response("404 Not Found", ""))?;
        let result = delete_volume(&path, VOLUME_ONE, false, &AtomicBool::new(false));
        let _ = fs::remove_file(&path);
        assert!(
            matches!(result, Err(DockerApiError::Status(404))),
            "expected Status(404), got {result:?}"
        );
        Ok(())
    }

    #[test]
    fn test_delete_volume_socket_missing() {
        let path = sock_path("volume_missing");
        let _ = fs::remove_file(&path);
        let result = delete_volume(&path, VOLUME_ONE, false, &AtomicBool::new(false));
        assert!(
            matches!(result, Err(DockerApiError::Connect(_))),
            "expected Connect, got {result:?}"
        );
    }

    #[test]
    fn test_delete_volume_cancelled() {
        let path = sock_path("volume_cancelled");
        let result = delete_volume(&path, VOLUME_ONE, false, &AtomicBool::new(true));
        assert!(
            matches!(result, Err(DockerApiError::Cancelled)),
            "expected Cancelled, got {result:?}"
        );
    }

    /// Real-daemon smoke test, ignored by default (`nextest` skips
    /// `#[ignore]`d tests, so CI never runs it). Deletes a
    /// definitely-nonexistent image id — the host daemon is never mutated —
    /// and requires the daemon's own 404, which also validates the wire
    /// format against a real peer.
    #[test]
    #[ignore = "requires a live Docker daemon; only ever deletes a nonexistent image"]
    fn test_delete_image_real_daemon_reports_not_found() -> Result<(), Box<dyn std::error::Error>> {
        let Some(socket) = discover_socket(|key| std::env::var_os(key)) else {
            return Ok(()); // No daemon on this host: nothing to validate.
        };
        let id = format!("weshtatistic-nonexistent-{}", std::process::id());
        let result = delete_image(&socket, &id, false, &AtomicBool::new(false));
        match result {
            Err(DockerApiError::Daemon {
                status: 404,
                message,
            }) => {
                assert!(
                    !message.is_empty(),
                    "a real 404 must carry the daemon's message"
                );
                Ok(())
            }
            Err(DockerApiError::Status(404)) => Ok(()), // message-less 404: still a 404
            other => Err(missing(&format!("expected a 404, got {other:?}")).into()),
        }
    }

    /// `POST /v1.44/images/create` on the daemon: pull `image:tag`, draining
    /// the streaming progress body to EOF. Real-daemon tests only — the
    /// production collection paths never pull.
    fn pull_image(
        socket_path: &Path,
        image: &str,
        tag: &str,
        cancel: &AtomicBool,
    ) -> Result<(), DockerApiError> {
        let request = format!(
            "POST /v1.44/images/create?fromImage={image}&tag={tag} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n"
        );
        let response = perform_request(socket_path, request.as_bytes(), cancel)?;
        match response.status {
            200 => Ok(()),
            other => Err(DockerApiError::Status(other)),
        }
    }

    /// `GET /v1.44/images/{name}/json` → the image's `.Id` (`Ok(None)` when
    /// the image does not exist: HTTP 404). Real-daemon tests only.
    fn find_image_id(
        socket_path: &Path,
        name: &str,
        cancel: &AtomicBool,
    ) -> Result<Option<String>, DockerApiError> {
        let request = format!(
            "GET /v1.44/images/{name}/json HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n"
        );
        let response = perform_request(socket_path, request.as_bytes(), cancel)?;
        match response.status {
            200 => {
                let json: Value =
                    serde_json::from_slice(&response.body).map_err(DockerApiError::Decode)?;
                Ok(member(&json, "Id")
                    .and_then(Value::as_str)
                    .map(str::to_owned))
            }
            404 => Ok(None),
            other => Err(DockerApiError::Status(other)),
        }
    }

    /// Real-daemon round trip for the delete happy path, ignored by default
    /// (`nextest` skips `#[ignore]`d tests, so CI never runs it). Pulls
    /// `hello-world:latest` through the daemon, resolves its id, deletes it
    /// through the production collector path, and requires the image to be
    /// gone afterwards. Every environmental gap (no socket, no registry
    /// access) self-skips with `Ok(())` — this test only fails on genuine
    /// delete-path breakage, and never touches images other than
    /// `hello-world:latest`.
    ///
    /// Run explicitly with:
    /// `cargo nextest run -p weshtatistic --run-ignored all -E 'test(real_daemon)'`
    #[test]
    #[ignore = "mutates a live Docker daemon (pulls + deletes hello-world:latest)"]
    fn test_delete_image_real_daemon_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let Some(socket) = discover_socket(|key| std::env::var_os(key)) else {
            return Ok(()); // No daemon on this host: nothing to validate.
        };
        let cancel = AtomicBool::new(false);

        // Environmental: without registry access there is nothing to delete.
        if pull_image(&socket, "hello-world", "latest", &cancel).is_err() {
            return Ok(());
        }
        let Some(id) = find_image_id(&socket, "hello-world:latest", &cancel)? else {
            return Ok(()); // The pull did not land: nothing to delete.
        };

        // Production path: delete through the collector.
        let collector = crate::engine::docker::NativeDockerCollector::new();
        let result = collector.delete_image(&id, false);
        let deletion = match result {
            Ok(deletion) => deletion,
            Err(error) => {
                // Keep repeated runs clean when the delete path broke.
                let _ = delete_image(&socket, &id, true, &cancel);
                return Err(missing(&format!("delete_image failed: {error}")).into());
            }
        };
        assert!(
            !deletion.untagged.is_empty() || !deletion.deleted.is_empty(),
            "a successful delete must report at least one untagged/deleted entry"
        );
        assert!(
            matches!(
                find_image_id(&socket, "hello-world:latest", &cancel),
                Ok(None)
            ),
            "hello-world:latest must be gone after deletion"
        );
        Ok(())
    }

    /// `POST /v1.44/containers/create` with a tiny JSON body; returns the new
    /// container's id (201 → `{"Id": …}`). Real-daemon tests only.
    fn create_container(
        socket_path: &Path,
        name: &str,
        cancel: &AtomicBool,
    ) -> Result<String, DockerApiError> {
        let body = r#"{"Image":"hello-world:latest","Cmd":["/hello"]}"#;
        let request = format!(
            "POST /v1.44/containers/create?name={name} HTTP/1.1\r\nHost: docker\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let response = perform_request(socket_path, request.as_bytes(), cancel)?;
        match response.status {
            201 => {
                let json: Value =
                    serde_json::from_slice(&response.body).map_err(DockerApiError::Decode)?;
                member(&json, "Id")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .ok_or(DockerApiError::InvalidResponse("create response has no Id"))
            }
            404 | 409 => match daemon_message(&response.body) {
                Some(message) => Err(DockerApiError::Daemon {
                    status: response.status,
                    message,
                }),
                None => Err(DockerApiError::Status(response.status)),
            },
            other => Err(DockerApiError::Status(other)),
        }
    }

    /// `GET /v1.44/containers/{id}/json`: `Ok(true)` while the container
    /// exists, `Ok(false)` on 404. Real-daemon tests only.
    fn container_exists(
        socket_path: &Path,
        id: &str,
        cancel: &AtomicBool,
    ) -> Result<bool, DockerApiError> {
        let request = format!(
            "GET /v1.44/containers/{id}/json HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n"
        );
        let response = perform_request(socket_path, request.as_bytes(), cancel)?;
        match response.status {
            200 => Ok(true),
            404 => Ok(false),
            other => Err(DockerApiError::Status(other)),
        }
    }

    /// `POST /v1.44/volumes/create` with `{"Name": …}` (201). The daemon
    /// reuses an existing name, so a leftover from an aborted run is simply
    /// reused and then deleted. Real-daemon tests only.
    fn create_volume(
        socket_path: &Path,
        name: &str,
        cancel: &AtomicBool,
    ) -> Result<(), DockerApiError> {
        let body = format!(r#"{{"Name":"{name}"}}"#);
        let request = format!(
            "POST /v1.44/volumes/create HTTP/1.1\r\nHost: docker\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let response = perform_request(socket_path, request.as_bytes(), cancel)?;
        match response.status {
            201 => Ok(()),
            404 | 409 => match daemon_message(&response.body) {
                Some(message) => Err(DockerApiError::Daemon {
                    status: response.status,
                    message,
                }),
                None => Err(DockerApiError::Status(response.status)),
            },
            other => Err(DockerApiError::Status(other)),
        }
    }

    /// `GET /v1.44/volumes/{name}`: `Ok(true)` while the volume exists,
    /// `Ok(false)` on 404. Real-daemon tests only.
    fn volume_exists(
        socket_path: &Path,
        name: &str,
        cancel: &AtomicBool,
    ) -> Result<bool, DockerApiError> {
        let request = format!(
            "GET /v1.44/volumes/{name} HTTP/1.1\r\nHost: docker\r\nConnection: close\r\n\r\n"
        );
        let response = perform_request(socket_path, request.as_bytes(), cancel)?;
        match response.status {
            200 => Ok(true),
            404 => Ok(false),
            other => Err(DockerApiError::Status(other)),
        }
    }

    /// Real-daemon round trip for container deletion, ignored by default
    /// (`nextest` skips `#[ignore]`d tests, so CI never runs it). Creates a
    /// stopped `hello-world` container, deletes it through the production
    /// collector path, and requires it to be gone. Environmental gaps (no
    /// socket, no registry access) self-skip with `Ok(())`; only the daemon
    /// object created here is ever touched.
    #[test]
    #[ignore = "mutates a live Docker daemon (creates + deletes a test container)"]
    fn test_delete_container_real_daemon_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let Some(socket) = discover_socket(|key| std::env::var_os(key)) else {
            return Ok(()); // No daemon on this host: nothing to validate.
        };
        let cancel = AtomicBool::new(false);

        // Environmental: without registry access there is no image to run.
        if pull_image(&socket, "hello-world", "latest", &cancel).is_err() {
            return Ok(());
        }
        let name = format!("weshtatistic-deltest-{}", std::process::id());
        let mut created = create_container(&socket, &name, &cancel);
        if created.is_err() {
            // Likely a stale same-name container from an aborted run:
            // remove it and retry once before giving up.
            let _ = delete_container(&socket, &name, true, &cancel);
            created = create_container(&socket, &name, &cancel);
        }
        let Ok(id) = created else {
            return Ok(());
        };

        // Production path: delete through the collector.
        let collector = crate::engine::docker::NativeDockerCollector::new();
        if let Err(error) = collector.delete_container(&id, false) {
            let _ = delete_container(&socket, &id, true, &cancel);
            return Err(missing(&format!("delete_container failed: {error}")).into());
        }
        assert!(
            matches!(container_exists(&socket, &id, &cancel), Ok(false)),
            "container {name} must be gone after deletion"
        );
        Ok(())
    }

    /// Real-daemon round trip for volume deletion, ignored by default.
    /// Creates a named volume, deletes it through the production collector
    /// path, and requires it to be gone. Only the volume created here is
    /// ever touched.
    #[test]
    #[ignore = "mutates a live Docker daemon (creates + deletes a test volume)"]
    fn test_delete_volume_real_daemon_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
        let Some(socket) = discover_socket(|key| std::env::var_os(key)) else {
            return Ok(()); // No daemon on this host: nothing to validate.
        };
        let cancel = AtomicBool::new(false);

        let name = format!("weshtatistic-deltest-{}", std::process::id());
        if create_volume(&socket, &name, &cancel).is_err() {
            return Ok(()); // Daemon refused (environmental): nothing to delete.
        }

        // Production path: delete through the collector.
        let collector = crate::engine::docker::NativeDockerCollector::new();
        if let Err(error) = collector.delete_volume(&name, false) {
            let _ = delete_volume(&socket, &name, true, &cancel);
            return Err(missing(&format!("delete_volume failed: {error}")).into());
        }
        assert!(
            matches!(volume_exists(&socket, &name, &cancel), Ok(false)),
            "volume {name} must be gone after deletion"
        );
        Ok(())
    }

    #[test]
    fn test_discover_socket_docker_host_wins() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::current_dir()?
            .join("target")
            .join("test_docker_api_env");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        let explicit = dir.join("explicit.sock");
        fs::write(&explicit, b"")?;

        // `DOCKER_HOST` is injected; every other variable is the real one.
        let with_host = |host: String| {
            move |key: &str| {
                if key == "DOCKER_HOST" {
                    Some(OsString::from(&host))
                } else {
                    std::env::var_os(key)
                }
            }
        };
        let unix_host = format!("unix://{}", explicit.display());
        assert_eq!(
            discover_socket(with_host(unix_host)).as_deref(),
            Some(explicit.as_path())
        );

        // A tcp:// DOCKER_HOST is not a Unix socket: discovery must ignore it.
        let found = discover_socket(with_host("tcp://127.0.0.1:2375".to_owned()));
        assert!(
            found.is_none() || found.as_deref() != Some(explicit.as_path()),
            "tcp:// DOCKER_HOST must not select the unix path"
        );

        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn test_enrich_from_disk_fills_logs_and_volume_sizes() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = std::env::current_dir()?
            .join("target")
            .join("test_docker_api_enrich");
        let _ = fs::remove_dir_all(&root);
        // c0ffee has a 600-byte json log on disk; deadbeef has none.
        let c0ffee_dir = root.join("containers").join("c0ffee42abc");
        fs::create_dir_all(&c0ffee_dir)?;
        fs::write(c0ffee_dir.join("c0ffee42abc-json.log"), vec![0u8; 600])?;
        // vol-null's `_data` holds 123 B on disk; vol-sized already has an
        // API size and must not be re-walked.
        fs::create_dir_all(root.join("volumes").join("vol-null").join("_data"))?;
        fs::write(
            root.join("volumes")
                .join("vol-null")
                .join("_data")
                .join("blob.bin"),
            vec![0u8; 123],
        )?;
        fs::create_dir_all(root.join("volumes").join("vol-sized").join("_data"))?;
        fs::write(
            root.join("volumes")
                .join("vol-sized")
                .join("_data")
                .join("ignored.bin"),
            vec![0u8; 999],
        )?;

        let mut inventory = inventory_from_df(&serde_json::from_str(&df_json())?);
        enrich_from_disk(&mut inventory, &root, &AtomicBool::new(false));
        let _ = fs::remove_dir_all(&root);

        assert_eq!(inventory.containers[0].log_bytes, 600);
        assert_eq!(inventory.containers[1].log_bytes, 0);
        assert_eq!(
            inventory.volumes[0].size_bytes, 123,
            "zero API size falls back to disk"
        );
        assert_eq!(
            inventory.volumes[1].size_bytes, 321,
            "API size wins over disk"
        );
        assert_eq!(inventory.totals.log_bytes, 600);
        assert_eq!(inventory.totals.volumes_bytes, 321 + 123);
        Ok(())
    }

    #[test]
    fn test_parse_http_response_content_length() -> Result<(), Box<dyn std::error::Error>> {
        let raw = http_response("200 OK", "hello");
        let resp = parse_http_response(&raw)?;
        assert_eq!(resp.status, 200);
        assert_eq!(resp.body, b"hello");
        Ok(())
    }

    #[test]
    fn test_parse_http_response_read_until_eof_without_length() {
        let raw = b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\nhello world";
        let resp = parse_http_response(raw);
        assert!(matches!(resp, Ok(ref r) if r.status == 200 && r.body == b"hello world"));
    }

    #[test]
    fn test_parse_http_response_chunked_roundtrip() {
        let resp = parse_http_response(&chunked_response(b"abcdefgh"));
        assert!(
            matches!(resp, Ok(ref r) if r.status == 200 && r.body == b"abcdefgh"),
            "chunked decode failed: {resp:?}"
        );
    }

    #[test]
    fn test_parse_http_response_malformed() {
        // Missing header/body separator.
        assert!(matches!(
            parse_http_response(b"HTTP/1.1 200 OK\r\nContent-Length: 5"),
            Err(DockerApiError::InvalidResponse(_))
        ));
        // Not an HTTP status line.
        assert!(matches!(
            parse_http_response(b"HELLO\r\n\r\n"),
            Err(DockerApiError::InvalidResponse(_))
        ));
        // Body shorter than Content-Length.
        assert!(matches!(
            parse_http_response(b"HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\nshort"),
            Err(DockerApiError::InvalidResponse(_))
        ));
        // Truncated chunk data.
        assert!(matches!(
            parse_http_response(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nab"),
            Err(DockerApiError::InvalidResponse(_))
        ));
        // Garbage chunk size.
        assert!(matches!(
            parse_http_response(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\nab\r\n0\r\n\r\n"
            ),
            Err(DockerApiError::InvalidResponse(_))
        ));
    }

    #[test]
    fn test_inventory_from_df_ignores_unexpected_shapes() {
        // Missing collections and wrong-typed fields must map to empties, errors.
        let empty = inventory_from_df(&serde_json::json!({}));
        assert!(empty.images.is_empty());
        assert!(empty.containers.is_empty());
        assert!(empty.volumes.is_empty());
        assert_eq!(empty.build_cache_bytes, 0);
        assert_eq!(empty.source, InventorySource::DockerApi);

        let weird = inventory_from_df(&serde_json::json!({
            "Images": [{"Id": 42, "Size": "lots", "Created": -5}],
        }));
        assert_eq!(weird.images.len(), 1);
        assert_eq!(weird.images[0].id, "");
        assert_eq!(weird.images[0].size_bytes, 0);
        assert_eq!(weird.images[0].created, 0);
    }

    fn missing(what: &str) -> WeshtatisticError {
        WeshtatisticError::Io(std::io::Error::new(
            ErrorKind::NotFound,
            format!("fixture missing {what}"),
        ))
    }
}
