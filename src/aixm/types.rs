use crate::geo::LatLon;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub designator: String,
    pub ty: String,
    pub airport_location: String,
}

#[derive(Debug, Clone)]
pub struct Airport {
    pub id: String,
    pub designator: String,
    pub latlon: LatLon
}

pub struct Navaid {
    pub designator: String,
    pub ty: NavaidType,
    pub latlon: LatLon,
    pub low_id: String,
}


pub enum NavaidType {
    VORTAC,
    VORDME
}
