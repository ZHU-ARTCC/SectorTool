use quick_xml::events::*;
use quick_xml::Reader;
use std::io::BufRead;
use crate::error::{Error, Result};
use super::types::*;
use crate::geo::LatLon;

const XMLNS_XLINK: &str = "http://www.w3.org/1999/xlink";
const XMLNS_GML: &str = "http://www.opengis.net/gml/3.2";

fn extract_gml_id(href: &str) -> Option<&str> {
    let s = href.split_at(href.find("@gml:id")?).1;
    let mut data = s.split('\'');
    data.next(); // Dispose of the gml:id part
    Some(data.next()?.trim())
}



fn get_attribute<B: BufRead>(reader: &Reader<B>, tag: &BytesStart, attr: &str) -> Option<Result<String>> {
    tag.attributes().flat_map(|x| x)
        .map(|x| (x.key, x.unescape_and_decode_value(reader)))
                    .find(|a| a.0 == attr.as_bytes())
                    .map(|a| a.1.map_err(|e| e.into()))
}

fn get_unit<B: BufRead>(reader: &mut Reader<B>, buf: &mut Vec<u8>) -> Result<Unit> {
    let mut unit = Unit {
        designator: String::new(),
        ty: String::new(),
        airport_location: String::new(),
    };
    let mut airport_location = None;

    loop {
        match reader.read_event(buf)? {
            Event::Start(ref event) if event.name() == b"aixm:designator" => {
                unit.designator = reader.read_text("aixm:designator", buf)?
            }
            Event::Start(ref event) if event.name() == b"aixm:type" => {
                unit.ty = reader.read_text("aixm:type", buf)?
            }
            Event::Empty(ref event) if event.name() == b"aixm:airportLocation" => {
                airport_location = get_attribute(reader, event, "xlink:href");
            }
            Event::End(ref event) if event.name() == b"aixm:Unit" => break,
            Event::Eof => return Err(quick_xml::Error::UnexpectedEof("EOF".to_owned()).into()),
            _ => ()
        }
        buf.clear();
    }

    if let Some(airport_location) = airport_location {
        let airport_location = airport_location?;
        let airport_location = extract_gml_id(&airport_location);
        if let Some(airport_location) = airport_location {
            unit.airport_location = airport_location.to_owned();
            Ok(unit)
        }
        else {
            Err(Error::NotYielded)
        }
    } else {
        Err(Error::NotYielded)
    }
}


fn get_airport<B: BufRead>(reader: &mut Reader<B>, buf: &mut Vec<u8>, start: &BytesStart) -> Result<Airport> {
    let id = get_attribute(reader, start, "gml:id").expect("Airport->ID")?;
    let mut designator = String::new();
    let mut latlon = None;

    loop {
        match reader.read_event(buf)? {
            Event::Start(ref event) if event.name() == b"aixm:designator" => {
                designator = reader.read_text("aixm:designator", buf)?;
            }
            Event::Start(ref event) if event.name() == b"gml:pos" => {
                latlon = Some(LatLon::from_aixm(&reader.read_text("gml:pos", buf)?));
            }
            Event::End(ref event) if event.name() == b"aixm:AirportHeliport" => break,
            Event::Eof => return Err(quick_xml::Error::UnexpectedEof("EOF".to_owned()).into()),
            _ => ()
        }
        buf.clear();
    }

    if let Some(latlon) = latlon {
        Ok(Airport{
            id, designator, latlon
        })
    } else {
        Err(Error::NotYielded)
    }
}

pub fn get_airport_info<B: BufRead, T: AsRef<str>>(aixm: &mut Reader<B>, filter: &[T]) -> Result<Vec<Airport>> {
    use std::collections::HashSet;
    let mut buf = Vec::new();
    let mut units = HashSet::new();
    let mut airports = Vec::new();
    loop {
        match aixm.read_event(&mut buf)? {
            Event::Start(ref event) if event.name() == b"aixm:Unit" => {
                match get_unit(aixm, &mut buf) {
                    Ok(unit) => {
                        if unit.ty == "ARTCC" && filter.iter().any(|x| x.as_ref() == unit.designator) {
                            units.insert(unit.airport_location);
                        }
                    }
                    Err(Error::NotYielded) => (),
                    Err(x) => return Err(x) 
                }
            }
            Event::Start(ref event) if event.name() == b"aixm:AirportHeliport" => {
                match get_airport(aixm, &mut Vec::new(), event) {
                    Ok(airport) => {
                        airports.push(airport);
                    }
                    Err(Error::NotYielded) => (),
                    Err(x) => return Err(x) 
                }
            }
            Event::Eof => break,
            _ => ()
        }
        buf.clear();
    }

    airports.retain(|a| units.contains(&a.id));

    Ok(airports)
}

fn get_navaid<B: BufRead>(reader: &mut Reader<B>) -> Result<Navaid> {
    let mut navaid = Navaid {
        designator: String::new(),
        ty: NavaidType::VORTAC,
        latlon: LatLon::new(0.0,0.0),
        low_id: String::new()
    };

    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf)? {
            Event::Start(ref event) if event.name() == b"aixm:designator" => {
                 let raw = reader.read_text(b"aixm:designator", &mut buf)?;
                 navaid.designator = raw; 
            }
            Event::Start(ref event) if event.name() == b"aixm:type" => {
                let raw = reader.read_text(b"aixm:type", &mut buf)?;
                navaid.ty = match &*raw {
                    "VORTAC" => NavaidType::VORTAC,
                    "VOR_DME" => NavaidType::VORDME,
                    _ => return Err(Error::NotYielded)
                }; 
            }
            Event::Start(ref event) if event.name() == b"gml:pos" => {
                let raw = reader.read_text(b"gml:pos", &mut buf)?;
                navaid.latlon = LatLon::from_aixm(&raw);
            }
            Event::Start(ref event) if event.name() == b"nav:artccIdForLowAltitude" => {
                let raw = reader.read_text(b"nav:artccIdForLowAltitude", &mut buf)?;
                navaid.low_id = raw;
            }
            Event::End(ref event) if event.name() == b"aixm:Navaid" => break,
            Event::Eof => return Err(quick_xml::Error::UnexpectedEof("EOF".to_owned()).into()),
            _ => ()
        }
        buf.clear();
    }
    Ok(navaid)
}

pub fn get_navaid_info<B: BufRead, T: AsRef<str>>(aixm: &mut Reader<B>, filter: &[T]) -> Result<Vec<Navaid>> {
    let mut buf = Vec::new();
    let mut navaids = Vec::new();

    loop {
        match aixm.read_event(&mut buf)? {
            Event::Start(ref event) if event.name() == b"aixm:Navaid" => {
                match get_navaid(aixm) {
                    Ok(navaid) => {
                        if filter.iter().any(|x| x.as_ref() == navaid.low_id) {
                            navaids.push(navaid)
                        }
                    }
                    Err(Error::NotYielded) => (),
                    Err(e) => break Err(e)
                }
            }
            Event::Eof => break Ok(navaids),
            _ => ()
        }
        buf.clear();
    }
}