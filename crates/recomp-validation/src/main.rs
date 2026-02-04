use clap::{Parser, Subcommand};
use recomp_validation::{
    run_baseline, run_video_validation, write_report, BaselinePaths, VideoValidationPaths,
};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    about = "Run baseline validation suite and emit regression reports",
    version
)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
    #[arg(long)]
    out_dir: Option<PathBuf>,
    #[arg(long)]
    repo_root: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    Baseline {
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long)]
        repo_root: Option<PathBuf>,
    },
    Video {
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long)]
        reference_config: PathBuf,
        #[arg(long)]
        test_video: Option<PathBuf>,
        #[arg(long)]
        summary: Option<PathBuf>,
        #[arg(long)]
        scripts_dir: Option<PathBuf>,
        #[arg(long)]
        thresholds: Option<PathBuf>,
        #[arg(long)]
        event_observations: Option<PathBuf>,
        #[arg(long, default_value_t = false)]
        strict: bool,
        #[arg(long)]
        python: Option<PathBuf>,
    },
}

fn main() {
    let args = Args::parse();
    let (report, out_dir) = match args.command {
        Some(Command::Baseline { out_dir, repo_root }) => {
            let repo_root = repo_root.unwrap_or_else(default_repo_root);
            let report = run_baseline(BaselinePaths {
                repo_root,
                out_dir: out_dir.clone(),
            });
            (report, out_dir)
        }
        Some(Command::Video {
            out_dir,
            reference_config,
            test_video,
            summary,
            scripts_dir,
            thresholds,
            event_observations,
            strict,
            python,
        }) => {
            let report = run_video_validation(VideoValidationPaths {
                reference_config,
                test_video,
                summary_path: summary,
                out_dir: out_dir.clone(),
                scripts_dir,
                thresholds_path: thresholds,
                event_observations,
                strict,
                python,
            })
            .unwrap_or_else(|err| {
                eprintln!("video validation failed: {err}");
                std::process::exit(1);
            });
            (report, out_dir)
        }
        None => {
            let out_dir = args.out_dir.unwrap_or_else(|| {
                eprintln!("--out-dir is required when no subcommand is provided");
                std::process::exit(2);
            });
            let repo_root = args.repo_root.unwrap_or_else(default_repo_root);
            let report = run_baseline(BaselinePaths {
                repo_root,
                out_dir: out_dir.clone(),
            });
            (report, out_dir)
        }
    };

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

fn default_repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .unwrap_or(&manifest_dir)
        .to_path_buf()
}
