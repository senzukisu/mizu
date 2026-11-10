//! Media diagnostics.
//!
//! Playback itself is done by the engine (Chromium's media stack). What a given page can play
//! depends on two layers, and this module reports the *system* one:
//!
//! 1. **Engine decoders** – built into the runtime: VP8, VP9, AV1 (dav1d), Opus, Vorbis, FLAC,
//!    MP3, PCM, and – because Edge WebView2 ships licensed proprietary codecs – H.264 and AAC.
//!    These are probed from JavaScript on `mizu://media` (`MediaSource.isTypeSupported`,
//!    `MediaCapabilities.decodingInfo`).
//! 2. **System decoders** – Windows Media Foundation transforms, which the engine uses for
//!    hardware-accelerated decoding (DXVA/D3D11) and for codecs the runtime does not bundle, most
//!    notably HEVC/H.265 (requires a GPU decoder plus the "HEVC Video Extensions") and AC-3/E-AC-3.
//!
//! No codec binaries are shipped with Mizu.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CodecSupport {
    pub codec: &'static str,
    pub kind: &'static str,
    /// Hardware decoder transforms registered on this system.
    pub hardware: Vec<String>,
    /// Software decoder transforms registered on this system.
    pub software: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GpuInfo {
    pub name: String,
    pub dedicated_video_memory_mb: u64,
    pub vendor_id: u32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SystemMediaReport {
    pub codecs: Vec<CodecSupport>,
    pub gpus: Vec<GpuInfo>,
    pub error: Option<String>,
}

#[cfg(windows)]
mod windows_probe;

/// Probes system decoders and GPUs. Takes a few milliseconds; call off the hot path.
pub fn probe_system() -> SystemMediaReport {
    #[cfg(windows)]
    {
        windows_probe::probe()
    }
    #[cfg(not(windows))]
    {
        SystemMediaReport {
            error: Some("system codec probing is only implemented on Windows".into()),
            ..Default::default()
        }
    }
}

#[cfg(all(test, windows))]
mod tests {
    #[test]
    fn probe_runs() {
        let report = super::probe_system();
        assert!(report.error.is_none(), "{:?}", report.error);
        assert!(!report.codecs.is_empty());
        // Every Windows 10/11 install ships a software H.264 decoder.
        let h264 = report.codecs.iter().find(|c| c.codec == "H.264 / AVC").unwrap();
        assert!(!h264.software.is_empty() || !h264.hardware.is_empty());
    }
}
