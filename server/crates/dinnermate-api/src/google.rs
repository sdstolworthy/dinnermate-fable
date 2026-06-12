//! Google Places (New) `RestaurantProvider` backed by `places:searchNearby`.

use async_trait::async_trait;
use dinnermate_core::{
    HoursPeriod, ProviderDetails, ProviderError, Restaurant, RestaurantProvider, Review,
    RoomParams,
};
use serde::Deserialize;
use serde_json::json;

/// Production endpoint; tests inject a stub server's URL instead.
pub const GOOGLE_PLACES_BASE_URL: &str = "https://places.googleapis.com";

const FIELD_MASK: &str = "places.id,places.displayName,places.formattedAddress,places.rating,\
                          places.userRatingCount,places.priceLevel,places.location,\
                          places.primaryType,places.photos,places.regularOpeningHours,\
                          places.utcOffsetMinutes";

const DETAILS_FIELD_MASK: &str = "websiteUri,nationalPhoneNumber,googleMapsUri,reviews";

const MAX_REVIEWS: usize = 5;

pub struct GooglePlacesProvider {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl GooglePlacesProvider {
    pub fn new(http: reqwest::Client, api_key: String, base_url: String) -> Self {
        GooglePlacesProvider { http, api_key, base_url }
    }

    fn to_restaurant(&self, place: Place) -> Restaurant {
        let cuisine = place
            .primary_type
            .map(|t| t.strip_suffix("_restaurant").unwrap_or(&t).to_string())
            .unwrap_or_else(|| "restaurant".to_string());
        let photo_url = place.photos.first().map(|photo| {
            format!(
                "{}/v1/{}/media?maxWidthPx=800&key={}",
                self.base_url, photo.name, self.api_key
            )
        });
        // v3 Task1: mechanical Some-wrapping to keep v2 behavior (defaults
        // for missing rating/price); honest None mapping is Task 4 work.
        Restaurant {
            id: place.id,
            name: place.display_name.text,
            cuisine: Some(cuisine),
            price_level: Some(price_level_from_enum(place.price_level.as_deref())),
            rating: Some(place.rating.unwrap_or(0.0)),
            rating_count: Some(place.user_rating_count.unwrap_or(0)),
            address: place.formatted_address.unwrap_or_default(),
            photo_url,
            lat: Some(place.location.latitude),
            lng: Some(place.location.longitude),
            hours: map_hours(place.regular_opening_hours),
            utc_offset_minutes: place.utc_offset_minutes,
        }
    }
}

/// Maps Google's `regularOpeningHours.periods` to weekly spans. Periods without
/// a close (Google's always-open sentinel) or with out-of-range fields are
/// skipped; an empty result collapses to `None` (unknown hours).
fn map_hours(hours: Option<OpeningHours>) -> Option<Vec<HoursPeriod>> {
    let periods: Vec<HoursPeriod> = hours?
        .periods
        .into_iter()
        .filter_map(|period| {
            let (open, close) = (period.open?, period.close?);
            let valid = open.day <= 6
                && open.hour < 24
                && open.minute < 60
                && close.hour < 24
                && close.minute < 60;
            valid.then(|| HoursPeriod {
                day: open.day,
                open: format!("{:02}:{:02}", open.hour, open.minute),
                close: format!("{:02}:{:02}", close.hour, close.minute),
            })
        })
        .collect();
    (!periods.is_empty()).then_some(periods)
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

/// Google omits `priceLevel` for many places; treat missing/unspecified/free
/// as mid-range so the price filter doesn't silently drop them.
fn price_level_from_enum(value: Option<&str>) -> u8 {
    match value {
        Some("PRICE_LEVEL_INEXPENSIVE") => 1,
        Some("PRICE_LEVEL_MODERATE") => 2,
        Some("PRICE_LEVEL_EXPENSIVE") => 3,
        Some("PRICE_LEVEL_VERY_EXPENSIVE") => 4,
        _ => 2,
    }
}

#[async_trait]
impl RestaurantProvider for GooglePlacesProvider {
    async fn search(&self, params: &RoomParams) -> Result<Vec<Restaurant>, ProviderError> {
        let body = json!({
            "includedTypes": ["restaurant"],
            "maxResultCount": 20,
            "locationRestriction": {
                "circle": {
                    "center": {"latitude": params.lat, "longitude": params.lng},
                    "radius": f64::from(params.radius_m),
                }
            }
        });
        let response = self
            .http
            .post(format!("{}/v1/places:searchNearby", self.base_url))
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", FIELD_MASK)
            .json(&body)
            .send()
            .await
            .map_err(|err| ProviderError::Unavailable(err.to_string()))?;
        let text = read_success_body(response).await?;

        let parsed: SearchResponse = serde_json::from_str(&text)
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;
        Ok(parsed
            .places
            .into_iter()
            .map(|place| self.to_restaurant(place))
            .collect())
    }

    // NOT verified against the live Place Details API (no key was available
    // at build time); covered by stub-server tests only.
    async fn details(&self, restaurant_id: &str) -> Result<ProviderDetails, ProviderError> {
        let response = self
            .http
            .get(format!("{}/v1/places/{restaurant_id}", self.base_url))
            .header("X-Goog-Api-Key", &self.api_key)
            .header("X-Goog-FieldMask", DETAILS_FIELD_MASK)
            .send()
            .await
            .map_err(|err| ProviderError::Unavailable(err.to_string()))?;
        let text = read_success_body(response).await?;

        let parsed: DetailsResponse = serde_json::from_str(&text)
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;
        Ok(ProviderDetails {
            website: parsed.website_uri,
            phone: parsed.national_phone_number,
            maps_url: parsed.google_maps_uri,
            reviews: parsed
                .reviews
                .into_iter()
                .take(MAX_REVIEWS)
                .map(|review| Review {
                    author: review
                        .author_attribution
                        .map(|a| a.display_name)
                        .unwrap_or_default(),
                    rating: review.rating.map(|r| r.round() as u8).unwrap_or(0),
                    text: review.text.map(|t| t.text).unwrap_or_default(),
                    relative_time: review.relative_publish_time_description,
                })
                .collect(),
        })
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    places: Vec<Place>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Place {
    id: String,
    display_name: DisplayName,
    formatted_address: Option<String>,
    rating: Option<f32>,
    user_rating_count: Option<u32>,
    price_level: Option<String>,
    location: LatLng,
    primary_type: Option<String>,
    #[serde(default)]
    photos: Vec<Photo>,
    regular_opening_hours: Option<OpeningHours>,
    utc_offset_minutes: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct OpeningHours {
    #[serde(default)]
    periods: Vec<OpeningPeriod>,
}

#[derive(Debug, Deserialize)]
struct OpeningPeriod {
    open: Option<TimePoint>,
    close: Option<TimePoint>,
}

/// Proto3 JSON omits zero-valued fields, so `day` 0 (Sunday) and midnight
/// hour/minute arrive as absent keys; default them to 0.
#[derive(Debug, Deserialize)]
struct TimePoint {
    #[serde(default)]
    day: u8,
    #[serde(default)]
    hour: u32,
    #[serde(default)]
    minute: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetailsResponse {
    website_uri: Option<String>,
    national_phone_number: Option<String>,
    google_maps_uri: Option<String>,
    #[serde(default)]
    reviews: Vec<GoogleReview>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleReview {
    author_attribution: Option<AuthorAttribution>,
    rating: Option<f32>,
    text: Option<ReviewText>,
    relative_publish_time_description: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthorAttribution {
    #[serde(default)]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct ReviewText {
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct DisplayName {
    text: String,
}

#[derive(Debug, Deserialize)]
struct LatLng {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
struct Photo {
    name: String,
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use dinnermate_core::{HoursPeriod, ProviderError, RestaurantProvider, RoomParams};
    use serde_json::{json, Value};

    use super::GooglePlacesProvider;

    #[derive(Debug, Clone)]
    struct CapturedRequest {
        headers: HeaderMap,
        body: Value,
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
        body: String,
    ) -> (StatusCode, String) {
        let parsed = serde_json::from_str(&body).unwrap_or(Value::Null);
        *state.captured.lock().unwrap() = Some(CapturedRequest { headers, body: parsed });
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
            .route("/v1/places:searchNearby", post(handler))
            .route("/v1/places/{id}", get(handler))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Stub { base_url: format!("http://{addr}"), captured }
    }

    fn provider(base_url: &str) -> GooglePlacesProvider {
        GooglePlacesProvider::new(
            reqwest::Client::new(),
            "test-key".to_string(),
            base_url.to_string(),
        )
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
        }
    }

    fn canned_two_places() -> String {
        json!({
            "places": [
                {
                    "id": "ChIJfull",
                    "displayName": {"text": "Thai Palace", "languageCode": "en"},
                    "formattedAddress": "123 Main St, Salt Lake City, UT",
                    "rating": 4.5,
                    "userRatingCount": 321,
                    "priceLevel": "PRICE_LEVEL_EXPENSIVE",
                    "location": {"latitude": 40.761, "longitude": -111.891},
                    "primaryType": "thai_restaurant",
                    "photos": [
                        {"name": "places/ChIJfull/photos/photo1"},
                        {"name": "places/ChIJfull/photos/photo2"}
                    ]
                },
                {
                    "id": "ChIJsparse",
                    "displayName": {"text": "Mystery Diner"},
                    "location": {"latitude": 40.762, "longitude": -111.892}
                }
            ]
        })
        .to_string()
    }

    #[tokio::test]
    async fn maps_places_response_with_defaults_for_missing_fields() {
        let stub = spawn_stub(StatusCode::OK, &canned_two_places()).await;

        let restaurants = provider(&stub.base_url).search(&params()).await.unwrap();

        assert_eq!(restaurants.len(), 2);

        let full = &restaurants[0];
        assert_eq!(full.id, "ChIJfull");
        assert_eq!(full.name, "Thai Palace");
        // v3 Task1: expectations wrap in Some; honest None mapping is Task 4.
        assert_eq!(full.cuisine.as_deref(), Some("thai"));
        assert_eq!(full.price_level, Some(3));
        assert_eq!(full.rating, Some(4.5));
        assert_eq!(full.rating_count, Some(321));
        assert_eq!(full.address, "123 Main St, Salt Lake City, UT");
        assert_eq!(
            full.photo_url.as_deref(),
            Some(
                format!(
                    "{}/v1/places/ChIJfull/photos/photo1/media?maxWidthPx=800&key=test-key",
                    stub.base_url
                )
                .as_str()
            )
        );
        assert_eq!((full.lat, full.lng), (Some(40.761), Some(-111.891)));

        let sparse = &restaurants[1];
        assert_eq!(sparse.id, "ChIJsparse");
        assert_eq!(sparse.name, "Mystery Diner");
        assert_eq!(sparse.cuisine.as_deref(), Some("restaurant"));
        assert_eq!(sparse.price_level, Some(2));
        assert_eq!(sparse.rating, Some(0.0));
        assert_eq!(sparse.rating_count, Some(0));
        assert_eq!(sparse.address, "");
        assert_eq!(sparse.photo_url, None);
    }

    #[tokio::test]
    async fn sends_expected_headers_and_search_body() {
        let stub = spawn_stub(StatusCode::OK, r#"{"places":[]}"#).await;

        provider(&stub.base_url).search(&params()).await.unwrap();

        let captured = stub.captured();
        assert_eq!(
            captured.headers.get("x-goog-api-key").map(|v| v.to_str().unwrap()),
            Some("test-key")
        );
        assert_eq!(
            captured
                .headers
                .get("x-goog-fieldmask")
                .map(|v| v.to_str().unwrap()),
            Some(
                "places.id,places.displayName,places.formattedAddress,places.rating,\
                 places.userRatingCount,places.priceLevel,places.location,\
                 places.primaryType,places.photos,places.regularOpeningHours,\
                 places.utcOffsetMinutes"
            )
        );
        assert_eq!(
            captured.body,
            json!({
                "includedTypes": ["restaurant"],
                "maxResultCount": 20,
                "locationRestriction": {
                    "circle": {
                        "center": {"latitude": 40.76, "longitude": -111.89},
                        "radius": 2500.0
                    }
                }
            })
        );
    }

    #[tokio::test]
    async fn non_success_status_maps_to_unavailable() {
        let stub = spawn_stub(
            StatusCode::FORBIDDEN,
            r#"{"error":{"message":"API key not authorized"}}"#,
        )
        .await;

        let err = provider(&stub.base_url).search(&params()).await.unwrap_err();

        match err {
            ProviderError::Unavailable(message) => {
                assert!(
                    message.contains("403") && message.contains("API key not authorized"),
                    "unexpected message: {message}"
                );
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
    async fn maps_opening_hours_and_utc_offset_with_zero_padding() {
        let body = json!({
            "places": [
                {
                    "id": "ChIJhours",
                    "displayName": {"text": "Night Owl"},
                    "location": {"latitude": 40.761, "longitude": -111.891},
                    "utcOffsetMinutes": -360,
                    "regularOpeningHours": {
                        "periods": [
                            // Proto3 JSON omits zero-valued hour/minute/day.
                            {"open": {"day": 1, "hour": 9, "minute": 5},
                             "close": {"day": 1, "hour": 17}},
                            {"open": {"hour": 11},
                             "close": {"hour": 14, "minute": 30}},
                            // Missing close (24h sentinel) must be skipped.
                            {"open": {"day": 3, "hour": 8}}
                        ]
                    }
                },
                {
                    "id": "ChIJnohours",
                    "displayName": {"text": "Mystery Diner"},
                    "location": {"latitude": 40.762, "longitude": -111.892}
                }
            ]
        })
        .to_string();
        let stub = spawn_stub(StatusCode::OK, &body).await;

        let restaurants = provider(&stub.base_url).search(&params()).await.unwrap();

        assert_eq!(restaurants[0].utc_offset_minutes, Some(-360));
        assert_eq!(
            restaurants[0].hours,
            Some(vec![
                HoursPeriod { day: 1, open: "09:05".into(), close: "17:00".into() },
                HoursPeriod { day: 0, open: "11:00".into(), close: "14:30".into() },
            ])
        );
        assert_eq!(restaurants[1].hours, None);
        assert_eq!(restaurants[1].utc_offset_minutes, None);
    }

    fn canned_details(review_count: usize) -> String {
        let reviews: Vec<Value> = (0..review_count)
            .map(|i| {
                json!({
                    "authorAttribution": {"displayName": format!("Reviewer {i}")},
                    "rating": 5,
                    "text": {"text": format!("Review text {i}")},
                    "relativePublishTimeDescription": "2 months ago"
                })
            })
            .collect();
        json!({
            "websiteUri": "https://thai-palace.example.com",
            "nationalPhoneNumber": "(801) 555-0100",
            "googleMapsUri": "https://maps.google.com/?cid=123",
            "reviews": reviews
        })
        .to_string()
    }

    #[tokio::test]
    async fn details_maps_full_payload_and_truncates_reviews_to_five() {
        let stub = spawn_stub(StatusCode::OK, &canned_details(6)).await;

        let details = provider(&stub.base_url).details("ChIJfull").await.unwrap();

        assert_eq!(details.website.as_deref(), Some("https://thai-palace.example.com"));
        assert_eq!(details.phone.as_deref(), Some("(801) 555-0100"));
        assert_eq!(details.maps_url.as_deref(), Some("https://maps.google.com/?cid=123"));
        assert_eq!(details.reviews.len(), 5, "six stub reviews must truncate to five");
        let first = &details.reviews[0];
        assert_eq!(first.author, "Reviewer 0");
        assert_eq!(first.rating, 5);
        assert_eq!(first.text, "Review text 0");
        assert_eq!(first.relative_time.as_deref(), Some("2 months ago"));

        let captured = stub.captured();
        assert_eq!(
            captured.headers.get("x-goog-api-key").map(|v| v.to_str().unwrap()),
            Some("test-key")
        );
        assert_eq!(
            captured.headers.get("x-goog-fieldmask").map(|v| v.to_str().unwrap()),
            Some("websiteUri,nationalPhoneNumber,googleMapsUri,reviews")
        );
    }

    #[tokio::test]
    async fn details_non_success_status_maps_to_unavailable() {
        let stub = spawn_stub(
            StatusCode::FORBIDDEN,
            r#"{"error":{"message":"API key not authorized"}}"#,
        )
        .await;

        let err = provider(&stub.base_url).details("ChIJfull").await.unwrap_err();

        match err {
            ProviderError::Unavailable(message) => {
                assert!(
                    message.contains("403") && message.contains("API key not authorized"),
                    "unexpected message: {message}"
                );
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn details_unparseable_body_maps_to_invalid_response() {
        let stub = spawn_stub(StatusCode::OK, "definitely not json").await;

        let err = provider(&stub.base_url).details("ChIJfull").await.unwrap_err();

        assert!(
            matches!(err, ProviderError::InvalidResponse(_)),
            "expected InvalidResponse, got {err:?}"
        );
    }
}
