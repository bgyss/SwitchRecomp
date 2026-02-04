use recomp_validation::{
    hash_audio_file, hash_frames_dir, run_video_validation, run_video_validation_with_config,
    write_hash_list, Timecode, ValidationStatus,
};
use sha2::{Digest, Sha256};
use std::fs;

#[test]
fn timecode_parses_hms_and_seconds() {
    let tc = Timecode::parse("01:02:03.500").expect("parse hms");
    assert!((tc.seconds - 3723.5).abs() < 0.001);
    let tc = Timecode::parse("90.25").expect("parse seconds");
    assert!((tc.seconds - 90.25).abs() < 0.001);
}

#[test]
fn video_validation_passes_with_offset() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ref_frames = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
    ];
    let capture_frames = vec![
        "x".to_string(),
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
    ];

    let ref_hash_path = temp.path().join("reference_frames.txt");
    let capture_hash_path = temp.path().join("capture_frames.txt");
    write_hash_list(&ref_hash_path, &ref_frames).expect("write ref hashes");
    write_hash_list(&capture_hash_path, &capture_frames).expect("write capture hashes");

    let reference_toml = format!(
        r#"[video]
path = "reference.mp4"
width = 1280
height = 720
fps = 30.0

[timeline]
start = "00:00:00.000"
end = "00:00:00.167"

[hashes.frames]
format = "list"
path = "{}"

[thresholds]
frame_match_ratio = 0.99
max_drift_frames = 1
max_dropped_frames = 1
"#,
        ref_hash_path.display()
    );
    let capture_toml = format!(
        r#"[video]
path = "capture.mp4"
width = 1280
height = 720
fps = 30.0

[hashes.frames]
format = "list"
path = "{}"
"#,
        capture_hash_path.display()
    );

    let reference_path = temp.path().join("reference_video.toml");
    let capture_path = temp.path().join("capture_video.toml");
    fs::write(&reference_path, reference_toml).expect("write reference config");
    fs::write(&capture_path, capture_toml).expect("write capture config");

    let report = run_video_validation(&reference_path, &capture_path).expect("run validation");
    assert_eq!(report.status, ValidationStatus::Passed);
    assert_eq!(report.frame_comparison.matched, 5);
    assert_eq!(report.frame_comparison.offset, 1);
    assert_eq!(report.drift.frame_offset, 1);
}

#[test]
fn video_validation_fails_on_low_match_ratio() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ref_frames = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let capture_frames = vec!["a".to_string(), "x".to_string(), "y".to_string()];

    let ref_hash_path = temp.path().join("reference_frames.txt");
    let capture_hash_path = temp.path().join("capture_frames.txt");
    write_hash_list(&ref_hash_path, &ref_frames).expect("write ref hashes");
    write_hash_list(&capture_hash_path, &capture_frames).expect("write capture hashes");

    let reference_toml = format!(
        r#"[video]
path = "reference.mp4"
width = 1280
height = 720
fps = 30.0

[timeline]
start = "0"
end = "0.100"

[hashes.frames]
format = "list"
path = "{}"

[thresholds]
frame_match_ratio = 0.9
max_drift_frames = 0
max_dropped_frames = 0
"#,
        ref_hash_path.display()
    );
    let capture_toml = format!(
        r#"[video]
path = "capture.mp4"
width = 1280
height = 720
fps = 30.0

[hashes.frames]
format = "list"
path = "{}"
"#,
        capture_hash_path.display()
    );

    let reference_path = temp.path().join("reference_video.toml");
    let capture_path = temp.path().join("capture_video.toml");
    fs::write(&reference_path, reference_toml).expect("write reference config");
    fs::write(&capture_path, capture_toml).expect("write capture config");

    let report = run_video_validation(&reference_path, &capture_path).expect("run validation");
    assert_eq!(report.status, ValidationStatus::Failed);
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.contains("frame match ratio")));
}

#[test]
fn hash_generation_matches_normalized_outputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let frames_dir = temp.path().join("frames");
    fs::create_dir_all(&frames_dir).expect("create frames dir");
    let frame_a = frames_dir.join("00000001.png");
    let frame_b = frames_dir.join("00000002.png");
    fs::write(&frame_a, b"frame-one").expect("write frame a");
    fs::write(&frame_b, b"frame-two").expect("write frame b");

    let frame_hashes = hash_frames_dir(&frames_dir).expect("hash frames");
    let expected_frames = vec![sha256_bytes(b"frame-one"), sha256_bytes(b"frame-two")];
    assert_eq!(frame_hashes, expected_frames);

    let audio_path = temp.path().join("audio.wav");
    let mut first = vec![0u8; 4096];
    first[0] = 1;
    let second = vec![2u8; 4096];
    let mut audio = Vec::new();
    audio.extend_from_slice(&first);
    audio.extend_from_slice(&second);
    fs::write(&audio_path, &audio).expect("write audio");

    let audio_hashes = hash_audio_file(&audio_path).expect("hash audio");
    let expected_audio = vec![sha256_bytes(&first), sha256_bytes(&second)];
    assert_eq!(audio_hashes, expected_audio);
}

#[test]
fn validation_override_config_applies_thresholds() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ref_frames = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let capture_frames = vec!["a".to_string(), "x".to_string(), "y".to_string()];

    let ref_hash_path = temp.path().join("reference_frames.txt");
    let capture_hash_path = temp.path().join("capture_frames.txt");
    write_hash_list(&ref_hash_path, &ref_frames).expect("write ref hashes");
    write_hash_list(&capture_hash_path, &capture_frames).expect("write capture hashes");

    let reference_toml = format!(
        r#"[video]
path = "reference.mp4"
width = 1280
height = 720
fps = 30.0

[timeline]
start = "0"
end = "0.100"

[hashes.frames]
format = "list"
path = "{}"

[thresholds]
frame_match_ratio = 0.95
max_drift_frames = 0
max_dropped_frames = 0
"#,
        ref_hash_path.display()
    );
    let capture_toml = format!(
        r#"[video]
path = "capture.mp4"
width = 1280
height = 720
fps = 30.0

[hashes.frames]
format = "list"
path = "{}"
"#,
        capture_hash_path.display()
    );
    let validation_toml = r#"schema_version = "1"
name = "override"
notes = "Relax thresholds"
require_audio = false

[thresholds]
frame_match_ratio = 0.0
max_drift_frames = 1
max_dropped_frames = 2
"#;

    let reference_path = temp.path().join("reference_video.toml");
    let capture_path = temp.path().join("capture_video.toml");
    let validation_path = temp.path().join("validation_config.toml");
    fs::write(&reference_path, reference_toml).expect("write reference config");
    fs::write(&capture_path, capture_toml).expect("write capture config");
    fs::write(&validation_path, validation_toml).expect("write validation config");

    let report =
        run_video_validation_with_config(&reference_path, &capture_path, Some(&validation_path))
            .expect("run validation");
    assert_eq!(report.status, ValidationStatus::Passed);
    assert_eq!(
        report.validation_config.schema_version.as_deref(),
        Some("1")
    );
    assert_eq!(report.validation_config.name.as_deref(), Some("override"));
    assert!((report.validation_config.thresholds.frame_match_ratio - 0.0).abs() < 0.0001);
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    format!("{:x}", digest)
}
