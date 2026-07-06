/*const artifact_entry = t.Object({
    name: t.String(),
    digest: t.String({ maxLength: 71, minLength: 71 }),
    url: t.String({ format: 'uri' }),
    size: t.Number(),
})

const releaseResponse = t.Object({
    name: t.String(),
    tag: semver,
    changelog: t.String(),
    artifacts: t.Object({
        macos: t.Object({
            x86_64: artifact_entry,
            aarch64: artifact_entry,
        }),
        windows: t.Object({
            baseline: artifact_entry,
            modern: artifact_entry,
        }),
        linux: t.Object({
            baseline: artifact_entry,
            modern: artifact_entry,
        }),
    }),
})*/

use crate::constants::API_URL;
use crate::download::DownloadArtifactError::Write;
use crate::download::FetchReleaseError::Deserialization;
use crate::download::FetchReleaseError::Fetch;
use futures::StreamExt;
use log::error;
use log::info;
use log::trace;
use reqwest::StatusCode;
use serde::Deserialize;
use sha2::{Digest, Sha256, digest::Output};
use slint::ToSharedString;
use slint::Weak;
use std::{
    fmt::{self},
    fs::{self},
    io::{self, BufRead, BufReader},
    path::Path,
};
use tokio::io::AsyncWriteExt;

use crate::AppWindow;
use crate::download::DownloadArtifactError::IO;
use crate::download::DownloadArtifactError::Request;
use crate::download::FetchReleaseError::Non200Code;

#[derive(Deserialize, Debug)]
pub struct ArtifactEntry {
    pub name: String,
    pub digest: String,
    pub url: String,
    pub size: u64,
}

#[derive(Deserialize, Debug)]
struct MACOSArtifacts {
    x86_64: ArtifactEntry,
    aarch64: ArtifactEntry,
}
#[derive(Deserialize, Debug)]
struct StandardArtifacts {
    modern: ArtifactEntry,
    baseline: ArtifactEntry,
}

#[derive(Deserialize, Debug)]
struct Artifacts {
    macos: MACOSArtifacts,
    linux: StandardArtifacts,
    windows: StandardArtifacts,
}
#[derive(Deserialize, Debug)]
pub struct ReleaseResponse {
    name: String,
    tag: String,
    changelog: String,
    artifacts: Artifacts,
}

#[derive(Debug)]
pub enum FetchReleaseError {
    Fetch(reqwest::Error),
    Non200Code(StatusCode),
    Deserialization,
}

#[derive(Debug)]
pub enum DownloadArtifactError {
    IO(std::io::Error),
    Request(reqwest::Error),
    Write,
}

impl fmt::Display for DownloadArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IO(e) => write!(f, "I/O Error: {e}"),
            Request(e) => write!(f, "Network Error: {e}"),
            Write => write!(f, "Write error"),
        }
    }
}

impl fmt::Display for FetchReleaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fetch(e) => write!(f, "Network Error: {e}"),
            Non200Code(code) => write!(f, "Invalid status code: {code}"),
            Deserialization => write!(f, "Invalid response from server"),
        }
    }
}

pub async fn fetch_release(alpha: bool) -> Result<ReleaseResponse, FetchReleaseError> {
    let url = if alpha {
        format!("{API_URL}/version/alpha")
    } else {
        format!("{API_URL}/version/latest")
    };

    info!(target:"fetch_release", "fetching from {url}");

    let release_response = reqwest::get(url).await.map_err(|e| {
        error!(target:"fetch_release", "failed to fetch release: {e}");
        FetchReleaseError::Fetch(e)
    })?;

    if release_response.status() != StatusCode::OK {
        error!(target:"fetch_release", "got status: {}, expected: 200", release_response.status());
        return Err(FetchReleaseError::Non200Code(release_response.status()));
    }

    let release = release_response
        .json::<ReleaseResponse>()
        .await
        .map_err(|e| {
            error!(target:"fetch_release", "failed to deserialize response: {e}");
            FetchReleaseError::Deserialization
        })?;

    info!(target:"fetch_release", "{}, {release:?}", release.tag);

    Ok(release)
}

pub fn get_artifact_from_release(release: ReleaseResponse) -> Option<ArtifactEntry> {
    #[cfg(target_os = "macos")]
    {
        #[cfg(target_arch = "aarch64")]
        return Some(release.artifacts.macos.aarch64);
        #[cfg(target_arch = "x86_64")]
        return Some(release.artifacts.macos.x86_64);
    }
    #[cfg(any(target_arch = "x86_64"))]
    {
        cpufeatures::new!(cpuid_avx2, "avx2");
        let avx2_token: cpuid_avx2::InitToken = cpuid_avx2::init();
        if avx2_token.get() {
            #[cfg(target_os = "windows")]
            return Some(release.artifacts.windows.modern);
            #[cfg(target_os = "linux")]
            return Some(release.artifacts.linux.modern);
        } else {
            cpufeatures::new!(cpuid_aesni, "aes");
            let aesni_token: cpuid_aesni::InitToken = cpuid_aesni::init();
            if aesni_token.get() {
                #[cfg(target_os = "windows")]
                return Some(release.artifacts.windows.baseline);
                #[cfg(target_os = "linux")]
                return Some(release.artifacts.linux.baseline);
            }
        }
    }
    return None;
}

pub fn compute_file_hash(file: &Path) -> io::Result<Output<Sha256>> {
    let mut digest = Sha256::new();
    let file = fs::File::open(file)?;
    let mut buf = BufReader::with_capacity(1024 * 128, file);

    loop {
        let buffer = buf.fill_buf()?;
        let read_len = buffer.len();
        if read_len == 0 {
            break;
        }
        Digest::update(&mut digest, buffer);
        buf.consume(read_len);
    }
    Ok(digest.finalize())
}

pub async fn download_artifact(
    weak_handle: Weak<AppWindow>,
    url: String,
    out: &Path,
    size: u64,
) -> Result<(), DownloadArtifactError> {
    info!(target:"download_artifact", "downloading {} bytes from {} to {}", size, url, out.display());
    let req = reqwest::get(url).await.map_err(|e| {
        error!(target:"download_artifact", "reqwest error: {e}");
        Request(e)
    })?;
    let mut stream = req.bytes_stream();
    let mut file = tokio::fs::File::create(out).await.map_err(|e| IO(e))?;
    let mut current = 0u64;
    let mut last_percentage = 0u8;

    info!(target: "download_artifact", "download + write started");

    while let Some(item) = stream.next().await {
        match item {
            Ok(data) => {
                file.write_all(&data).await.map_err(|e| {
                    error!(target:"download_artifact", "i/o error: {e}");
                    IO(e)
                })?;
                let n_bytes = data.len();
                current += n_bytes as u64;
                let percentage = ((current * 100) / size) as u8;

                if percentage > last_percentage {
                    last_percentage = percentage;
                    let tmp = weak_handle.clone();
                    let _ = slint::invoke_from_event_loop(move || {
                        let _ = tmp.unwrap().set_status_text(
                            if last_percentage < 100 {
                                format!("{last_percentage}% downloaded")
                            } else {
                                "".to_string()
                            }
                            .to_shared_string(),
                        );
                    });
                }
                trace!(target: "download_artifact", "wrote {n_bytes} bytes, {percentage}% done");
            }
            Err(e) => {
                error!(target:"download_artifact","got error while writing {e}");
                return Err(Write);
            }
        }
    }
    info!(target:"download_artifact", "downloaded {current} bytes");
    Ok(())
}

#[test]
fn file_hash_succes() {
    let expected = "sha256:1fe3aff27c420629a096a0575caaacdfc24a485dbf9941430cee51886392bc54";
    let hash = &expected[7..];
    println!("expected hash: {hash}");
    let computed = compute_file_hash(Path::new("./tests/lilith-macos-2.1.0-alpha.4")).unwrap();
    let hash_bytes = hex::decode(hash).unwrap();

    assert_eq!(hash_bytes.as_slice(), computed.as_slice())
}
#[test]
fn file_hash_failure() {
    let expected = "sha256:1fe3aff27c420629a096a0575caaacdfc24a485dbf9941430cee51886392bc67";
    let hash = &expected[7..];
    println!("expected hash: {hash}");
    let computed = compute_file_hash(Path::new("./tests/lilith-macos-2.1.0-alpha.4")).unwrap();
    let hash_bytes = hex::decode(hash).unwrap();

    assert_ne!(hash_bytes.as_slice(), computed.as_slice())
}
#[test]
fn file_hash_unkown_file() {
    let expected = "sha256:1fe3aff27c420629a096a0575caaacdfc24a485dbf9941430cee51886392bc67";
    let hash = &expected[7..];
    println!("expected hash: {hash}");
    let computed = compute_file_hash(Path::new("./tests/lilith"));
    assert!(computed.is_err())
}
