use regex::Regex;
use lazy_static::lazy_static;
use itertools::Itertools;

#[derive(Clone, Copy, Debug)]
pub struct LatLon(f64, f64);

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> Self {
        LatLon(lat, lon)
    }

    pub fn from_aixm(x: &str) -> Self {
        let whatever: Vec<_> = x
            .split_whitespace()
            .map(|s| s.parse().expect("LatLon fail"))
            .collect();
        // Its backwards in AIXM
        LatLon(whatever[1], whatever[0])
    }

    //Ex: 31-53-00.510N
    pub fn from_fix_txt(lat: &str, lon: &str) -> Option<Self> {
        fn to_dd(d: f64, m: f64, s: f64) -> f64 {
            d + m/60.0 + s/3600.0
        }

        lazy_static! {
            static ref LAT_LON_REGEX : Regex = Regex::new(r"(\d+)-(\d+)-(\d+\.\d+)(\w)").unwrap();
        }

        let lat = LAT_LON_REGEX.captures(lat)
            .and_then(|cap| {
                let (d,m,s,dir) = (&cap[1], &cap[2], &cap[3], &cap[4]);
                let (d, m, s) = (d.parse().ok()?, m.parse().ok()?, s.parse().ok()?);
                let mut dd = to_dd(d, m, s);
                if dir == "S" || dir == "W" {
                    dd = - dd;
                }
                Some(dd)
            });

        let lon = LAT_LON_REGEX.captures(lon)
            .and_then(|cap| {
                let (d,m,s,dir) = (&cap[1], &cap[2], &cap[3], &cap[4]);
                let (d, m, s) = (d.parse().ok()?, m.parse().ok()?, s.parse().ok()?);
                let mut dd = to_dd(d, m, s);
                if dir == "S" || dir == "W" {
                    dd = - dd;
                }
                Some(dd)
            });

        match (lat, lon) {
            (Some(lat), Some(lon)) => Some(LatLon(lat, lon)),
            _ => None
        }
    }

    pub fn to_vrc(self) -> String {
        fn to_dms(dd: f64) -> (i32, i32, f64) {
            let d = dd.trunc() as i32;
            let m = (dd.abs() * 60.0).trunc() as i32 % 60;
            let s = (dd.abs() * 3600.0) % 60.0;
            (d, m, s)
        }

        let mut tmp = String::new();
        tmp += if self.0.is_sign_positive() { "N" } else { "S" };
        let (d, m, s) = to_dms(self.0);
        tmp += &format!("{:03}.{:02}.{:06.03}", d.abs(), m, s);

        tmp += " ";

        tmp += if self.1.is_sign_positive() { "E" } else { "W" };
        let (d, m, s) = to_dms(self.1);
        tmp += &format!("{:03}.{:02}.{:06.03}", d.abs(), m, s);
        tmp
    }
}