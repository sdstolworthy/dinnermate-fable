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

/// Filters a provider result down to the room's deck and orders it so all
/// participants see the same deck.
///
/// Fairness rule: unknown passes. Each predicate only excludes a restaurant
/// when the field is `Some` and fails; `None` (data the provider doesn't
/// have) never disqualifies. Ordering: rated before unrated, rating desc
/// within rated, then name asc.
pub fn apply(params: &RoomParams, restaurants: Vec<Restaurant>) -> Vec<Restaurant> {
    let mut deck: Vec<Restaurant> = restaurants
        .into_iter()
        .filter(|r| {
            (params.cuisines.is_empty()
                || r.cuisine
                    .as_ref()
                    .is_none_or(|c| params.cuisines.iter().any(|p| p.eq_ignore_ascii_case(c))))
                && r.price_level
                    .is_none_or(|p| (params.price_min..=params.price_max).contains(&p))
                && r.rating.is_none_or(|rating| rating >= params.min_rating)
                && r.lat.zip(r.lng).is_none_or(|(lat, lng)| {
                    haversine_m(params.lat, params.lng, lat, lng) <= f64::from(params.radius_m)
                })
        })
        .collect();
    deck.sort_by(|a, b| {
        match (a.rating, b.rating) {
            (Some(ar), Some(br)) => br.total_cmp(&ar),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
        .then_with(|| a.name.cmp(&b.name))
    });
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
            cuisine: Some(cuisine.into()),
            price_level: Some(price),
            rating: Some(rating),
            rating_count: Some(100),
            address: "123 Main St".into(),
            photo_url: None,
            lat: Some(CENTER_LAT),
            lng: Some(CENTER_LNG),
            hours: None,
            utc_offset_minutes: None,
        }
    }

    /// Restaurant with every optional field unknown.
    fn unknown(id: &str, name: &str) -> Restaurant {
        Restaurant {
            id: id.into(),
            name: name.into(),
            cuisine: None,
            price_level: None,
            rating: None,
            rating_count: None,
            address: String::new(),
            photo_url: None,
            lat: None,
            lng: None,
            hours: None,
            utc_offset_minutes: None,
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
            lat: Some(FAR_LAT),
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

    #[test]
    fn unknown_passes_table() {
        let cases: Vec<(&str, RoomParams, Vec<Restaurant>, Vec<&str>)> = vec![
            (
                "price None passes any window",
                RoomParams { price_min: 2, price_max: 3, ..base_params() },
                vec![
                    Restaurant { price_level: None, ..restaurant("u", "Unknown", "thai", 2, 4.0) },
                    restaurant("out", "Out", "thai", 4, 4.0),
                ],
                vec!["u"],
            ),
            (
                "rating None passes min_rating 4",
                RoomParams { min_rating: 4.0, ..base_params() },
                vec![
                    Restaurant { rating: None, ..restaurant("u", "Unknown", "thai", 2, 4.0) },
                    restaurant("low", "Low", "thai", 2, 3.0),
                ],
                vec!["u"],
            ),
            (
                "lat/lng None passes radius",
                RoomParams { radius_m: 1_000, ..base_params() },
                vec![
                    Restaurant {
                        lat: None,
                        lng: None,
                        ..restaurant("u", "Unknown", "thai", 2, 4.0)
                    },
                    Restaurant {
                        lat: Some(FAR_LAT),
                        ..restaurant("far", "Far", "thai", 2, 4.0)
                    },
                ],
                vec!["u"],
            ),
            (
                "cuisine None passes cuisine filter",
                RoomParams { cuisines: vec!["thai".into()], ..base_params() },
                vec![
                    Restaurant { cuisine: None, ..restaurant("u", "Unknown", "thai", 2, 4.0) },
                    restaurant("mex", "Mex", "mexican", 2, 4.0),
                ],
                vec!["u"],
            ),
            (
                "all-unknown restaurant passes every filter",
                RoomParams {
                    cuisines: vec!["thai".into()],
                    price_min: 2,
                    price_max: 3,
                    min_rating: 4.5,
                    radius_m: 500,
                    ..base_params()
                },
                vec![unknown("u", "Mystery")],
                vec!["u"],
            ),
            (
                "Some values still excluded correctly",
                RoomParams {
                    cuisines: vec!["thai".into()],
                    price_min: 1,
                    price_max: 2,
                    min_rating: 4.0,
                    radius_m: 1_000,
                    ..base_params()
                },
                vec![
                    restaurant("ok", "Ok", "thai", 2, 4.2),
                    restaurant("cuisine", "Wrong Cuisine", "mexican", 2, 4.2),
                    restaurant("price", "Too Pricey", "thai", 3, 4.2),
                    restaurant("rating", "Too Low", "thai", 2, 3.9),
                    Restaurant {
                        lat: Some(FAR_LAT),
                        ..restaurant("far", "Too Far", "thai", 2, 4.2)
                    },
                ],
                vec!["ok"],
            ),
        ];
        for (name, params, input, want_ids) in cases {
            let got: Vec<String> = apply(&params, input).into_iter().map(|r| r.id).collect();
            assert_eq!(got, want_ids, "{name}");
        }
    }

    #[test]
    fn ordering_puts_rated_before_unrated() {
        let input = vec![
            unknown("u-b", "Bistro Unknown"),
            restaurant("zero", "Zero Star", "thai", 2, 0.0),
            unknown("u-a", "Aardvark Cafe"),
            restaurant("best", "Best", "thai", 2, 4.8),
            restaurant("good", "Good", "thai", 2, 4.2),
        ];
        let got: Vec<String> = apply(&base_params(), input).into_iter().map(|r| r.id).collect();
        // Rated first (desc, even a 0.0 rating beats unrated), then unrated name asc.
        assert_eq!(got, vec!["best", "good", "zero", "u-a", "u-b"]);
    }
}
