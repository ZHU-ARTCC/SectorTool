use crate::geo::LatLon;

pub mod parse;

#[derive(Debug, Builder, Clone, PartialEq, Eq)]
pub struct Unit {
    pub designator: String,
    pub ty: String,
    pub airport_location: String,
}

#[derive(Clone, Debug, Builder)]
#[builder(private)]
pub struct Airport {
    pub id: String,
    pub designator: String,
    pub latlon: LatLon
}

#[derive(Clone, Debug, Builder)]
#[builder(private)]
pub struct Navaid {
    pub designator: String,
    pub ty: NavaidType,
    pub latlon: LatLon,
    pub low_id: String,
}

#[derive(Copy, Clone, Debug)]
pub enum NavaidType {
    VORTAC,
    VORDME
}