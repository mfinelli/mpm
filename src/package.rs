use std::fs;
use std::path::Path;

use clap::ArgMatches;
use reqwest::Client;

pub mod recipe;

use super::downloader;
use recipe::PackageRecipe;

static SRCDIR_BASE: &str = "tmpsrc";
static PKGDIR_BASE: &str = "tmppkg";

pub async fn run(cli: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let recipe_file = match cli.value_of("recipe") {
        Some(file) => file,
        None => "pkgrecipe.yaml",
    };
    let recipe = PackageRecipe::from_file(recipe_file)?;

    // TODO: abort if the package is already built

    let client = Client::builder().build().unwrap();
    recipe.download_sources(&client).await;
    match recipe.verify_sources() {
        Ok(_) => (),
        Err(err) => return Err(err),
    }

    // cleanup any existing packaging artifacts
    let packaging_dirs = [SRCDIR_BASE, PKGDIR_BASE];
    for dir in packaging_dirs {
        if Path::new(dir).exists() {
            match fs::remove_dir_all(dir) {
                Ok(_) => (),
                Err(err) => return Err(Box::new(err)),
            }
        }
    }

    // setup packaging directories
    for dir in packaging_dirs {
        match fs::create_dir(dir) {
            Ok(_) => (),
            Err(err) => return Err(Box::new(err)),
        }
    }

    match recipe.symlink_sources(SRCDIR_BASE) {
        Ok(_) => (),
        Err(err) => return Err(err),
    }
    // TODO: extract source archives

    Ok(())
}
