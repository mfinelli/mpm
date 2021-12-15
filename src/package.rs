use std::collections::HashMap;
use std::fs;
use std::path::Path;

use clap::ArgMatches;
use reqwest::Client;

pub mod bash;
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

    let extracted_sources = match recipe.extract_sources(SRCDIR_BASE) {
        Ok(sources) => sources,
        Err(err) => return Err(err),
    };

    let mut vars = HashMap::new();
    vars.insert("pkgname", recipe.name());
    vars.insert("pkgver", recipe.version());

    let mut vars_with_srcdir = vars.clone();
    let full_srcdir = std::fs::canonicalize(SRCDIR_BASE).unwrap().to_str().unwrap().to_string();
    vars_with_srcdir.insert("srcdir", &full_srcdir);

    if let Some(ref source) = recipe.source {
        let status = bash::run_script(SRCDIR_BASE, &source, &vars_with_srcdir);
        if !status {
            return Err("source failed")?;
        }
    }

    match recipe.create_source_package(SRCDIR_BASE, recipe_file, extracted_sources) {
        Ok(status) => {
            if ! status {
                return Err("failed to create source package")?;
            }

            ()
        },
        Err(err) => return Err(err),
    }

    if let Some(ref prepare) = recipe.prepare {
        let status = bash::run_script(SRCDIR_BASE, &prepare, &vars_with_srcdir);
        if !status {
            return Err("prepare failed")?;
        }
    }

    if let Some(ref build) = recipe.build {
        let status = bash::run_script(SRCDIR_BASE, &build, &vars_with_srcdir);
        if !status {
            return Err("build failed")?;
        }
    }

    if let Some(ref check) = recipe.check {
        let status = bash::run_script(SRCDIR_BASE, &check, &vars_with_srcdir);
        if !status {
            return Err("check failed")?;
        }
    }

    if let Some(ref packages) = recipe.packages {
        for package in packages.iter() {
            match fs::create_dir(Path::new(PKGDIR_BASE).join(package.name())) {
                Ok(_) => (),
                Err(err) => return Err(Box::new(err)),
            }

            let mut vars_with_pkgdir = vars_with_srcdir.clone();
            let full_pkgdir = std::fs::canonicalize(Path::new(PKGDIR_BASE).join(package.name())).unwrap().to_str().unwrap().to_string();
            vars_with_pkgdir.insert("pkgdir", &full_pkgdir);

            if let Some(p) = &package.package() {
                let status = bash::run_script(SRCDIR_BASE, p, &vars_with_pkgdir);

                if !status {
                    return Err(format!("package {} failed", package.name()))?;
                }
            }

            package.create_debug_package();
            package.create_package();
        }
    }

    Ok(())
}
