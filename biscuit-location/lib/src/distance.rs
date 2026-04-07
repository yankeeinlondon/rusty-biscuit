use geo::algorithm::line_measures::metric_spaces::Geodesic;
use geo::algorithm::line_measures::Distance;
use geo::Point;

use crate::types::{Coordinates, Distance as CoordDistance};

/// Compute the geodesic (ellipsoidal) distance between two coordinates.
///
/// Uses the Karney algorithm via the `geo` crate for high accuracy on WGS-84.
pub fn distance(from: &Coordinates, to: &Coordinates) -> crate::Result<CoordDistance> {
    // geo::Point uses (x=longitude, y=latitude) convention
    let p1 = Point::new(from.longitude, from.latitude);
    let p2 = Point::new(to.longitude, to.latitude);
    let meters = Geodesic::distance(p1, p2);
    Ok(CoordDistance { meters })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_point_is_zero() {
        let coords = Coordinates::new(34.0522, -118.2437).unwrap();
        let d = distance(&coords, &coords).unwrap();
        assert!(d.meters.abs() < 1.0);
    }

    #[test]
    fn la_to_new_york() {
        // LA to NYC is approximately 3,944 km
        let la = Coordinates::new(34.0522, -118.2437).unwrap();
        let nyc = Coordinates::new(40.7128, -74.0060).unwrap();
        let d = distance(&la, &nyc).unwrap();
        let km = d.meters / 1000.0;
        assert!(km > 3900.0 && km < 4000.0, "Expected ~3944 km, got {km}");
    }

    #[test]
    fn la_to_london() {
        // LA to London is approximately 8,757 km
        let la = Coordinates::new(34.0522, -118.2437).unwrap();
        let london = Coordinates::new(51.5074, -0.1278).unwrap();
        let d = distance(&la, &london).unwrap();
        let km = d.meters / 1000.0;
        assert!(km > 8700.0 && km < 8800.0, "Expected ~8757 km, got {km}");
    }

    #[test]
    fn distance_is_symmetric() {
        let a = Coordinates::new(34.0522, -118.2437).unwrap();
        let b = Coordinates::new(51.5074, -0.1278).unwrap();
        let d1 = distance(&a, &b).unwrap();
        let d2 = distance(&b, &a).unwrap();
        assert!((d1.meters - d2.meters).abs() < 1.0);
    }
}
