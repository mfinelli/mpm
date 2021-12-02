use clap::{App, AppSettings};

mod install;
mod package;
mod upgrade;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let version = format!("{}.{}.{}{}",
                     env!("CARGO_PKG_VERSION_MAJOR"),
                     env!("CARGO_PKG_VERSION_MINOR"),
                     env!("CARGO_PKG_VERSION_PATCH"),
                     option_env!("CARGO_PKG_VERSION_PRE").unwrap_or(""));

    let cli = App::new("mpm")
        .version(version.as_str())
        .author("Mario Finelli <mario@finel.li>")
        .about("mario's package manager")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(
            App::new("install")
                .about("install a package")
        )
        .subcommand(
            App::new("package")
                .about("build a package")
        )
        .subcommand(
            App::new("upgrade")
                .aliases(&["up", ""])
                .about("upgrade all installed packages")
        )
        .get_matches();

    match cli.subcommand() {
        Some(("install", install_matches)) => install::run(),
        Some(("package", package_matches)) => package::run(),
        Some(("upgrade", upgrade_matches)) => upgrade::run(),
        _ => unreachable!(),
    }
}

fn main() {
    std::process::exit(match run() {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("error: {:?}", err);
            1
        }
    });
}
