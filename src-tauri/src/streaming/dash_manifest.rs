use crate::models::video::StreamInfo;
use crate::streaming::codec::VideoCodecFamily;
use std::collections::BTreeMap;
use tracing::info;

fn extract_codecs(mime_type: Option<&str>) -> Option<String> {
    let mime_type = mime_type?;
    let codecs = mime_type
        .split(';')
        .find_map(|part| {
            part.trim()
                .strip_prefix("codecs=")
                .map(|value| value.trim_matches('"').to_string())
        })
        .filter(|value| !value.is_empty())?;

    if codecs == "vp9" {
        Some("vp09.00.10.08".to_string())
    } else {
        Some(codecs)
    }
}

fn extract_base_mime_type(mime_type: Option<&str>) -> Option<&str> {
    mime_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn format_duration_xml(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = duration_ms % 1000;

    let mut output = String::from("PT");
    if hours > 0 {
        output.push_str(&format!("{hours}H"));
    }
    if minutes > 0 {
        output.push_str(&format!("{minutes}M"));
    }
    if millis > 0 {
        output.push_str(&format!("{seconds}.{millis:03}S"));
    } else {
        output.push_str(&format!("{seconds}S"));
    }
    output
}

fn escape_xml_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Build a DASH manifest for responses YouTube serves without one.
///
/// Public so `tests/dash_manifest.rs` can assert the audio contract the player
/// depends on; nothing outside this module calls it.
pub fn build_synthetic_dash_manifest(stream_info: &StreamInfo) -> Option<String> {
    let has_segment_ranges =
        |init_s: Option<u64>, init_e: Option<u64>, idx_s: Option<u64>, idx_e: Option<u64>| {
            init_s.is_some() && init_e.is_some() && idx_s.is_some() && idx_e.is_some()
        };

    let audio_tracks: Vec<_> = stream_info
        .audio_tracks
        .iter()
        .filter(|track| {
            !track.local_url.is_empty()
                && has_segment_ranges(
                    track.init_range_start,
                    track.init_range_end,
                    track.index_range_start,
                    track.index_range_end,
                )
        })
        .collect();

    if audio_tracks.is_empty() {
        return None;
    }

    let video_variants: Vec<_> = stream_info
        .variants
        .iter()
        .filter(|variant| {
            variant.is_video_only
                && variant.is_playable
                && !variant.local_url.is_empty()
                && has_segment_ranges(
                    variant.init_range_start,
                    variant.init_range_end,
                    variant.index_range_start,
                    variant.index_range_end,
                )
        })
        .collect();

    if video_variants.is_empty() {
        return None;
    }

    let duration_ms = video_variants
        .iter()
        .filter_map(|variant| variant.approx_duration_ms)
        .chain(
            audio_tracks
                .iter()
                .filter_map(|track| track.approx_duration_ms),
        )
        .max()?;

    // Log the codecs/resolutions that won selection so a "video frozen, audio
    // plays" report (a missing system decoder on Linux, issue #29) is diagnosable
    // from the logs alone — pairs with the caller's `video_id`.
    let selected_video: Vec<String> = video_variants
        .iter()
        .map(|variant| {
            format!(
                "{} {}p {}kbps",
                extract_codecs(variant.mime_type.as_deref()).unwrap_or_else(|| "?".to_string()),
                variant.height.unwrap_or(0),
                variant.bitrate.unwrap_or(0) / 1000
            )
        })
        .collect();
    let selected_audio: Vec<String> = audio_tracks
        .iter()
        .map(|track| extract_codecs(track.mime_type.as_deref()).unwrap_or_else(|| "?".to_string()))
        .collect();
    info!(
        video_reps = selected_video.len(),
        audio_reps = selected_audio.len(),
        video = ?selected_video,
        audio = ?selected_audio,
        "synthetic DASH representation selection"
    );

    // One AdaptationSet per codec family, H.264 first so `firstTrack` starts there, and HDR apart
    // from SDR. Nothing links the sets, so ABR stays inside the set it is playing: it can't hop
    // between containers or colour ranges mid-video. A manual pick crosses sets by representation
    // id, and Auto chooses its set in the player.
    let mut video_groups: BTreeMap<(VideoCodecFamily, bool, &str), Vec<&StreamVariant>> =
        BTreeMap::new();
    for variant in &video_variants {
        let mime = variant.mime_type.as_deref();
        let key = (
            VideoCodecFamily::from_mime(mime.unwrap_or("")),
            variant.quality_label.contains("HDR"),
            extract_base_mime_type(mime).unwrap_or("video/mp4"),
        );
        video_groups.entry(key).or_default().push(variant);
    }

    let mut manifest = String::new();
    manifest.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    manifest.push_str(&format!(
        "<MPD xmlns=\"urn:mpeg:dash:schema:mpd:2011\" type=\"static\" profiles=\"urn:mpeg:dash:profile:isoff-on-demand:2011\" minBufferTime=\"PT1.5S\" mediaPresentationDuration=\"{}\">\n",
        format_duration_xml(duration_ms)
    ));
    manifest.push_str("  <Period id=\"flow-period-0\" start=\"PT0S\">\n");
    for audio_track in audio_tracks.iter() {
        let audio_mime_type =
            extract_base_mime_type(audio_track.mime_type.as_deref()).unwrap_or("audio/mp4");
        let audio_codecs = extract_codecs(audio_track.mime_type.as_deref())
            .unwrap_or_else(|| "mp4a.40.2".to_string());
        let audio_id = escape_xml_attr(&audio_track.id);
        let lang = escape_xml_attr(audio_track.language_code.as_deref().unwrap_or("und"));
        let label = escape_xml_attr(&audio_track.label);
        let role = if audio_track.is_default {
            "main"
        } else {
            "alternate"
        };

        manifest.push_str(&format!(
            "    <AdaptationSet id=\"audio-{}\" contentType=\"audio\" lang=\"{}\" mimeType=\"{}\" codecs=\"{}\" startWithSAP=\"1\" subsegmentAlignment=\"true\">\n",
            audio_id,
            lang,
            audio_mime_type,
            audio_codecs
        ));
        manifest.push_str(&format!(
            "      <Role schemeIdUri=\"urn:mpeg:dash:role:2011\" value=\"{}\" />\n",
            role
        ));
        manifest.push_str(&format!("      <Label>{}</Label>\n", label));
        manifest.push_str(&format!(
            "      <Representation id=\"{}\" bandwidth=\"{}\" audioSamplingRate=\"48000\">\n",
            audio_id,
            audio_track.bitrate.unwrap_or(128_000)
        ));
        manifest.push_str(&format!(
            "        <BaseURL>{}</BaseURL>\n",
            escape_xml_attr(&audio_track.local_url)
        ));
        manifest.push_str(&format!(
            "        <SegmentBase indexRange=\"{}-{}\">\n",
            audio_track.index_range_start?, audio_track.index_range_end?
        ));
        manifest.push_str(&format!(
            "          <Initialization range=\"{}-{}\" />\n",
            audio_track.init_range_start?, audio_track.init_range_end?
        ));
        manifest.push_str("        </SegmentBase>\n");
        manifest.push_str("      </Representation>\n");
        manifest.push_str("    </AdaptationSet>\n");
    }
    for ((family, is_hdr, mime_type), variants) in &video_groups {
        let container = mime_type.rsplit('/').next().unwrap_or("mp4");
        let hdr = if *is_hdr { "-hdr" } else { "" };
        manifest.push_str(&format!(
            "    <AdaptationSet id=\"video-{}{hdr}-{container}\" contentType=\"video\" mimeType=\"{mime_type}\" startWithSAP=\"1\" subsegmentAlignment=\"true\">\n",
            family.slug(),
        ));

        for variant in variants {
            let codecs = extract_codecs(variant.mime_type.as_deref())
                .unwrap_or_else(|| "avc1.640028".to_string());
            manifest.push_str(&format!(
                "      <Representation id=\"{}\" bandwidth=\"{}\" width=\"{}\" height=\"{}\" frameRate=\"{}\" codecs=\"{}\">\n",
                escape_xml_attr(&variant.id),
                variant.bitrate.unwrap_or(1_000_000),
                variant.width.unwrap_or(0),
                variant.height.unwrap_or(0),
                variant.fps.unwrap_or(30),
                escape_xml_attr(&codecs)
            ));
            manifest.push_str(&format!(
                "        <BaseURL>{}</BaseURL>\n",
                escape_xml_attr(&variant.local_url)
            ));
            manifest.push_str(&format!(
                "        <SegmentBase indexRange=\"{}-{}\">\n",
                variant.index_range_start?, variant.index_range_end?
            ));
            manifest.push_str(&format!(
                "          <Initialization range=\"{}-{}\" />\n",
                variant.init_range_start?, variant.init_range_end?
            ));
            manifest.push_str("        </SegmentBase>\n");
            manifest.push_str("      </Representation>\n");
        }

        manifest.push_str("    </AdaptationSet>\n");
    }
    manifest.push_str("  </Period>\n");
    manifest.push_str("</MPD>\n");
    Some(manifest)
}
