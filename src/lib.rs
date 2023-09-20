use anyhow::Result;
use flate2::read::GzDecoder;
use quick_xml::{events::Event, Reader};
use std::{fs::File, io::BufReader, path::Path};
use ureq::Agent;

#[derive(Default, Debug)]
pub struct Region {
    name: Option<String>,
    // factbook: Option<String>,
    population: Option<i32>,
    // delegate: Option<String>,
    delegate_votes: Option<i32>,
    delegate_exec: Option<bool>,
    // frontier: Option<bool>,
    // governor: Option<String>,
    last_major: Option<i64>,
    last_minor: Option<i64>,
    nations_before: Option<i32>,
    embassies: Vec<String>,
}

pub fn download_dump(agent: &Agent, output_file: impl AsRef<Path>) -> Result<()> {
    let mut res = agent
        .get("https://www.nationstates.net/pages/regions.xml.gz")
        .call()?
        .into_reader();

    std::io::copy(&mut res, &mut File::create(output_file)?)?;

    Ok(())
}

pub fn parse_dump(dump_path: impl AsRef<Path>) -> Result<Vec<Region>> {
    let gz = BufReader::new(GzDecoder::new(File::open(dump_path)?));
    let mut reader = Reader::from_reader(gz);
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut current_tag = None;
    let mut current_region = Region::default();

    let mut current_population = 0;

    let mut regions: Vec<Region> = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                current_tag = Some(e.to_owned());
            }
            Ok(Event::End(e)) => {
                if let Some(current_tag_name) = current_tag.as_deref() {
                    if e.name().as_ref() == current_tag_name {
                        current_tag = None;
                    }
                }

                if e.name().as_ref() == b"REGION" {
                    current_region.nations_before = Some(current_population);

                    if let Some(population) = current_region.population {
                        current_population += population;
                    }

                    regions.push(current_region);

                    current_region = Region::default();
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(tag) = current_tag.as_ref() {
                    match tag.name().as_ref() {
                        b"NAME" => current_region.name = Some(e.unescape()?.to_string()),
                        b"NUMNATIONS" => current_region.population = Some(e.unescape()?.parse()?),
                        b"DELEGATEVOTES" => {
                            current_region.delegate_votes = Some(e.unescape()?.parse()?);
                        }
                        b"DELEGATEAUTH" => {
                            current_region.delegate_exec = Some(e.unescape()?.contains('X'));
                        }
                        b"LASTMAJORUPDATE" => {
                            current_region.last_major = Some(e.unescape()?.parse()?);
                        }
                        b"LASTMINORUPDATE" => {
                            current_region.last_minor = Some(e.unescape()?.parse()?);
                        }
                        b"EMBASSY" => current_region.embassies.push(e.unescape()?.to_string()),
                        _ => (),
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            _ => (),
        }

        buf.clear();
    }

    Ok(regions)
}
