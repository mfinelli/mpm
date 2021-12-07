use clap::{App, AppSettings, Arg};

mod install;
mod package;
mod upgrade;

mod downloader;

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let version = format!(
        "{}.{}.{}{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH"),
        option_env!("CARGO_PKG_VERSION_PRE").unwrap_or("")
    );

    let cli = App::new("mpm")
        .version(version.as_str())
        .author("Mario Finelli <mario@finel.li>")
        .about("mario's package manager")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(App::new("install").about("install a package"))
        .subcommand(
            App::new("package").about("build a package").arg(
                Arg::new("recipe")
                    .short('r')
                    .long("recipe")
                    .about(concat!(
                        "Specify a custom recipe file ",
                        "(defaults to pkgrecipe.yml)"
                    ))
                    .required(false)
                    .multiple_occurrences(false)
                    .multiple_values(false)
                    .forbid_empty_values(true)
                    .takes_value(true)
                    .value_name("FILE")
                    .default_value("pkgrecipe.yaml"),
            ),
        )
        .subcommand(
            App::new("upgrade")
                .aliases(&["up", ""])
                .about("upgrade all installed packages"),
        )
        .get_matches();

    match cli.subcommand() {
        Some(("install", _install_matches)) => install::run(),
        Some(("package", package_matches)) => package::run(package_matches).await,
        Some(("upgrade", _upgrade_matches)) => upgrade::run(),
        _ => unreachable!(),
    }
}

#[tokio::main]
async fn main() {
    std::process::exit(match run().await {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    });
}
