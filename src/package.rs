// pub mod stuff;
//
use indicatif::MultiProgress;
use reqwest::Client;

use super::downloader;

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    // stuff::stuff();

    // println!("{}", downloader::get_url_basename("https://test.com/thing.zip").unwrap());
    //
    let client = Client::builder().build()?;
    // let mb = MultiProgress::new();

    // match downloader::download_file(&client, "https://www.example.com/binary.tar.gz", "todo") {
    //     Ok(_) => return Ok(()),
    //     Err(err) => return Err(err),
    // }
    // let results = downloader::download_file(&client, "https://www.example.com/binary.tar.gz", "todo").await;
    let results = downloader::download_file(&client, "https://www.example.com/binary.tar.gz", "todo").await;
    // let results = downloader::download_file(&client, &mb, "https://www.example.com/binary.tar.gz", "todo").await;
    // let results = downloader::download_file(&client, &mb, "https://www.example.com/binary.tar.gz", "todo").await;

    // mb.join().unwrap();


    Ok(())
}
