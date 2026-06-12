use crate::model::{Restaurant, RoomParams};

/// Mean Earth radius in meters (IUGG); fine for city-scale distances.
const EARTH_RADIUS_M: f64 = 6_371_000.0;

pub fn haversine_m(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    let d_lat = (lat2 - lat1).to_radians();
    let d_lng = (lng2 - lng1).to_radians();
    let a = (d_lat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (d_lng / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_M * a.sqrt().asin()
}

/// Filters a provider result down to the room's deck and orders it
/// rating desc, then name asc, so all participants see the same deck.
pub fn apply(params: &RoomParams, restaurants: Vec<Restaurant>) -> Vec<Restaurant> {
    let mut deck: Vec<Restaurant> = restaurants
        .into_iter()
        .filter(|r| {
            (params.cuisines.is_empty()
                || params.cuisines.iter().any(|c| c.eq_ignore_ascii_case(&r.cuisine)))
                && (params.price_min..=params.price_max).contains(&r.price_level)
                && r.rating >= params.min_rating
                && haversine_m(params.lat, params.lng, r.lat, r.lng) <= f64::from(params.radius_m)
        })
        .collect();
    deck.sort_by(|a, b| b.rating.total_cmp(&a.rating).then_with(|| a.name.cmp(&b.name)));
    deck
}

#[cfg(test)]
mod tests {
    use super::*;

    const CENTER_LAT: f64 = 40.7600;
    const CENTER_LNG: f64 = -111.8900;
    // ~1.11 km due north of center.
    const FAR_LAT: f64 = 40.7700;

    fn base_params() -> RoomParams {
        RoomParams {
            lat: CENTER_LAT,
            lng: CENTER_LNG,
            location_label: "Downtown".into(),
            radius_m: 10_000,
            cuisines: vec![],
            price_min: 1,
            price_max: 4,
            min_rating: 0.0,
        }
    }

    fn restaurant(id: &str, name: &str, cuisine: &str, price: u8, rating: f32) -> Restaurant {
        Restaurant {
            id: id.into(),
            name: name.into(),
            cuisine: cuisine.into(),
            price_level: price,
            rating,
            rating_count: 100,
            address: "123 Main St".into(),
            photo_url: None,
            lat: CENTER_LAT,
            lng: CENTER_LNG,
        }
    }

    #[test]
    fn haversine_known_distance() {
        let d = haversine_m(CENTER_LAT, CENTER_LNG, FAR_LAT, CENTER_LNG);
        assert!((d - 1112.0).abs() < 5.0, "expected ~1112m, got {d}");
    }

    #[test]
    fn apply_table() {
        let far = Restaurant {
            lat: FAR_LAT,
            ..restaurant("far", "Far Spot", "thai", 2, 4.0)
        };
        let cases: Vec<(&str, RoomParams, Vec<Restaurant>, Vec<&str>)> = vec![
            (
                "empty cuisines passes all",
                base_params(),
                vec![
                    restaurant("a", "Alpha", "thai", 2, 4.0),
                    restaurant("b", "Beta", "mexican", 2, 4.0),
                ],
                vec!["a", "b"],
            ),
            (
                "cuisine filter is case-insensitive",
                RoomParams { cuisines: vec!["Thai".into()], ..base_params() },
                vec![
                    restaurant("a", "Alpha", "thai", 2, 4.0),
                    restaurant("b", "Beta", "mexican", 2, 4.0),
                ],
                vec!["a"],
            ),
            (
                "price window is inclusive",
                RoomParams { price_min: 2, price_max: 3, ..base_params() },
                vec![
                    restaurant("p1", "Cheap", "thai", 1, 4.0),
                    restaurant("p2", "Mid", "thai", 2, 4.0),
                    restaurant("p3", "Upper", "thai", 3, 4.0),
                    restaurant("p4", "Fancy", "thai", 4, 4.0),
                ],
                vec!["p2", "p3"],
            ),
            (
                "min_rating cut is inclusive",
                RoomParams { min_rating: 4.0, ..base_params() },
                vec![
                    restaurant("low", "Low", "thai", 2, 3.9),
                    restaurant("edge", "Edge", "thai", 2, 4.0),
                    restaurant("high", "High", "thai", 2, 4.5),
                ],
                vec!["high", "edge"],
            ),
            (
                "radius 1000m excludes point 1.11km away",
                RoomParams { radius_m: 1_000, ..base_params() },
                vec![restaurant("near", "Near Spot", "thai", 2, 4.0), far.clone()],
                vec!["near"],
            ),
            (
                "radius 1500m includes point 1.11km away",
                RoomParams { radius_m: 1_500, ..base_params() },
                vec![restaurant("near", "Near Spot", "thai", 2, 4.0), far.clone()],
                vec!["far", "near"],
            ),
            (
                "ordered by rating desc then name asc",
                base_params(),
                vec![
                    restaurant("zest", "Zest", "thai", 2, 4.5),
                    restaurant("apex", "Apex", "thai", 2, 4.5),
                    restaurant("top", "Middling Name", "thai", 2, 4.8),
                ],
                vec!["top", "apex", "zest"],
            ),
        ];
        for (name, params, input, want_ids) in cases {
            let got: Vec<String> = apply(&params, input).into_iter().map(|r| r.id).collect();
            assert_eq!(got, want_ids, "{name}");
        }
    }
}
