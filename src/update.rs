//! Self-updater: checks GitHub Releases for a newer PikCraft, downloads the matching
//! executable, verifies its SHA-256 digest and swaps it in place of the running binary.
//!
//! All network work happens on background threads; the game polls for progress.

use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

pub const REPO: &str = "malik05051/pikcraft";
pub const RELEASES_PAGE: &str = "https://github.com/malik05051/pikcraft/releases";

/// The raw executable each release publishes for this platform.
pub const ASSET_NAME: &str = if cfg!(windows) {
    "pikcraft-windows-x86_64.exe"
} else {
    "pikcraft-linux-x86_64"
};

pub fn current_version() -> (u32, u32, u32) {
    parse_version(env!("CARGO_PKG_VERSION")).unwrap_or((0, 0, 0))
}

/// "v1.2.3" / "1.2.3" / "1.2" → (1, 2, 3). Pre-release suffixes ("1.2.3-beta") are ignored.
pub fn parse_version(tag: &str) -> Option<(u32, u32, u32)> {
    let t = tag.trim().trim_start_matches(['v', 'V']);
    let t = t.split(['-', '+']).next()?;
    let mut parts = t.split('.').map(|p| p.parse::<u32>());
    let major = parts.next()?.ok()?;
    let minor = parts.next().unwrap_or(Ok(0)).ok()?;
    let patch = parts.next().unwrap_or(Ok(0)).ok()?;
    Some((major, minor, patch))
}

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    pub tag: String,
    pub version: (u32, u32, u32),
    pub url: String,
    pub size: u64,
    /// Lower-case hex SHA-256 published by GitHub for the asset, when available.
    pub sha256: Option<String>,
}

/// Pick the newest release worth installing from a GitHub `releases/latest` JSON document.
pub fn parse_latest(json: &str, current: (u32, u32, u32)) -> Result<Option<Release>, String> {
    let v: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("bad release data: {e}"))?;
    if v["draft"].as_bool() == Some(true) || v["prerelease"].as_bool() == Some(true) {
        return Ok(None);
    }
    let tag = v["tag_name"]
        .as_str()
        .ok_or("release has no tag")?
        .to_string();
    let Some(version) = parse_version(&tag) else {
        return Ok(None);
    };
    if version <= current {
        return Ok(None);
    }
    let assets = v["assets"].as_array().ok_or("release has no assets")?;
    let asset = assets
        .iter()
        .find(|a| a["name"].as_str() == Some(ASSET_NAME))
        .ok_or_else(|| format!("{tag} has no {ASSET_NAME} download"))?;
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or("asset has no download URL")?
        .to_string();
    let sha256 = asset["digest"]
        .as_str()
        .and_then(|d| d.strip_prefix("sha256:"))
        .map(|d| d.to_ascii_lowercase());
    Ok(Some(Release {
        tag,
        version,
        url,
        size: asset["size"].as_u64().unwrap_or(0),
        sha256,
    }))
}

fn agent(roots: ureq::tls::RootCerts) -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(roots).build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(Duration::from_secs(300)))
        .user_agent(format!("PikCraft/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// GET with the OS trust store first (works behind corporate TLS proxies), then Mozilla's
/// bundled roots if the OS verifier can't decide (e.g. revocation servers unreachable).
fn get(url: &str, accept: &str) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    use ureq::tls::RootCerts;
    let first = agent(RootCerts::PlatformVerifier)
        .get(url)
        .header("Accept", accept)
        .call();
    match first {
        Err(e) if !matches!(e, ureq::Error::StatusCode(_)) => agent(RootCerts::WebPki)
            .get(url)
            .header("Accept", accept)
            .call()
            .map_err(|_| e),
        other => other,
    }
}

/// API root, overridable for testing (`PIKCRAFT_UPDATE_API=http://127.0.0.1:8000`).
fn api_base() -> String {
    std::env::var("PIKCRAFT_UPDATE_API").unwrap_or_else(|_| "https://api.github.com".to_string())
}

fn fetch_latest() -> Result<Option<Release>, String> {
    let url = format!("{}/repos/{REPO}/releases/latest", api_base());
    let mut resp = get(&url, "application/vnd.github+json")
        .map_err(|e| format!("couldn't reach GitHub ({e})"))?;
    let body = resp
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_latest(&body, current_version())
}

/// Stream the asset into memory, reporting progress, and check size and digest.
fn download(rel: &Release, progress: &dyn Fn(f32)) -> Result<Vec<u8>, String> {
    let resp =
        get(&rel.url, "application/octet-stream").map_err(|e| format!("download failed ({e})"))?;
    let total = resp.body().content_length().unwrap_or(rel.size).max(1);
    let mut reader = resp.into_body().into_reader();
    let mut data = Vec::with_capacity(total as usize);
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download interrupted ({e})"))?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        progress((data.len() as f32 / total as f32).min(1.0));
    }
    verify(&data, rel)?;
    Ok(data)
}

pub fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn verify(data: &[u8], rel: &Release) -> Result<(), String> {
    if rel.size > 0 && data.len() as u64 != rel.size {
        return Err(format!(
            "download was {} bytes, expected {}",
            data.len(),
            rel.size
        ));
    }
    if let Some(want) = &rel.sha256 {
        let got = sha256_hex(data);
        if &got != want {
            return Err("download failed its checksum; nothing was changed".to_string());
        }
    }
    if data.len() < 64 * 1024 {
        return Err("download is too small to be PikCraft".to_string());
    }
    Ok(())
}

/// Where the previous executable is parked during a swap (deleted on the next start).
pub fn old_path(exe: &Path) -> PathBuf {
    let mut name = exe
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".old");
    exe.with_file_name(name)
}

/// Replace `exe` with `new_bin`. A running executable can be renamed (but not overwritten)
/// on Windows, so the old one is moved aside first; on Unix the rename is atomic.
pub fn install_binary(exe: &Path, new_bin: &[u8]) -> Result<(), String> {
    let dir = exe.parent().ok_or("can't find the game folder")?;
    let tmp = dir.join(".pikcraft-update.tmp");
    let explain = |e: std::io::Error| {
        format!(
            "can't write to {} ({e}). Download the update from {RELEASES_PAGE}",
            dir.display()
        )
    };
    std::fs::write(&tmp, new_bin).map_err(explain)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755)).map_err(explain)?;
    }
    let old = old_path(exe);
    let _ = std::fs::remove_file(&old);
    if cfg!(windows) {
        if let Err(e) = std::fs::rename(exe, &old) {
            let _ = std::fs::remove_file(&tmp);
            return Err(explain(e));
        }
    }
    if let Err(e) = std::fs::rename(&tmp, exe) {
        // Put the original back so the game still starts.
        if cfg!(windows) {
            let _ = std::fs::rename(&old, exe);
        }
        let _ = std::fs::remove_file(&tmp);
        return Err(explain(e));
    }
    Ok(())
}

static EXE: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Path of the game executable, captured before any update. (After a swap, Linux reports the
/// running image as "<path> (deleted)", so it can't be asked again later.)
fn exe_path() -> Option<PathBuf> {
    EXE.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .map(|p| p.canonicalize().unwrap_or(p))
    })
    .clone()
}

/// Call at startup: remembers where the executable lives and removes the copy a previous
/// update left behind.
pub fn cleanup_previous() {
    let Some(exe) = exe_path() else { return };
    let old = old_path(&exe);
    if !old.exists() {
        return;
    }
    // Right after a restart the previous process may still hold the file for a moment.
    std::thread::spawn(move || {
        for _ in 0..20 {
            if std::fs::remove_file(&old).is_ok() || !old.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
    });
}

/// Start the freshly installed game with the same arguments, then exit this process.
pub fn relaunch() -> ! {
    if let Some(exe) = exe_path() {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let _ = std::process::Command::new(exe).args(args).spawn();
    }
    std::process::exit(0);
}

#[derive(Clone, Debug, PartialEq)]
pub enum UpdateState {
    Off,
    Checking,
    UpToDate,
    Available(Release),
    Downloading { tag: String, progress: f32 },
    Installed(String),
    Failed(String),
}

enum Msg {
    Checked(Result<Option<Release>, String>),
    Progress(f32),
    Done(Result<(), String>),
}

pub struct Updater {
    pub state: UpdateState,
    rx: Option<Receiver<Msg>>,
}

impl Updater {
    pub fn disabled() -> Updater {
        Updater {
            state: UpdateState::Off,
            rx: None,
        }
    }

    /// Ask GitHub for the latest release in the background.
    pub fn check() -> Updater {
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Checked(fetch_latest()));
        });
        Updater {
            state: UpdateState::Checking,
            rx: Some(rx),
        }
    }

    /// Download and install the available release in the background.
    pub fn install(&mut self) {
        let UpdateState::Available(rel) = self.state.clone() else {
            return;
        };
        let (tx, rx) = channel();
        self.state = UpdateState::Downloading {
            tag: rel.tag.clone(),
            progress: 0.0,
        };
        self.rx = Some(rx);
        std::thread::spawn(move || {
            let progress_tx = tx.clone();
            let result = download(&rel, &move |p| {
                let _ = progress_tx.send(Msg::Progress(p));
            })
            .and_then(|data| {
                let exe = exe_path().ok_or("can't find the game executable")?;
                install_binary(&exe, &data)
            });
            let _ = tx.send(Msg::Done(result));
        });
    }

    /// Apply finished background work. Call once per frame.
    pub fn poll(&mut self) {
        let Some(rx) = &self.rx else { return };
        while let Ok(msg) = rx.try_recv() {
            self.state = match (msg, &self.state) {
                (Msg::Checked(Ok(Some(rel))), _) => UpdateState::Available(rel),
                (Msg::Checked(Ok(None)), _) => UpdateState::UpToDate,
                (Msg::Checked(Err(e)), _) | (Msg::Done(Err(e)), _) => UpdateState::Failed(e),
                (Msg::Progress(p), UpdateState::Downloading { tag, .. }) => {
                    UpdateState::Downloading {
                        tag: tag.clone(),
                        progress: p,
                    }
                }
                (Msg::Progress(_), s) => s.clone(),
                (Msg::Done(Ok(())), UpdateState::Downloading { tag, .. }) => {
                    UpdateState::Installed(tag.clone())
                }
                (Msg::Done(Ok(())), s) => s.clone(),
            };
        }
    }

    /// One-line status for menus, or `None` when there's nothing worth showing.
    pub fn status_line(&self) -> Option<String> {
        match &self.state {
            UpdateState::Off | UpdateState::Checking => None,
            UpdateState::UpToDate => Some(format!(
                "PikCraft v{} is up to date",
                env!("CARGO_PKG_VERSION")
            )),
            UpdateState::Available(r) => Some(format!(
                "Update {} is available - press [U] to install it",
                r.tag
            )),
            UpdateState::Downloading { tag, progress } => {
                Some(format!("Downloading {tag}... {:.0}%", progress * 100.0))
            }
            UpdateState::Installed(tag) => Some(format!("{tag} installed - press [U] to restart")),
            UpdateState::Failed(e) => Some(format!("Update failed: {e}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_json(tag: &str, asset: &str, digest: &str, size: u64) -> String {
        format!(
            r#"{{"tag_name":"{tag}","draft":false,"prerelease":false,"assets":[
                {{"name":"other.zip","browser_download_url":"https://x/other.zip","size":1}},
                {{"name":"{asset}","browser_download_url":"https://x/{asset}","size":{size},"digest":"sha256:{digest}"}}]}}"#
        )
    }

    #[test]
    fn versions_parse_and_compare() {
        assert_eq!(parse_version("v1.2.3"), Some((1, 2, 3)));
        assert_eq!(parse_version("1.10"), Some((1, 10, 0)));
        assert_eq!(parse_version("v2.0.0-beta.1"), Some((2, 0, 0)));
        assert_eq!(parse_version("nightly"), None);
        assert!(parse_version("v1.10.0") > parse_version("v1.9.9"));
        assert_eq!(
            current_version(),
            parse_version(env!("CARGO_PKG_VERSION")).unwrap()
        );
    }

    #[test]
    fn picks_newer_release_with_this_platforms_asset() {
        let json = release_json("v9.0.0", ASSET_NAME, "ABCD", 123);
        let rel = parse_latest(&json, (1, 2, 0)).unwrap().unwrap();
        assert_eq!(rel.version, (9, 0, 0));
        assert_eq!(rel.url, format!("https://x/{ASSET_NAME}"));
        assert_eq!(rel.sha256.as_deref(), Some("abcd"));
        assert_eq!(rel.size, 123);
        // Same or older: nothing to do.
        assert_eq!(
            parse_latest(&release_json("v1.2.0", ASSET_NAME, "a", 1), (1, 2, 0)).unwrap(),
            None
        );
        assert_eq!(
            parse_latest(&release_json("v1.1.9", ASSET_NAME, "a", 1), (1, 2, 0)).unwrap(),
            None
        );
        // Newer but missing our download: report it rather than installing the wrong thing.
        assert!(
            parse_latest(&release_json("v9.0.0", "something-else", "a", 1), (1, 2, 0)).is_err()
        );
    }

    #[test]
    fn verification_rejects_tampered_downloads() {
        let data = vec![7u8; 100_000];
        let mut rel = Release {
            tag: "v9".into(),
            version: (9, 0, 0),
            url: String::new(),
            size: data.len() as u64,
            sha256: Some(sha256_hex(&data)),
        };
        assert!(verify(&data, &rel).is_ok());
        let mut bad = data.clone();
        bad[500] ^= 1;
        assert!(verify(&bad, &rel).is_err());
        rel.size += 1;
        assert!(verify(&data, &rel).is_err());
    }

    #[test]
    fn install_swaps_the_executable_in_place() {
        let dir = std::env::temp_dir().join(format!("pikcraft-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join(if cfg!(windows) {
            "PikCraft.exe"
        } else {
            "pikcraft"
        });
        std::fs::write(&exe, b"old build").unwrap();
        install_binary(&exe, b"new build").unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), b"new build");
        assert!(!dir.join(".pikcraft-update.tmp").exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&exe).unwrap().permissions().mode() & 0o111,
                0o111
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
