use clap::{Args, Parser, Subcommand};
use recomp_validation::{
    hash_audio_file, hash_frames_dir, run_baseline, run_video_suite, write_hash_list, write_report,
    BaselinePaths,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    about = "Run baseline validation suite and emit regression reports",
    version
)]
struct Cli {
    #[arg(long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    repo_root: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Video(VideoArgs),
    HashFrames(HashFramesArgs),
    HashAudio(HashAudioArgs),
}

#[derive(Args, Debug)]
struct VideoArgs {
    #[arg(long)]
    reference: PathBuf,
    #[arg(long)]
    capture: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Args, Debug)]
struct HashFramesArgs {
    #[arg(long)]
    frames_dir: PathBuf,
    #[arg(long)]
    out: PathBuf,
}

#[derive(Args, Debug)]
struct HashAudioArgs {
    #[arg(long)]
    audio_file: PathBuf,
    #[arg(long)]
    out: PathBuf,
}

fn main() {
    let args = Cli::parse();
    match args.command {
        Some(Command::Video(cmd)) => {
            let report = run_video_suite(&cmd.reference, &cmd.capture);
            if let Err(err) = write_report(&cmd.out_dir, &report) {
                eprintln!("failed to write validation report: {err}");
                std::process::exit(1);
            }
            if report.failed > 0 {
                eprintln!("validation failed: {} cases failed", report.failed);
                std::process::exit(1);
            }
            println!(
                "validation passed: {} cases, report written to {}",
                report.total,
                cmd.out_dir.display()
            );
        }
        Some(Command::HashFrames(cmd)) => {
            let hashes = hash_frames_dir(&cmd.frames_dir).unwrap_or_else(|err| {
                eprintln!("failed to hash frames: {err}");
                std::process::exit(1);
            });
            write_hash_list(&cmd.out, &hashes).unwrap_or_else(|err| {
                eprintln!("failed to write hash list: {err}");
                std::process::exit(1);
            });
            println!(
                "frame hashes written: {} entries -> {}",
                hashes.len(),
                cmd.out.display()
            );
        }
        Some(Command::HashAudio(cmd)) => {
            let hashes = hash_audio_file(&cmd.audio_file).unwrap_or_else(|err| {
                eprintln!("failed to hash audio: {err}");
                std::process::exit(1);
            });
            write_hash_list(&cmd.out, &hashes).unwrap_or_else(|err| {
                eprintln!("failed to write hash list: {err}");
                std::process::exit(1);
            });
            println!(
                "audio hashes written: {} entries -> {}",
                hashes.len(),
                cmd.out.display()
            );
        }
        None => {
            let out_dir = args.out_dir.unwrap_or_else(|| {
                eprintln!("--out-dir is required unless using a subcommand");
                std::process::exit(2);
            });
            let repo_root = args.repo_root.unwrap_or_else(default_repo_root);
            let report = run_baseline(BaselinePaths {
                repo_root,
                out_dir: out_dir.clone(),
            });
            if let Err(err) = write_report(&out_dir, &report) {
                eprintln!("failed to write validation report: {err}");
                std::process::exit(1);
            }
            if report.failed > 0 {
                eprintln!("validation failed: {} cases failed", report.failed);
                std::process::exit(1);
            }
            println!(
                "validation passed: {} cases, report written to {}",
                report.total,
                out_dir.display()
            );
        }
    }
}

fn default_repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or(&manifest_dir)
        .to_path_buf()
}
