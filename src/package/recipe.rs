use std::fs::File;
use std::os::unix::fs;
use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_yaml;

use super::downloader;

#[derive(Debug, Deserialize)]
pub struct PackageRecipe {
    pub name: String,
    pub version: String,
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
    pub prepare: Option<String>,
    pub build: Option<String>,
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

    pub fn package_basename(&self) -> String {
        if let Some(epoch) = self.epoch {
            format!("{}-{}:{}-{}", self.name, epoch, self.version, self.release)
        } else {
            format!("{}-{}-{}", self.name, self.version, self.release)
        }
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
                let dl = downloader::download_file(client, source.url.as_str(), ".", false).await;
            }
        }
    }

    pub fn verify_sources(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sources) = &self.sources {
            for source in sources.iter() {
                match &source.sha256sum {
                    Some(hash) => {
                        if downloader::file_sha256sum_matches(
                            &downloader::get_url_basename(&source.url).unwrap(),
                            &hash,
                        ) {
                            continue;
                        } else {
                            return Err("hash doesn't match")?;
                        }
                    }
                    None => continue,
                }
            }
        }

        // TODO: add support for pgp signatures

        Ok(())
    }

    pub fn symlink_sources(&self, dest: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sources) = &self.sources {
            for source in sources.iter() {
                let filename = downloader::get_url_basename(&source.url).unwrap();

                fs::symlink(
                    std::fs::canonicalize(&filename).unwrap(),
                    Path::new(dest).join(&filename),
                )
                .unwrap();
            }
        }

        Ok(())
    }

    pub fn extract_sources(&self, dest: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(sources) = &self.sources {
            for source in sources.iter() {
                let filename = downloader::get_url_basename(&source.url).unwrap();
                let mut source = File::open(Path::new(dest).join(&filename)).unwrap();
                // ? is ok here, because we want to "fail" silently on non-archive formats
                compress_tools::uncompress_archive(
                    &mut source,
                    Path::new(dest),
                    compress_tools::Ownership::Ignore,
                )?;
            }
        }

        Ok(())
    }
}

impl PackageRecipeSource {
    fn variable_substitution(&mut self, find: &str, replace: &str) {
        let search = format!("${{{}}}", find);
        self.url = str::replace(&self.url, search.as_str(), replace);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basename_with_epoch() {
        let recipe = PackageRecipe {
            name: String::from("testpkg"),
            version: String::from("1.2.3"),
            epoch: Some(1),
            release: 4,
            description: String::from("test"),
            url: None,
            arch: None,
            license: None,
            depends: None,
            makedepends: None,
            checkdepends: None,
            sources: None,
            prepare: None,
            build: None,
            check: None,
            packages: None,
        };
        assert_eq!(recipe.package_basename(), "testpkg-1:1.2.3-4");
    }

    #[test]
    fn test_basename_without_epoch() {
        let recipe = PackageRecipe {
            name: String::from("testpkg"),
            version: String::from("1.2.3"),
            epoch: None,
            release: 4,
            description: String::from("test"),
            url: None,
            arch: None,
            license: None,
            depends: None,
            makedepends: None,
            checkdepends: None,
            sources: None,
            prepare: None,
            build: None,
            check: None,
            packages: None,
        };
        assert_eq!(recipe.package_basename(), "testpkg-1.2.3-4");
    }
}
