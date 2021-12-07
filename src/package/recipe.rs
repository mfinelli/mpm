use std::fs::File;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_yaml;

use super::downloader;

#[derive(Debug, Deserialize)]
pub struct PackageRecipe {
    name: String,
    version: String,
    epoch: Option<u32>,
    release: u32,
    description: String,
    url: Option<String>,
    arch: Option<Vec<String>>,
    license: Option<Vec<String>>,
    depends: Option<Vec<String>>,
    makedepends: Option<Vec<String>>,
    checkdepends: Option<Vec<String>>,
    sources: Option<Vec<PackageRecipeSource>>,
    prepare: Option<String>,
    build: Option<String>,
    check: Option<String>,
    packages: Option<Vec<PackageRecipePackage>>,
}

#[derive(Debug, Deserialize)]
struct PackageRecipeSource {
    url: String,
    sha256sum: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PackageRecipePackage {
    name: String,
    description: Option<String>,
    package: Option<String>,
}

impl PackageRecipe {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path).unwrap();
        let mut data: PackageRecipe = serde_yaml::from_reader(file).unwrap();

        data.variable_substitution();

        // println!("{:#?}", data);
        Ok(data)
    }

    fn variable_substitution(&mut self) {
        if let Some(ref mut sources) = self.sources {
            for source in sources.iter_mut() {
                source.variable_substitution("pkgname", &self.name);
                source.variable_substitution("pkgver", &self.version);
                if let Some(url) = &self.url {
                    source.variable_substitution("url", &url);
                }
            }
        }
    }

    pub async fn download_sources(&self, client: &Client) {
        if let Some(sources) = &self.sources {
            for source in sources.iter() {
                // TODO: error handling
                let dl = downloader::download_file(client, source.url.as_str(), ".").await;
            }
        }
    }

    pub fn verify_sources(self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sources) = self.sources {
            for source in sources.iter() {
                match &source.sha256sum {
                    Some(hash) => {
                        if downloader::file_sha256sum_matches(
                            &downloader::get_url_basename(&source.url).unwrap(),
                            &hash) {
                            continue;
                        } else {
                            return Err("hash doesn't match")?
                        }
                    },
                    None => continue,
                }
            }
        }

        // TODO: add support for pgp signatures

        Ok(())
    }
}

impl PackageRecipeSource {
    fn variable_substitution(&mut self, find: &str, replace: &str) {
        let search = format!("${{{}}}", find);
        self.url = str::replace(&self.url, search.as_str(), replace);
    }
}
