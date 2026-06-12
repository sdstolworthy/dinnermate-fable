//! Google Places (New) `RestaurantProvider` backed by `places:searchNearby`.

use async_trait::async_trait;
use dinnermate_core::{ProviderError, Restaurant, RestaurantProvider, RoomParams};
use serde::Deserialize;
use serde_json::json;

/// Production endpoint; tests inject a stub server's URL instead.
pub const GOOGLE_PLACES_BASE_URL: &str = "https://places.googleapis.com";

const FIELD_MASK: &str = "places.id,places.displayName,places.formattedAddress,places.rating,\
                          places.userRatingCount,places.priceLevel,places.location,\
                          places.primaryType,places.photos";

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
        Restaurant {
            id: place.id,
            name: place.display_name.text,
            cuisine,
            price_level: price_level_from_enum(place.price_level.as_deref()),
            rating: place.rating.unwrap_or(0.0),
            rating_count: place.user_rating_count.unwrap_or(0),
            address: place.formatted_address.unwrap_or_default(),
            photo_url,
            lat: place.location.latitude,
            lng: place.location.longitude,
        }
    }
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

        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|err| ProviderError::Unavailable(err.to_string()))?;
        if !status.is_success() {
            let excerpt: String = text.chars().take(200).collect();
            return Err(ProviderError::Unavailable(format!("status {status}: {excerpt}")));
        }

        let parsed: SearchResponse = serde_json::from_str(&text)
            .map_err(|err| ProviderError::InvalidResponse(err.to_string()))?;
        Ok(parsed
            .places
            .into_iter()
            .map(|place| self.to_restaurant(place))
            .collect())
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
    use axum::routing::post;
    use axum::Router;
    use dinnermate_core::{ProviderError, RestaurantProvider, RoomParams};
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
        assert_eq!(full.cuisine, "thai");
        assert_eq!(full.price_level, 3);
        assert_eq!(full.rating, 4.5);
        assert_eq!(full.rating_count, 321);
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
        assert_eq!((full.lat, full.lng), (40.761, -111.891));

        let sparse = &restaurants[1];
        assert_eq!(sparse.id, "ChIJsparse");
        assert_eq!(sparse.name, "Mystery Diner");
        assert_eq!(sparse.cuisine, "restaurant");
        assert_eq!(sparse.price_level, 2);
        assert_eq!(sparse.rating, 0.0);
        assert_eq!(sparse.rating_count, 0);
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
                 places.primaryType,places.photos"
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
}
