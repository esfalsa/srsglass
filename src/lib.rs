use anyhow::Result;
use flate2::read::GzDecoder;
use quick_xml::{events::Event, Reader};
use rust_xlsxwriter::{Color, ExcelDateTime, Format, Workbook};
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
    governorless: Vec<String>,
    passwordless: Vec<String>,
) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.set_column_width(0, 45)?;

    let red_fill = Format::new().set_background_color(Color::Red);
    let yellow_fill = Format::new().set_background_color(Color::Yellow);
    let green_fill = Format::new().set_background_color(Color::Lime);

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
            "Embassies",
            "WFE",
        ],
    )?;

    let duration_format = Format::new().set_num_format("[h]:mm:ss");
    worksheet.set_column_format(4, &duration_format)?;
    worksheet.set_column_format(5, &duration_format)?;

    let mut row_index = 1;

    for region in regions {
        let Region {
            name: Some(name),
            population: Some(population),
            delegate_votes: Some(delegate_votes),
            factbook: Some(mut factbook),
            nations_before: Some(nations_before),
            delegate_exec: Some(delegate_exec),
            embassies,
            ..
        } = region
        else {
            continue;
        };

        let is_governorless = governorless.iter().any(|r| r == &name);
        let is_passwordless = passwordless.iter().any(|r| r == &name);

        let format = if is_governorless && is_passwordless {
            Some(&green_fill)
        } else if !is_governorless && delegate_exec && is_passwordless {
            Some(&yellow_fill)
        } else if !is_passwordless {
            Some(&red_fill)
        } else {
            None
        };

        let link = format!(
            "https://www.nationstates.net/region={}",
            name.to_lowercase().replace(' ', "_")
        );

        if let Some(format) = format {
            worksheet.write_string_with_format(row_index, 0, &name, format)?;
            worksheet.write_url_with_format(row_index, 1, link.as_str(), format)?;
        } else {
            worksheet.write_string(row_index, 0, &name)?;
            worksheet.write_url(row_index, 1, link.as_str())?;
        }

        worksheet.write_number(row_index, 2, population)?;

        worksheet.write_number(row_index, 3, nations_before)?;

        let progress = nations_before as f32 / total_population as f32;

        let minor_duration = (progress * minor_length as f32).floor() as i32;
        let minor_h = minor_duration / 3600;
        let minor_m = (minor_duration / 60) % 60;
        let minor_s = minor_duration % 60;
        worksheet.write_datetime(
            row_index,
            4,
            &ExcelDateTime::from_hms(minor_h.try_into()?, minor_m.try_into()?, minor_s)?,
        )?;

        let major_duration = (progress * major_length as f32).floor() as i32;
        let major_h = major_duration / 3600;
        let major_m = (major_duration / 60) % 60;
        let major_s = major_duration % 60;
        worksheet.write_datetime(
            row_index,
            5,
            &ExcelDateTime::from_hms(major_h.try_into()?, major_m.try_into()?, major_s)?,
        )?;

        worksheet.write_number(row_index, 6, delegate_votes)?;

        if delegate_votes == 0 {
            worksheet.write_number_with_format(row_index, 7, delegate_votes, &red_fill)?;
        } else {
            worksheet.write_number(row_index, 7, delegate_votes - 1)?;
        }

        // maximum length of cell contents in Excel is 32,767 characters
        // https://support.microsoft.com/en-us/office/excel-specifications-and-limits-1672b34d-7043-467e-8e27-269d656771c3
        let mut embassy_list = embassies.join(",");
        embassy_list.truncate(32767);
        worksheet.write_string(row_index, 8, embassy_list)?;

        factbook.truncate(32767);
        worksheet.write_string(row_index, 9, factbook)?;

        row_index += 1;
    }

    workbook.save(output_file)?;

    Ok(())
}

fn get_regions(agent: &Agent, url: &str) -> Result<Vec<String>> {
    let body = agent.get(url).call()?.into_string()?;

    let mut reader = Reader::from_str(&body);

    let mut collecting = false;
    let mut regions: Vec<String> = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"REGIONS" => {
                collecting = true;
            }
            Event::End(e) if e.name().as_ref() == b"REGIONS" => {
                collecting = false;
            }
            Event::Text(e) if collecting => {
                // dbg!(e.unescape()?);
                regions = e.unescape()?.split(',').map(|s| s.to_string()).collect();
            }
            Event::Eof => break,
            _ => (),
        }
    }

    Ok(regions)
}

pub fn get_governorless_regions(agent: &Agent) -> Result<Vec<String>> {
    let url = "https://www.nationstates.net/cgi-bin/api.cgi?q=regionsbytag;tags=governorless";
    get_regions(agent, url)
}

pub fn get_passwordless_regions(agent: &Agent) -> Result<Vec<String>> {
    let url = "https://www.nationstates.net/cgi-bin/api.cgi?q=regionsbytag;tags=-password";
    get_regions(agent, url)
}
