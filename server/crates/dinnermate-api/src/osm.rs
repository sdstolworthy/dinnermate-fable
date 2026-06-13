//! OSM/Overpass `RestaurantProvider`. Keyless; identifies itself with a
//! descriptive User-Agent per the Overpass usage policy.

use std::collections::HashMap;

use async_trait::async_trait;
use dinnermate_core::{
    parse_osm_opening_hours, ProviderDetails, ProviderError, Restaurant, RestaurantProvider,
    RoomParams,
};
use serde::Deserialize;

/// Production endpoint; tests inject a stub server's URL instead.
pub const OVERPASS_BASE_URL: &str = "https://overpass-api.de";

const USER_AGENT: &str = "dinnermate/1.0 (https://dinnermate.coolify.stolworthy.co)";

pub struct OsmProvider {
    http: reqwest::Client,
    base_url: String,
}

impl OsmProvider {
    pub fn new(http: reqwest::Client, base_url: String) -> Self {
        OsmProvider { http, base_url }
    }
}

fn overpass_query(params: &RoomParams) -> String {
    format!(
        "[out:json][timeout:10];\
         nwr[\"amenity\"=\"restaurant\"][\"name\"](around:{},{},{});\
         out center 60;",
        params.radius_m, params.lat, params.lng
    )
}

/// "123 Main St, Maple City" from whichever addr:* parts are tagged; empty
/// string when none are.
fn join_address(tags: &HashMap<String, String>) -> String {
    let street_line = [tags.get("addr:housenumber"), tags.get("addr:street")]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    let mut parts = Vec::new();
    if !street_line.is_empty() {
        parts.push(street_line);
    }
    if let Some(city) = tags.get("addr:city") {
        parts.push(city.clone());
    }
    parts.join(", ")
}

/// Maps one Overpass element; `None` drops it from the deck. Nodes carry
/// their own coords, ways/relations rely on `out center`; an element with
/// neither cannot be shown on a map or distance-filtered, so it is skipped.
/// OSM has no rating/price/photo data, so those stay `None` and the
/// unknown-passes filter keeps the entries. The UTC offset is resolved from
/// the coords at search time — a snapshot; DST flips mid-room-life are
/// accepted because rooms are short-lived (see the meal-time design doc).
fn to_restaurant(element: Element) -> Option<Restaurant> {
    let (lat, lng) = element
        .lat
        .zip(element.lon)
        .or_else(|| element.center.as_ref().map(|c| (c.lat, c.lon)))?;
    // The query requires a name tag; this guards against malformed elements.
    let name = element.tags.get("name")?.clone();
    let cuisine = element.tags.get("cuisine").map(|raw| {
        raw.split(';')
            .next()
            .unwrap_or(raw)
            .trim()
            .to_lowercase()
            .replace('_', " ")
    });
    let hours = element
        .tags
        .get("opening_hours")
        .and_then(|raw| parse_osm_opening_hours(raw));
    Some(Restaurant {
        id: format!("osm-{}-{}", element.kind, element.id),
        name,
        cuisine,
        price_level: None,
        rating: None,
        rating_count: None,
        address: join_address(&element.tags),
        photo_url: None,
        lat: Some(lat),
        lng: Some(lng),
        hours,
        utc_offset_minutes: crate::tz::utc_offset_minutes(lat, lng, chrono::Utc::now()),
    })
}

/// "osm-<type>-<id>" → the element's openstreetmap.org URL; anything else
/// (malformed or foreign ids) yields `None`.
fn maps_url(restaurant_id: &str) -> Option<String> {
    let (kind, id) = restaurant_id.strip_prefix("osm-")?.split_once('-')?;
    (!kind.is_empty() && !id.is_empty())
        .then(|| format!("https://www.openstreetmap.org/{kind}/{id}"))
}

/// Returns the body of a 2xx response; non-2xx becomes `Unavailable` carrying
/// the status and a body excerpt.
async fn read_success_body(response: reqwest::Response) -> Result<String, ProviderError> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|err| ProviderError::Unavailable(err.to_string()))?;
    if !status.is_success() {
        let excerpt: String = text.chars().take(200).collect();
        return Err(ProviderError::Unavailable(format!("status {status}: {excerpt}")));
    }
    Ok(text)
}

#[async_trait]
impl RestaurantProvider for OsmProvider {
    async fn search(&self, params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError> {
        let response = self
            .http
            .post(format!("{}/api/interpreter", self.base_url))
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .form(&[("data", overpass_query(params))])
            .send()
            .await
            .map_err(|err| ProviderError::Unavailable(err.to_string()))?;
        let text = read_success_body(response).await?;

        let parsed: OverpassResponse = serde_json::from_str(&text)
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;
        Ok(parsed.elements.into_iter().filter_map(to_restaurant).collect())
    }

    /// OSM has no reviews/website/phone endpoint worth a second round trip;
    /// details are just a link back to the element.
    async fn details(&self, restaurant_id: &str) -> Result<ProviderDetails, ProviderError> {
        Ok(ProviderDetails { maps_url: maps_url(restaurant_id), ..ProviderDetails::default() })
    }
}

#[derive(Debug, Deserialize)]
struct OverpassResponse {
    #[serde(default)]
    elements: Vec<Element>,
}

#[derive(Debug, Deserialize)]
struct Element {
    #[serde(rename = "type")]
    kind: String,
    id: u64,
    lat: Option<f64>,
    lon: Option<f64>,
    center: Option<Center>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Center {
    lat: f64,
    lon: f64,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::post;
    use axum::{Form, Router};
    use dinnermate_core::{HoursPeriod, ProviderError, RestaurantProvider, RoomParams};
    use serde_json::json;

    use super::OsmProvider;

    #[derive(Debug, Clone)]
    struct CapturedRequest {
        headers: HeaderMap,
        form: HashMap<String, String>,
    }

    #[derive(Clone)]
    struct StubState {
        status: StatusCode,
        body: String,
        captured: Arc<Mutex<Option<CapturedRequest>>>,
    }

    struct Stub {
        base_url: String,
        captured: Arc<Mutex<Option<CapturedRequest>>>,
    }

    impl Stub {
        fn captured(&self) -> CapturedRequest {
            self.captured
                .lock()
                .unwrap()
                .clone()
                .expect("stub received no request")
        }
    }

    async fn handler(
        State(state): State<StubState>,
        headers: HeaderMap,
        Form(form): Form<HashMap<String, String>>,
    ) -> (StatusCode, String) {
        *state.captured.lock().unwrap() = Some(CapturedRequest { headers, form });
        (state.status, state.body)
    }

    async fn spawn_stub(status: StatusCode, body: &str) -> Stub {
        let captured = Arc::new(Mutex::new(None));
        let state = StubState {
            status,
            body: body.to_string(),
            captured: Arc::clone(&captured),
        };
        let app = Router::new()
            .route("/api/interpreter", post(handler))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Stub { base_url: format!("http://{addr}"), captured }
    }

    fn provider(base_url: &str) -> OsmProvider {
        OsmProvider::new(reqwest::Client::new(), base_url.to_string())
    }

    fn params() -> RoomParams {
        RoomParams {
            lat: 40.76,
            lng: -111.89,
            location_label: "Salt Lake City".to_string(),
            radius_m: 2500,
            cuisines: vec![],
            price_min: 1,
            price_max: 4,
            min_rating: 0.0,
            eat_at_utc: None,
        }
    }

    fn canned_elements() -> String {
        json!({
            "elements": [
                {
                    "type": "node",
                    "id": 111,
                    "lat": 40.761,
                    "lon": -111.891,
                    "tags": {
                        "name": "Taco Stand",
                        "cuisine": "mexican;tacos",
                        "opening_hours": "Mo-Fr 11:00-14:30,17:00-22:00; Sa-Su 12:00-23:00",
                        "addr:housenumber": "123",
                        "addr:street": "Main St",
                        "addr:city": "Maple City"
                    }
                },
                {
                    "type": "way",
                    "id": 222,
                    "center": {"lat": 40.762, "lon": -111.892},
                    "tags": {"name": "Scoop Shop", "cuisine": "ice_cream"}
                },
                {
                    "type": "way",
                    "id": 333,
                    "tags": {"name": "Nowhere Diner"}
                }
            ]
        })
        .to_string()
    }

    fn period(day: u8, open: &str, close: &str) -> HoursPeriod {
        HoursPeriod { day, open: open.into(), close: close.into() }
    }

    #[tokio::test]
    async fn maps_elements_with_center_fallback_and_skips_unlocatable() {
        let stub = spawn_stub(StatusCode::OK, &canned_elements()).await;

        let restaurants = provider(&stub.base_url).search(&params()).await.unwrap();

        assert_eq!(
            restaurants.len(),
            2,
            "the entry with neither lat/lon nor center must be skipped"
        );

        let node = &restaurants[0];
        assert_eq!(node.id, "osm-node-111");
        assert_eq!(node.name, "Taco Stand");
        assert_eq!(node.cuisine.as_deref(), Some("mexican"), "first ;-token only");
        assert_eq!(node.address, "123 Main St, Maple City");
        assert_eq!((node.lat, node.lng), (Some(40.761), Some(-111.891)));
        let mut expected_hours: Vec<HoursPeriod> = (1..=5)
            .flat_map(|day| [period(day, "11:00", "14:30"), period(day, "17:00", "22:00")])
            .collect();
        expected_hours.push(period(6, "12:00", "23:00"));
        expected_hours.push(period(0, "12:00", "23:00"));
        assert_eq!(node.hours.as_deref(), Some(expected_hours.as_slice()));
        assert_eq!(
            (node.rating, node.price_level, node.rating_count, node.photo_url.as_deref()),
            (None, None, None, None),
            "OSM has no rating/price/photo data"
        );
        let offset = node
            .utc_offset_minutes
            .expect("SLC fixture coords must resolve to a tz offset");
        assert!(
            (-420..=-360).contains(&offset),
            "offset {offset} outside the America/Denver MST/MDT range"
        );

        let way = &restaurants[1];
        assert_eq!(way.id, "osm-way-222");
        assert_eq!(way.cuisine.as_deref(), Some("ice cream"), "underscores become spaces");
        assert_eq!((way.lat, way.lng), (Some(40.762), Some(-111.892)), "way uses center");
        assert_eq!(way.address, "");
        assert_eq!(way.hours, None);
    }

    #[tokio::test]
    async fn sends_user_agent_and_exact_overpass_query() {
        let stub = spawn_stub(StatusCode::OK, r#"{"elements":[]}"#).await;

        provider(&stub.base_url).search(&params()).await.unwrap();

        let captured = stub.captured();
        assert_eq!(
            captured.headers.get("user-agent").map(|v| v.to_str().unwrap()),
            Some("dinnermate/1.0 (https://dinnermate.coolify.stolworthy.co)")
        );
        assert_eq!(
            captured.form.get("data").map(String::as_str),
            Some(
                "[out:json][timeout:10];\
                 nwr[\"amenity\"=\"restaurant\"][\"name\"](around:2500,40.76,-111.89);\
                 out center 60;"
            )
        );
    }

    #[tokio::test]
    async fn gateway_timeout_maps_to_unavailable() {
        let stub = spawn_stub(StatusCode::GATEWAY_TIMEOUT, "overloaded").await;

        let err = provider(&stub.base_url).search(&params()).await.unwrap_err();

        match err {
            ProviderError::Unavailable(message) => {
                assert!(message.contains("504"), "unexpected message: {message}");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn unparseable_body_maps_to_invalid_response() {
        let stub = spawn_stub(StatusCode::OK, "definitely not json").await;

        let err = provider(&stub.base_url).search(&params()).await.unwrap_err();

        assert!(
            matches!(err, ProviderError::InvalidResponse(_)),
            "expected InvalidResponse, got {err:?}"
        );
    }

    #[tokio::test]
    async fn details_builds_maps_url_from_osm_id() {
        let details = provider("http://unused.invalid").details("osm-way-123").await.unwrap();

        assert_eq!(
            details.maps_url.as_deref(),
            Some("https://www.openstreetmap.org/way/123")
        );
        assert_eq!((details.website, details.phone), (None, None));
        assert!(details.reviews.is_empty());
    }

    #[tokio::test]
    async fn details_on_malformed_id_is_default() {
        let details = provider("http://unused.invalid").details("garbage").await.unwrap();

        assert_eq!(details, dinnermate_core::ProviderDetails::default());
    }
}
