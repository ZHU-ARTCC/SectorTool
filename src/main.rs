#![deny(clippy::all)]
#![forbid(unsafe_code)]

// FIXME: When derive_builder supports Rust 2018 syntax switch to a local import
#[macro_use]
extern crate derive_builder;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

use regex::Regex;
use std::path::PathBuf;
use structopt::StructOpt;
use zip::ZipArchive;

mod aixm;
mod error;
mod geo;
mod txt_data;
mod zip_util;

use geo::LatLon;
use txt_data::*;
use zip_util::*;

static VRC_SEPERATOR: &str =
    "\n\n;===============================================================================\n\n";

#[derive(StructOpt)]
struct Args {
    #[structopt(name = "input", parse(from_os_str))]
    input: PathBuf,
    #[structopt(
        short = "o",
        long = "output",
        parse(from_os_str),
        default_value = "./output.sct2"
    )]
    output: PathBuf,
    #[structopt(short = "f", long = "filter", raw(required = "true"))]
    artcc_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AirspaceType {
    ClassB,
    ClassC,
    ClassD,
    ClassE,
    MOA,
}

impl From<AirspaceType> for &str {
    fn from(x: AirspaceType) -> &'static str {
        match x {
            AirspaceType::ClassB => "B",
            AirspaceType::ClassC => "C",
            AirspaceType::ClassD => "D",
            AirspaceType::ClassE => "E",
            AirspaceType::MOA => "MOA",
        }
    }
}

fn main() -> Result<(), Box<Error>> {
    use std::collections::HashMap;
    use std::io::BufReader;

    let args = Args::from_args();
    let mut archive = ZipArchive::new(BufReader::new(File::open(args.input)?))?;

    // Airport processing
    println!("Unpacking airport AIXM...");
    let apt_aixm_zip =
        archive.by_name("Additional_Data/AIXM/AIXM_5.1/XML-Subscriber-Files/APT_AIXM.zip")?;
    let apt_aixm_file = zip_to_pseudofile(apt_aixm_zip).expect("Z->P failed");
    let mut apt = quick_xml::Reader::from_reader(apt_aixm_file);
    println!("Processing airport AIXM...");
    let airports = aixm::parse::get_airport_info(&mut apt, &args.artcc_ids)?;
    println!("Processing tower frequencies...");
    let twr = DataFile::from_reader(&mut archive.by_name("TWR.txt")?)?;
    let mut sct = String::new();

    // Free some memory (pls)
    drop(apt);

    let twr3_delim = &[
        (0, 4),  // Type
        (4, 4),  // Facillity Ident
        (8, 44), // Primary Frequency
    ];

    let twr8_delim = &[
        (0, 4),  // Type,
        (4, 4),  // Ident,
        (8, 1),  // Class B?
        (9, 1),  // Class C?
        (10, 1), // Class D?
        (11, 1), // Class E?
    ];

    let mut twr_frequencies = HashMap::new();

    let frequency_regex = Regex::new(r"\d{1,3}\.\d{1,3}").expect("Bad regex");

    for r in twr.records("TWR3", twr3_delim) {
        let thing = r[2].split_whitespace().next().unwrap();
        let freq = frequency_regex.find(thing);
        if let Some(freq) = freq {
            twr_frequencies.entry(r[1]).or_insert_with(|| freq.as_str());
        }
    }

    let twr_airspace: HashMap<_, _> = twr
        .records("TWR8", twr8_delim)
        .map(|r| {
            let class = match (r[2], r[3], r[4], r[5]) {
                ("Y", _, _, _) => AirspaceType::ClassB,
                (_, "Y", _, _) => AirspaceType::ClassC,
                (_, _, "Y", _) => AirspaceType::ClassD,
                _ => AirspaceType::ClassE,
            };
            (r[1], class)
        })
        .collect();

    println!("Merging airport data...");

    sct += VRC_SEPERATOR;
    sct += "[AIRPORT]\n";

    for a in airports {
        let airspace = twr_airspace
            .get(&*a.designator)
            .cloned()
            .unwrap_or(AirspaceType::ClassE);
        //if airspace == AirspaceType::ClassE { continue; }
        sct += &format!(
            "{:4} {:7} {} {}\n",
            a.designator,
            twr_frequencies.get(&*a.designator).unwrap_or(&"122.800"),
            a.latlon.to_vrc(),
            Into::<&str>::into(airspace)
        );
    }

    // VOR and DME Processing
    println!("Unpacking navaid AIXM...");
    let nav_aixm_zip =
        archive.by_name("Additional_Data/AIXM/AIXM_5.1/XML-Subscriber-Files/NAV_AIXM.zip")?;
    let nav_aixm_file = zip_to_pseudofile(nav_aixm_zip).expect("Z->P failed");

    println!("Processing navaid AIXM...");
    let mut nav = quick_xml::Reader::from_reader(nav_aixm_file);
    let navaids = aixm::parse::get_navaid_info(&mut nav, &args.artcc_ids)?;

    sct += VRC_SEPERATOR;
    sct += "[VOR]\n";

    for n in navaids {
        sct += &format!("{} 000.000 {}\n", n.designator, n.latlon.to_vrc());
    }

    // Fixes / NAVAID processing
    println!("Processing fix data...");
    let fix = DataFile::from_reader(&mut archive.by_name("FIX.txt")?)?;
    let fix1_delim = &[
        (0, 4),   // Type
        (66, 14), // Lat
        (80, 14), // Lon
        (228, 5), // NAS ID
        (237, 4), // ARTCC ID
    ];

    sct += VRC_SEPERATOR;
    sct += "[FIXES]\n";

    for r in fix.records("FIX1", fix1_delim) {
        let artcc_id = r[4];
        if args.artcc_ids.iter().any(|x| x == artcc_id) {
            let coord = LatLon::from_fix_txt(r[1], r[2]);
            if let Some(coord) = coord {
                sct += &format!("{} {}\n", r[3], coord.to_vrc());
            } else {
                println!("WARN: Bad Lat/Lon pair on FIX {}, ignoring!", r[3]);
            }
        }
    }

    println!("Outputing sct2 data...");
    let mut output = std::fs::File::create(args.output)?;
    output.write_all(sct.as_bytes())?;
    Ok(())
}
