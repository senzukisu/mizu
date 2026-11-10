# Video, audio and codecs

Mizu plays media through the Chromium media pipeline of the WebView2 engine. **Mizu ships no codec binaries.**
`mizu://media` shows the real state of the machine it runs on.

## What is available

| Format | Source | Notes |
|---|---|---|
| H.264 / AVC | Engine (licensed by Microsoft in Edge WebView2) | Hardware decoding via D3D11/DXVA when the GPU supports it |
| AAC | Engine (licensed) | |
| VP8, VP9 | Engine | VP9 hardware decoding on most GPUs from ~2016 on |
| AV1 | Engine (dav1d) + system | Hardware on recent GPUs (e.g. RTX 30/40, Intel Xe, Radeon RX 6000+), software fallback otherwise |
| Opus, Vorbis, FLAC, MP3, PCM | Engine | |
| H.265 / HEVC | **System**: needs a GPU decoder **and** the "HEVC Video Extensions" from the Microsoft Store | Not available in software inside the engine |
| AC-3, E-AC-3 | System (Dolby decoders shipped with some Windows editions or OEM images) | Availability varies |
| ALAC | Engine/system support varies | |
| MPEG-4 Part 2, MPEG-2, Theora | Not supported by Chromium for web playback | Shown as "No" even if Windows has decoders |

Containers that websites can use: MP4, WebM, Ogg, WAV, FLAC, MPEG-TS (through Media Source Extensions with suitable
codecs). Matroska (MKV) works when it contains supported codecs; QuickTime and AVI generally do not play as files.

## Hardware acceleration

The engine decides between hardware and software decoding per stream. Mizu's job is to keep it enabled
(Settings → Performance → Hardware acceleration, on by default) and to make the result visible: `mizu://media` reports
the `powerEfficient` flag from the MediaCapabilities API, which corresponds to hardware decoding.

If video stutters or uses a lot of CPU:
1. Open `mizu://media` and check that the codec shows **Hardware**.
2. Update the GPU driver.
3. For HEVC, install the HEVC Video Extensions.
4. As a last resort, turn hardware acceleration off (some broken drivers do better without it) and restart.

Autoplay can be restricted in Settings → Media; that takes effect after a restart.

## Licensing

- H.264 and AAC are covered by Microsoft's licences for the Edge WebView2 runtime, which is installed and updated by
  Windows rather than distributed by Mizu.
- HEVC, AC-3 and E-AC-3 are used only through decoders already installed on the user's system.
- Mizu deliberately does not bundle FFmpeg or GStreamer. Doing so would require per-codec patent decisions, and it is
  unnecessary while the engine and the OS provide decoding. If Mizu adopts CEF (see the roadmap), codec licensing will
  be evaluated before shipping any proprietary codec.
