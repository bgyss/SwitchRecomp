use clap::{Parser, Subcommand};
use recomp_pipeline::bundle::{package_bundle, PackageOptions};
use recomp_pipeline::homebrew::{intake_homebrew, lift_homebrew, IntakeOptions, LiftOptions};
use recomp_pipeline::{run_pipeline, PipelineOptions};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(about = "Static recompilation pipeline driver", version)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Run(RunArgs),
    Package(PackageArgs),
    HomebrewIntake(HomebrewIntakeArgs),
    HomebrewLift(HomebrewLiftArgs),
}

#[derive(Parser, Debug)]
struct RunArgs {
    #[arg(long)]
    module: PathBuf,
    #[arg(long)]
    config: PathBuf,
    #[arg(long)]
    provenance: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long, default_value = "../crates/recomp-runtime")]
    runtime_path: PathBuf,
}

#[derive(Parser, Debug)]
struct PackageArgs {
    #[arg(long)]
    project_dir: PathBuf,
    #[arg(long)]
    provenance: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    assets_dir: Option<PathBuf>,
}

#[derive(Parser, Debug)]
struct HomebrewIntakeArgs {
    #[arg(long)]
    module: PathBuf,
    #[arg(long)]
    nso: Vec<PathBuf>,
    #[arg(long)]
    provenance: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Parser, Debug)]
struct HomebrewLiftArgs {
    #[arg(long)]
    module_json: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long, default_value = "entry")]
    entry: String,
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Run(run) => {
            let options = PipelineOptions {
                module_path: run.module,
                config_path: run.config,
                provenance_path: run.provenance,
                out_dir: run.out_dir,
                runtime_path: run.runtime_path,
            };

            match run_pipeline(options) {
                Ok(report) => {
                    println!(
                        "Wrote {} files to {}",
                        report.files_written.len(),
                        report.out_dir.display()
                    );
                    for input in report.detected_inputs {
                        println!(
                            "Input {} format={} sha256={} size={}",
                            input.path.display(),
                            input.format.as_str(),
                            input.sha256,
                            input.size
                        );
                    }
                }
                Err(err) => {
                    eprintln!("Pipeline error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Command::Package(package) => {
            let options = PackageOptions {
                project_dir: package.project_dir,
                provenance_path: package.provenance,
                out_dir: package.out_dir,
                assets_dir: package.assets_dir,
            };
            match package_bundle(options) {
                Ok(report) => {
                    println!(
                        "Packaged bundle at {} ({} files)",
                        report.out_dir.display(),
                        report.files_written.len()
                    );
                    println!("Bundle manifest: {}", report.manifest_path.display());
                }
                Err(err) => {
                    eprintln!("Packaging error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Command::HomebrewIntake(intake) => {
            let options = IntakeOptions {
                module_path: intake.module,
                nso_paths: intake.nso,
                provenance_path: intake.provenance,
                out_dir: intake.out_dir,
            };
            match intake_homebrew(options) {
                Ok(report) => {
                    println!(
                        "Homebrew intake wrote {} files to {}",
                        report.files_written.len(),
                        report.out_dir.display()
                    );
                    println!("module.json: {}", report.module_json_path.display());
                    println!("manifest.json: {}", report.manifest_path.display());
                }
                Err(err) => {
                    eprintln!("Homebrew intake error: {err}");
                    std::process::exit(1);
                }
            }
        }
        Command::HomebrewLift(lift) => {
            let options = LiftOptions {
                module_json_path: lift.module_json,
                out_dir: lift.out_dir,
                entry_name: lift.entry,
            };
            match lift_homebrew(options) {
                Ok(report) => {
                    println!(
                        "Homebrew lift wrote {} functions to {}",
                        report.functions_emitted,
                        report.module_json_path.display()
                    );
                    if !report.warnings.is_empty() {
                        println!("Warnings:");
                        for warning in report.warnings {
                            println!("- {}", warning);
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Homebrew lift error: {err}");
                    std::process::exit(1);
                }
            }
        }
    }
}
