use anyhow::Result;
use clap::{Parser, Subcommand};
use simdock_core::{
    model::Platform,
    provider::{PlatformProvider, android::AndroidProvider, ios::IosProvider},
    service::SimdockService,
};
use simdock_infra::AppPaths;

#[derive(Debug, Parser)]
#[command(
    name = "simdock",
    about = "Manage iOS and Android simulator runtimes on macOS"
)]
struct Cli {
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Doctor,
    Runtime {
        #[command(subcommand)]
        command: RuntimeCommand,
    },
    Start {
        #[arg(long)]
        platform: Platform,
        profile: String,
    },
    Stop {
        #[arg(long)]
        platform: Platform,
        instance: String,
    },
}

#[derive(Debug, Subcommand)]
enum RuntimeCommand {
    List {
        #[arg(long)]
        platform: Option<Platform>,
    },
    Install {
        #[arg(long)]
        platform: Platform,
        #[arg(long)]
        version: String,
        #[arg(long)]
        device: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::detect()?;
    let ios = IosProvider::new();
    let android = AndroidProvider::with_avd_root(
        paths.android_sdk_root.clone(),
        paths.android_avd_root.clone(),
    );
    let service = SimdockService::new(ios.clone(), android.clone());

    match cli.command {
        Command::Doctor => {
            let reports = service.doctor_all().await?;
            print_output(cli.json, &reports)?;
        }
        Command::Runtime { command } => match command {
            RuntimeCommand::List { platform } => {
                let runtimes = match platform {
                    Some(Platform::Ios) => ios.list_runtimes().await?,
                    Some(Platform::Android) => android.list_runtimes().await?,
                    None => {
                        let mut combined = ios.list_runtimes().await?;
                        combined.extend(android.list_runtimes().await?);
                        combined
                    }
                };
                print_output(cli.json, &runtimes)?;
            }
            RuntimeCommand::Install {
                platform,
                version,
                device,
            } => {
                let request = simdock_core::InstallRequest {
                    platform,
                    runtime_version: version,
                    device_name: device,
                };

                let result = match platform {
                    Platform::Ios => ios.install_runtime(request, None).await,
                    Platform::Android => android.install_runtime(request, None).await,
                };

                result?;
            }
        },
        Command::Start { platform, profile } => {
            anyhow::bail!("start is not implemented yet for {platform:?} profile {profile}");
        }
        Command::Stop { platform, instance } => {
            anyhow::bail!("stop is not implemented yet for {platform:?} instance {instance}");
        }
    }

    Ok(())
}

fn print_output<T>(as_json: bool, value: &T) -> Result<()>
where
    T: serde::Serialize + std::fmt::Debug,
{
    if as_json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{value:#?}");
    }

    Ok(())
}
