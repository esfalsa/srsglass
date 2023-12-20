use anyhow::Result;
use flate2::read::GzDecoder;
use quick_xml::{events::Event, Reader};
use rust_xlsxwriter::{Color, ExcelDateTime, Format, Workbook};
use std::{fs::File, io::BufReader, path::Path, time::SystemTime};
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
    timestamp_precision: i32,
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

    // excel only suppots up to 3 milliseconds of precision
    if !(0..=3).contains(&timestamp_precision) {
        return Err(anyhow::anyhow!(
            "timestamp_precision must be between 0 and 3"
        ));
    }

    let duration_string = match timestamp_precision {
        0 => "[h]:mm:ss",
        1 => "[h]:mm:ss.0",
        2 => "[h]:mm:ss.00",
        3 => "[h]:mm:ss.000",
        _ => unreachable!(),
    };

    let duration_format = Format::new().set_num_format(duration_string);
    worksheet.set_column_format(4, &duration_format)?;
    worksheet.set_column_format(5, &duration_format)?;

    worksheet.write_column(
        0,
        11,
        [
            "World Data",
            "Nations",
            "Major Length",
            "Secs/Nation",
            "Nations/Sec",
            "Minor Length",
            "Secs/Nation",
            "Nations/Sec",
            "",
            "Srsglass Version",
            "Date Generated",
        ],
    )?;

    worksheet.write_number(1, 12, total_population)?;
    worksheet.write_number(2, 12, major_length)?;
    worksheet.write_number(3, 12, major_length as f64 / total_population as f64)?;
    worksheet.write_number(4, 12, total_population as f64 / major_length as f64)?;
    worksheet.write_number(5, 12, minor_length)?;
    worksheet.write_number(6, 12, minor_length as f64 / total_population as f64)?;
    worksheet.write_number(7, 12, total_population as f64 / minor_length as f64)?;
    worksheet.write_string(9, 12, env!("CARGO_PKG_VERSION"))?;

    // set column width to fit date
    worksheet.set_column_width(12, 10)?;

    // set column widths to fit timestamp
    worksheet.set_column_width(4, 10)?;
    worksheet.set_column_width(5, 10)?;

    worksheet.write_datetime_with_format(
        10,
        12,
        &ExcelDateTime::from_timestamp(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs()
                .try_into()?,
        )?,
        &Format::new().set_num_format("yyyy-mm-dd;@"),
    )?;

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

        let progress = nations_before as f64 / total_population as f64;

        let minor_duration = progress * minor_length as f64;
        let minor_h = (minor_duration / 3600.0).floor() as u16;
        let minor_m = ((minor_duration / 60.0) % 60.0).floor() as u8;
        let minor_s = (minor_duration % 60.0).floor() as u8;
        let minor_ms = (minor_duration.fract() * 1000.0).round().clamp(0.0, 999.0) as u16;

        worksheet.write_datetime(
            row_index,
            4,
            &ExcelDateTime::from_hms_milli(minor_h, minor_m, minor_s, minor_ms)?,
        )?;

        let major_duration = progress * major_length as f64;
        let major_h = (major_duration / 3600.0).floor() as u16;
        let major_m = ((major_duration / 60.0) % 60.0).floor() as u8;
        let major_s = (major_duration % 60.0).floor() as u8;
        let major_ms = (major_duration.fract() * 1000.0).round().clamp(0.0, 999.0) as u16;

        worksheet.write_datetime(
            row_index,
            5,
            &ExcelDateTime::from_hms_milli(major_h, major_m, major_s, major_ms)?,
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
