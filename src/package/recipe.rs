use std::env;
use std::fs::File;
use std::os::unix::fs;
use std::path::Path;

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_yaml;
use subprocess::{Exec, NullFile, Redirection};

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
    pub source: Option<String>,
    pub prepare: Option<String>,
    pub build: Option<String>,
    pub check: Option<String>,
    pub packages: Option<Vec<PackageRecipePackage>>,
}

#[derive(Debug, Deserialize)]
struct PackageRecipeSource {
    url: String,
    filename: Option<String>,
    sha256sum: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PackageRecipePackage {
    name: String,
    description: Option<String>,
    package: Option<String>,
}

impl PackageRecipe {
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let file = File::open(path).unwrap();
        let mut data: PackageRecipe = serde_yaml::from_reader(file).unwrap();

        data.variable_substitution();
        data.compute_filenames();

        // println!("{:#?}", data);
        Ok(data)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn version(&self) -> &String {
        &self.version
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

    fn compute_filenames(&mut self) {
        if let Some(ref mut sources) = self.sources {
            for source in sources.iter_mut() {
                source.filename = match &source.filename {
                    Some(f) => Some(f.to_string()),
                    None => Some(downloader::get_url_basename(&source.url).unwrap()),
                }
            }
        }
    }

    pub fn all_source_filenames(&self) -> Vec<&str> {
        let mut source_filenames: Vec<&str> = Vec::new();

        match &self.sources {
            Some(sources) => {
                for source in sources.iter() {
                    let filename = &source.filename.as_ref().unwrap().as_str();
                    source_filenames.push(filename);
                }
            }
            None => (),
        };

        source_filenames
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
                            &source.filename.as_ref().unwrap(),
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
                let filename = &source.filename.as_ref().unwrap();

                fs::symlink(
                    std::fs::canonicalize(&filename).unwrap(),
                    Path::new(dest).join(&filename),
                )
                .unwrap();
            }
        }

        Ok(())
    }

    pub fn extract_sources(&self, dest: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut extracted_sources = Vec::new();

        if let Some(sources) = &self.sources {
            for source in sources.iter() {
                let filename = &source.filename.as_ref().unwrap();
                if !is_archive(&filename) {
                    continue;
                }

                let mut source = File::open(Path::new(dest).join(&filename)).unwrap();

                match compress_tools::uncompress_archive(
                    &mut source,
                    Path::new(dest),
                    compress_tools::Ownership::Ignore,
                ) {
                    Ok(_) => extracted_sources.push(filename.to_string()),
                    Err(_) => return Err(format!("unable to extract {}", filename))?,
                };
            }
        }

        Ok(extracted_sources)
    }

    pub fn create_source_package(
        &self,
        srcdir: &str,
        recipe_file: &str,
        extracted_sources: Vec<String>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let cwd = env::current_dir().unwrap().display().to_string();
        let srcdir = Path::new(&cwd).join(srcdir);

        // TODO: put this in a tempdir to avoid cluttering srcdir
        std::fs::create_dir_all(
            srcdir
                .join("usr")
                .join("share")
                .join("src")
                .join(&self.name),
        )
        .unwrap();

        let srcdir = srcdir.to_str().unwrap().to_string();

        let mut compress = Exec::cmd("fakeroot")
            .arg("--")
            .arg("bsdtar")
            .arg("czf")
            .arg(format!("{}.src.tar.gz", &self.package_basename()))
            .arg("-s")
            .arg(format!("|\\./|usr/share/src/{}/|", &self.name))
            .arg("-C")
            .arg(&srcdir)
            .arg("usr")
            .arg("-C")
            .arg(&cwd)
            .arg(format!("./{}", recipe_file));

        let mut last_dir = &cwd;

        let mut entries = std::fs::read_dir(&srcdir)?
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()?;
        entries.sort();
        let all_sources = &self.all_source_filenames();

        for entry in entries.iter() {
            let entry = entry.strip_prefix(&srcdir).unwrap().to_str().unwrap();
            if entry == "usr" {
                continue;
            }

            if extracted_sources.contains(&entry.to_string()) {
                println!("skipping extracted source {}", &entry);
                // if we extracted the source we _don't_ want to include the
                // archive symlink
                continue;
            } else if all_sources.iter().any(|&s| s == entry) {
                println!("compressing in rootdir {}", &entry);
                // we didn't extract the source because it wasn't an archive
                // but we need to include the original, non-symlink from the
                // root directory
                if last_dir != &cwd {
                    compress = compress.arg("-C").arg(&cwd);
                    last_dir = &cwd;
                }
                compress = compress.arg(format!("./{}", &entry));
            } else {
                println!("all_sources: {:?}", &all_sources);
                println!("entry: {:?}", &entry);
                println!("compressing in srcdir {}", &entry);
                // this is _not_ in the source list explicitly which means it's
                // the results of extracting an archive
                if last_dir != &srcdir {
                    compress = compress.arg("-C").arg(&srcdir);
                    last_dir = &srcdir;
                }
                compress = compress.arg(format!("./{}", &entry));
            }
        }

        // println!("{:?}", &compress);
        let status = compress.join().unwrap();

        // TODO: remove temporary usr directory

        Ok(status.success())
    }
}

impl PackageRecipeSource {
    fn variable_substitution(&mut self, find: &str, replace: &str) {
        let search = format!("${{{}}}", find);
        self.url = str::replace(&self.url, search.as_str(), replace);

        match &mut self.filename {
            Some(f) => {
                self.filename = Some(str::replace(&f, search.as_str(), replace));
            }
            None => (),
        }
    }
}

impl PackageRecipePackage {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn package(&self) -> Option<&String> {
        self.package.as_ref()
    }

    pub fn create_package(&self) {
    }

    pub fn create_debug_package(&self) {
    }
}

fn is_archive(path: &str) -> bool {
    // compress_tools will extract even regular files into "data", even
    // attempting to list the files does the same, so we need to exec the real
    // bsdtar and have it attempt to list the files where it will complain if
    // not given a real archive. the `-q '*'` will match only the first file in
    // the archive which is much faster on large archives
    let check = Exec::cmd("bsdtar")
        .arg("tf")
        .arg(path)
        .arg("-q")
        .arg("*")
        .stderr(Redirection::Merge)
        .stdout(NullFile)
        .join()
        .unwrap();

    check.success()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_substitution() {
        let mut s = PackageRecipeSource {
            url: String::from("${url}/archive/${pkgname}-${pkgver}.tar.gz"),
            sha256sum: None,
            filename: Some(String::from("${pkgname}.tgz")),
        };

        s.variable_substitution("url", "https://example.com");
        s.variable_substitution("pkgname", "test");
        s.variable_substitution("pkgver", "1.0");

        assert_eq!(s.url, "https://example.com/archive/test-1.0.tar.gz");
        assert_eq!(s.filename.unwrap(), "test.tgz");
    }

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
            source: None,
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
            source: None,
            sources: None,
            prepare: None,
            build: None,
            check: None,
            packages: None,
        };
        assert_eq!(recipe.package_basename(), "testpkg-1.2.3-4");
    }
}
