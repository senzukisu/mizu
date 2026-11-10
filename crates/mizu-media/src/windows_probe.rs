use crate::{CodecSupport, GpuInfo, SystemMediaReport};
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE, IDXGIFactory1};
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::CoTaskMemFree;
use windows_core::{GUID, PWSTR};

struct Target {
    codec: &'static str,
    kind: &'static str,
    major: GUID,
    subtype: GUID,
}

fn targets() -> Vec<Target> {
    let v = |codec, subtype| Target { codec, kind: "video", major: MFMediaType_Video, subtype };
    let a = |codec, subtype| Target { codec, kind: "audio", major: MFMediaType_Audio, subtype };
    vec![
        v("H.264 / AVC", MFVideoFormat_H264),
        v("H.265 / HEVC", MFVideoFormat_HEVC),
        v("AV1", MFVideoFormat_AV1),
        v("VP9", MFVideoFormat_VP90),
        v("VP8", MFVideoFormat_VP80),
        v("MPEG-4 Part 2", MFVideoFormat_MP4V),
        v("MPEG-2", MFVideoFormat_MPEG2),
        v("Theora", GUID::from_values(0x6F_65_68_74, 0, 0x0010, [0x80, 0x00, 0x00, 0xAA, 0x00, 0x38, 0x9B, 0x71])),
        a("AAC", MFAudioFormat_AAC),
        a("MP3", MFAudioFormat_MP3),
        a("Opus", MFAudioFormat_Opus),
        a("FLAC", MFAudioFormat_FLAC),
        a("ALAC", MFAudioFormat_ALAC),
        a("AC-3", MFAudioFormat_Dolby_AC3),
        a("E-AC-3", MFAudioFormat_Dolby_DDPlus),
    ]
}

fn enum_decoders(t: &Target, hardware: bool) -> Vec<String> {
    let category = if t.kind == "video" { MFT_CATEGORY_VIDEO_DECODER } else { MFT_CATEGORY_AUDIO_DECODER };
    let input = MFT_REGISTER_TYPE_INFO { guidMajorType: t.major, guidSubtype: t.subtype };
    let flags = if hardware {
        MFT_ENUM_FLAG_HARDWARE | MFT_ENUM_FLAG_SORTANDFILTER
    } else {
        MFT_ENUM_FLAG_SYNCMFT | MFT_ENUM_FLAG_ASYNCMFT | MFT_ENUM_FLAG_SORTANDFILTER
    };
    let mut activates: *mut Option<IMFActivate> = std::ptr::null_mut();
    let mut count = 0u32;
    // SAFETY: out-pointers are valid; on success we own `count` activates in a CoTaskMem array.
    unsafe {
        if MFTEnumEx(category, flags, Some(&input), None, &mut activates, &mut count).is_err() || activates.is_null() {
            return Vec::new();
        }
        let slice = std::slice::from_raw_parts_mut(activates, count as usize);
        let mut names = Vec::new();
        for slot in slice.iter_mut() {
            if let Some(act) = slot.take() {
                let mut name = PWSTR::null();
                let mut len = 0u32;
                if act.GetAllocatedString(&MFT_FRIENDLY_NAME_Attribute, &mut name, &mut len).is_ok() {
                    names.push(name.to_string().unwrap_or_default());
                    CoTaskMemFree(Some(name.0 as *const _));
                }
            }
        }
        CoTaskMemFree(Some(activates as *const _));
        names.sort();
        names.dedup();
        names
    }
}

fn gpus() -> Vec<GpuInfo> {
    let mut out = Vec::new();
    // SAFETY: standard DXGI enumeration.
    unsafe {
        let Ok(factory) = CreateDXGIFactory1::<IDXGIFactory1>() else { return out };
        let mut i = 0;
        while let Ok(adapter) = factory.EnumAdapters1(i) {
            i += 1;
            let Ok(desc) = adapter.GetDesc1() else { continue };
            if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
                continue;
            }
            let end = desc.Description.iter().position(|&c| c == 0).unwrap_or(desc.Description.len());
            out.push(GpuInfo {
                name: String::from_utf16_lossy(&desc.Description[..end]),
                dedicated_video_memory_mb: (desc.DedicatedVideoMemory / (1024 * 1024)) as u64,
                vendor_id: desc.VendorId,
            });
        }
    }
    out
}

pub fn probe() -> SystemMediaReport {
    // SAFETY: MFStartup/MFShutdown are balanced within this function.
    let started = unsafe { MFStartup(MF_VERSION, MFSTARTUP_LITE) };
    if let Err(e) = started {
        return SystemMediaReport {
            error: Some(format!("Media Foundation unavailable: {e}")),
            gpus: gpus(),
            ..Default::default()
        };
    }
    let codecs = targets()
        .iter()
        .map(|t| CodecSupport {
            codec: t.codec,
            kind: t.kind,
            hardware: enum_decoders(t, true),
            software: enum_decoders(t, false),
        })
        .collect();
    // SAFETY: balanced with the successful MFStartup above.
    unsafe {
        let _ = MFShutdown();
    }
    SystemMediaReport { codecs, gpus: gpus(), error: None }
}
