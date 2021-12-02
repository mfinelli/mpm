pub mod stuff;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    stuff::stuff();
    Ok(())
}
