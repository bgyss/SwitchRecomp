use clap::Parser;
use recomp_pipeline::{run_pipeline, PipelineOptions};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Static recompilation pipeline driver", version)]
struct Args {
    #[arg(long)]
    module: PathBuf,
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long, default_value = "../crates/recomp-runtime")]
    runtime_path: PathBuf,
}

fn main() {
    let args = Args::parse();
    let options = PipelineOptions {
        module_path: args.module,
        config_path: args.config,
        out_dir: args.out_dir,
        runtime_path: args.runtime_path,
    };

    match run_pipeline(options) {
        Ok(report) => {
            println!("Wrote {} files to {}", report.files_written.len(), report.out_dir.display());
        }
        Err(err) => {
            eprintln!("Pipeline error: {err}");
            std::process::exit(1);
        }
    }
}
