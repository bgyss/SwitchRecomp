use recomp_validation::{run_video_validation, write_hash_list, Timecode, ValidationStatus};
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
