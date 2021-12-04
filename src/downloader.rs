use std::path::Path;

use url::Url;

use std::cmp::min;
use std::io::{Cursor, Write};
use std::fs::File;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use futures_util::StreamExt;
use reqwest::Client;

// pub async fn download_file(client: &Client, mb: &MultiProgress, url: &str, dest: &str) -> Result<(), Box<dyn std::error::Error>> {
pub async fn download_file(client: &Client, url: &str, dest: &str) -> Result<(), Box<dyn std::error::Error>> {
    let response = client.get(url).send().await?;
    let filename = get_url_basename(url).unwrap();

    let total_bytes = match response.content_length() {
        Some(tb) => tb,
        None => 0,
    };

    // let pb = mb.add(ProgressBar::new(total_bytes));
    let pb = ProgressBar::new(total_bytes);

    if total_bytes == 0 {
        pb.set_style(ProgressStyle::default_spinner()
                     .template("{msg} [{spinner}] [{elapsed_precise}] {bytes_per_sec}"));
    } else {
        pb.set_style(ProgressStyle::default_bar()
                     .template("{msg} [{bar}] [{elapsed_precise}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
                     .progress_chars("# -"));
    }

    pb.set_message(format!("Downloading: {}", filename));

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
    };

    pb.finish_with_message(format!("Done"));

    Ok(())

     // match client.get(&url).send() {
     //     Ok(response) => {


 }


fn get_url_basename(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    match Url::parse(url) {
        Ok(parsed_url) => {
            match Path::new(parsed_url.path()).file_name() {
                Some(basename) => Ok(basename.to_os_string().into_string().unwrap()),
                None => return Err("unable to parse filename from url")?,
            }
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
}
