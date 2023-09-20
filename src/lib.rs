use anyhow::Result;
use std::{fs::File, path::Path};
use ureq::Agent;

pub fn download_dump(agent: &Agent, output_file: impl AsRef<Path>) -> Result<()> {
    let mut res = agent
        .get("https://www.nationstates.net/pages/regions.xml.gz")
        .call()?
        .into_reader();

    std::io::copy(&mut res, &mut File::create(output_file)?)?;

    Ok(())
}
