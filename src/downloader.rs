use std::cmp::min;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use futures_util::StreamExt;
use hex;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use sha2::{Digest, Sha256};
use url::Url;

pub async fn download_file(
    client: &Client,
    url: &str,
    dest: &str,
    overwrite: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let filename = get_url_basename(url).unwrap();

    if Path::new(&filename).exists() && !overwrite {
        return Ok(());
    }

    let response = client.get(url).send().await?;

    let total_bytes = match response.content_length() {
        Some(tb) => tb,
        None => 0,
    };

    let pb = ProgressBar::new(total_bytes);

    if total_bytes == 0 {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg} [{spinner}] [{elapsed_precise}] {bytes_per_sec}"),
        );
    } else {
        pb.set_style(ProgressStyle::default_bar()
                     .template("{msg} [{bar}] [{elapsed_precise}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                     .progress_chars("# -"));
    }

    pb.set_message(format!("Downloading: {}", filename));

    // TODO: handle destination correctly
    let mut file = match File::create(filename) {
        Ok(f) => f,
        Err(err) => return Err(Box::new(err)),
    };

    let mut downloaded_bytes: u64 = 0;
    let mut stream = response.bytes_stream();

    while let Some(slice) = stream.next().await {
        let chunk = slice?;
        file.write(&chunk)?;
        if total_bytes == 0 {
            pb.inc(chunk.len() as u64);
        } else {
            downloaded_bytes = min(downloaded_bytes + (chunk.len() as u64), total_bytes);
            pb.set_position(downloaded_bytes);
        }
    }

    pb.finish_with_message(format!("Done"));

    Ok(())
}

pub fn file_sha256sum_matches(path: &str, expected: &str) -> bool {
    let mut file = File::open(path).unwrap();
    let mut sum = Sha256::new();
    std::io::copy(&mut file, &mut sum).unwrap();
    let result = sum.finalize();
    hex::encode(result) == expected
}

pub fn get_url_basename(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    match Url::parse(url) {
        Ok(parsed_url) => match Path::new(parsed_url.path()).file_name() {
            Some(basename) => Ok(basename.to_os_string().into_string().unwrap()),
            None => return Err("unable to parse filename from url")?,
        },
        Err(err) => return Err(format!("{}", err))?,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn good_parse_example() {
        assert_eq!(
            get_url_basename("https://example.com/dir/file.tar.gz").unwrap(),
            "file.tar.gz"
        );
    }

    #[test]
    fn test_known_hash() {
        assert_eq!(
            file_sha256sum_matches(
                "tests/fixtures/src.tar.gz",
                "b6492e004ca58d23bb38e9ea50dab9698edb49b759777143a9105fca58597125"
            ),
            true
        );
    }
}
