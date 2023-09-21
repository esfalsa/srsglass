use anyhow::Result;
use flate2::read::GzDecoder;
use quick_xml::{events::Event, Reader};
use rust_xlsxwriter::Workbook;
use std::{fs::File, io::BufReader, path::Path};
use ureq::Agent;

#[derive(Default, Debug)]
pub struct Region {
    pub name: Option<String>,
    factbook: Option<String>,
    pub population: Option<i32>,
    // delegate: Option<String>,
    pub delegate_votes: Option<i32>,
    pub delegate_exec: Option<bool>,
    // frontier: Option<bool>,
    // governor: Option<String>,
    pub last_major: Option<i64>,
    pub last_minor: Option<i64>,
    pub nations_before: Option<i32>,
    pub embassies: Vec<String>,
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
            Ok(Event::CData(e)) => {
                if let Some(b"FACTBOOK") = current_tag.as_deref() {
                    current_region.factbook = Some(e.escape()?.unescape()?.trim().to_string());
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

pub fn save_to_excel(
    regions: impl Iterator<Item = Region>,
    total_population: i32,
    output_file: impl AsRef<Path>,
    major_length: i32,
    minor_length: i32,
) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_row(
        0,
        0,
        [
            "Region",
            "Link",
            "Population",
            "Total Nations",
            "Minor",
            "Major",
            "Del. Votes",
            "Del. Endos",
            "WFE",
        ],
    )?;

    let mut row_index = 1;

    for region in regions {
        let Region {
            name: Some(name),
            population: Some(population),
            delegate_votes: Some(delegate_votes),
            factbook: Some(mut factbook),
            nations_before: Some(nations_before),
            ..
        } = region
        else {
            continue;
        };

        worksheet.write_string(row_index, 0, &name)?;

        let link = format!(
            "https://www.nationstates.net/region={}",
            name.to_lowercase().replace(' ', "_")
        );
        worksheet.write_url(row_index, 1, link.as_str())?;

        worksheet.write_number(row_index, 2, population)?;

        worksheet.write_number(row_index, 3, nations_before)?;

        let progress = nations_before as f32 / total_population as f32;

        let minor_duration = (progress * minor_length as f32).floor() as i32;
        let minor_h = minor_duration / 3600;
        let minor_m = (minor_duration / 60) % 60;
        let minor_s = minor_duration % 60;
        worksheet.write_string(
            row_index,
            4,
            format!("{minor_h}:{minor_m:0>2}:{minor_s:0>2}"),
        )?;

        let major_duration = (progress * major_length as f32).floor() as i32;
        let major_h = major_duration / 3600;
        let major_m = (major_duration / 60) % 60;
        let major_s = major_duration % 60;
        worksheet.write_string(
            row_index,
            5,
            format!("{major_h}:{major_m:0>2}:{major_s:0>2}"),
        )?;

        worksheet.write_number(row_index, 6, delegate_votes)?;

        worksheet.write_number(
            row_index,
            7,
            if delegate_votes == 0 {
                delegate_votes
            } else {
                delegate_votes - 1
            },
        )?;

        // truncate factbook entry to maximum string length supported by Excel
        // https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3
        factbook.truncate(32767);
        worksheet.write_string(row_index, 8, factbook)?;

        row_index += 1;
    }

    workbook.save(output_file)?;

    Ok(())
}
