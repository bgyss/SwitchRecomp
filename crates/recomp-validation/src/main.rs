use clap::Parser;
use recomp_validation::{run_baseline, write_report, BaselinePaths};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    about = "Run baseline validation suite and emit regression reports",
    version
)]
struct Args {
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    repo_root: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();
    let repo_root = args.repo_root.unwrap_or_else(default_repo_root);
    let report = run_baseline(BaselinePaths {
        repo_root,
        out_dir: args.out_dir.clone(),
    });
    if let Err(err) = write_report(&args.out_dir, &report) {
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
        args.out_dir.display()
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
