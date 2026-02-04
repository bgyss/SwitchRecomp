use clap::{Parser, Subcommand, ValueEnum};
use recomp_pipeline::bundle::{package_bundle, PackageOptions};
use recomp_pipeline::homebrew::{
    intake_homebrew, lift_homebrew, IntakeOptions, LiftMode, LiftOptions,
};
use recomp_pipeline::xci::{
    intake_xci, IntakeOptions as XciIntakeOptions, ProgramSelection, ToolKind,
};
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
    XciIntake(XciIntakeArgs),
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
    #[arg(long, value_enum, default_value = "decode")]
    mode: HomebrewLiftMode,
}

#[derive(Parser, Debug)]
struct XciIntakeArgs {
    #[arg(long)]
    xci: PathBuf,
    #[arg(long)]
    keys: PathBuf,
    #[arg(long)]
    title_keys: Option<PathBuf>,
    #[arg(long)]
    provenance: PathBuf,
    #[arg(long)]
    out_dir: PathBuf,
    #[arg(long)]
    xci_tool: Option<PathBuf>,
    #[arg(long, value_enum, default_value = "auto")]
    xci_tool_kind: XciToolKind,
    #[arg(long, conflicts_with = "program_name")]
    program_title_id: Option<String>,
    #[arg(long)]
    program_name: Option<String>,
}

#[derive(ValueEnum, Debug, Clone)]
enum XciToolKind {
    Auto,
    Hactool,
    Hactoolnet,
}

#[derive(ValueEnum, Debug, Clone)]
enum HomebrewLiftMode {
    Stub,
    Decode,
}

impl From<HomebrewLiftMode> for LiftMode {
    fn from(value: HomebrewLiftMode) -> Self {
        match value {
            HomebrewLiftMode::Stub => LiftMode::Stub,
            HomebrewLiftMode::Decode => LiftMode::Decode,
        }
    }
}

impl From<XciToolKind> for ToolKind {
    fn from(value: XciToolKind) -> Self {
        match value {
            XciToolKind::Auto => ToolKind::Auto,
            XciToolKind::Hactool => ToolKind::Hactool,
            XciToolKind::Hactoolnet => ToolKind::HactoolNet,
        }
    }
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
                mode: lift.mode.into(),
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
        Command::XciIntake(intake) => {
            let program = if let Some(title_id) = intake.program_title_id {
                ProgramSelection::TitleId(title_id)
            } else if let Some(name) = intake.program_name {
                ProgramSelection::Name(name)
            } else {
                eprintln!("XCI intake requires --program-title-id or --program-name");
                std::process::exit(2);
            };

            let options = XciIntakeOptions {
                xci_path: intake.xci,
                keys_path: intake.keys,
                provenance_path: intake.provenance,
                out_dir: intake.out_dir,
                program,
                tool_path: intake.xci_tool,
                tool_kind: intake.xci_tool_kind.into(),
                title_keys_path: intake.title_keys,
            };
            match intake_xci(options) {
                Ok(report) => {
                    println!(
                        "XCI intake wrote {} files to {}",
                        report.files_written.len(),
                        report.out_dir.display()
                    );
                    println!("module.json: {}", report.module_json_path.display());
                    println!("manifest.json: {}", report.manifest_path.display());
                }
                Err(err) => {
                    eprintln!("XCI intake error: {err}");
                    std::process::exit(1);
                }
            }
        }
    }
}
