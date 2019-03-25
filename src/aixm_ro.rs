use roxmltree::ExpandedName;
use roxmltree::{Document, Node};

const XMLNS_XLINK: &str = "http://www.w3.org/1999/xlink";
const XMLNS_GML: &str = "http://www.opengis.net/gml/3.2";

#[derive(Clone, Copy, Debug)]
struct LatLon(f64, f64);

impl LatLon {
    fn from_aixm(x: &str) -> Self {
        let whatever : Vec<_> = x.split_whitespace().map(|s| s.parse().expect("LatLon fail")).collect();
        // Its backwards in AIXM
        LatLon(whatever[1], whatever[0])
    }

    fn to_vrc(self) -> String {
        fn to_dms(dd:f64) -> (i32, i32, f64) {
            let d = dd.trunc() as i32;
            let m = (dd.abs() * 60.0).trunc() as i32 % 60;
            let s = (dd.abs() * 3600.0) % 60.0;
            (d, m, s)
        }

        let mut tmp = String::new();
        tmp += if self.0.is_sign_positive() {"N"} else {"S"};
        let (d, m, s) = to_dms(self.0);
        tmp += &format!("{:03}.{:02}.{:02.03}", d.abs(), m, s);

        tmp += " ";

        tmp += if self.1.is_sign_positive() {"E"} else {"W"};
        let (d, m, s) = to_dms(self.1);
        tmp += &format!("{:03}.{:02}.{:02.03}", d.abs(), m, s);
        tmp
    }
}

#[derive(Debug)]
struct Airport<'a> {
    id: &'a str,
    designator: &'a str,
    location: LatLon
}

fn get_decendent_text<'a>(n: Node<'a, '_>, tag: &str) -> Option<&'a str> {
    for d in n.descendants() {
        if d.has_tag_name(tag) {
            return d.text();
        }
    }
    None
}

fn get_decendent_node<'a, 'd>(n: Node<'a, 'd>, tag: &str) -> Option<Node<'a, 'd>> {
    for d in n.descendants() {
        if d.has_tag_name(tag) {
            return Some(d);
        }
    }
    None
}

fn get_decendent_attribute<'a, 'd, A: Into<ExpandedName<'d>>> (n: Node<'a, 'd>, tag: &str, attr: A) -> Option<&'a str> {
    let attr = attr.into();
    for d in n.descendants() {
        if d.has_tag_name(tag) {
            return d.attribute(attr);
        }
    }
    None
}

fn get_unit_info<'a>(unit: Node<'a, '_>, name: &str) -> Option<&'a str> {
    let des = get_decendent_text(unit, "designator")?;
    let unit_type = get_decendent_text(unit, "type")?;
    let airport_location = get_decendent_attribute(unit, "airportLocation", (XMLNS_XLINK, "href"));

    if des == name && unit_type == "ARTCC" {
        airport_location
    } else {
        None
    }
}

fn get_airport<'a>(airport: Node<'a, '_>) -> Airport<'a> {
    let designator = 
    get_decendent_text(airport, "locationIndicatorICAO")
        .or(get_decendent_text(airport, "designator"))
        .expect("AirportHeliport->designator/icao");

    let id = airport.attribute((XMLNS_GML, "id")).expect("AirportHeliport@gml:id");
    let arp = get_decendent_node(airport, "ARP").expect("AirportHeliport->ARP");
    let location = get_decendent_text(arp, "pos").expect("ARP->pos");
    
    Airport {
        id,
        designator,
        location: LatLon::from_aixm(location)
    }
}

fn extract_gml_id(href: &str) -> Option<&str> {
    let s = href.split_at(href.find("@gml:id")?).1;
    let mut data = s.split("'");
    data.next(); // Dispose of the gml:id part
    Some(data.next()?.trim())
}


fn get_airport_info(apt: &Document) {
    use std::collections::HashSet;
    let mut valid_airport_ids = HashSet::new();
    let mut airports = Vec::new();
    
    // Build Airport and Unit Nodes
    for node in apt.descendants() {
        match node.tag_name().name() {
            "Unit" => {
                let unit = get_unit_info(node, "ZHU");
                if let Some(unit) = unit {
                    if let Some(id) = extract_gml_id(unit) {
                        valid_airport_ids.insert(id);
                    }
                }
            }
            "AirportHeliport" => {
                let airport = get_airport(node);
                airports.push(airport);
            }
            _ => (),
        }
    }

    //Filter Airports that are not in or border the ARTCC
    airports.retain(|a| valid_airport_ids.contains(a.id));
    
    for a in airports {
        println!("{} {}", a.designator, a.location.to_vrc());
    }
}