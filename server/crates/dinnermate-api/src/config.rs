use thiserror::Error;

#[derive(Debug, Error)]
#[error("config error: {0}")]
pub struct ConfigError(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestaurantProviderKind {
    Seed,
    Google,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub provider: RestaurantProviderKind,
    pub google_places_api_key: Option<String>,
    pub cors_allowed_origins: String,
}

impl Config {
    /// Reads configuration through an injected getter (`std::env::var` in
    /// production, a map lookup in tests) so tests never touch the process
    /// environment.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let database_url = get("DATABASE_URL")
            .ok_or_else(|| ConfigError("DATABASE_URL is required".to_string()))?;
        let bind_addr = get("BIND_ADDR").unwrap_or_else(|| "0.0.0.0:8080".to_string());
        let provider = match get("RESTAURANT_PROVIDER").as_deref() {
            None | Some("seed") => RestaurantProviderKind::Seed,
            Some("google") => RestaurantProviderKind::Google,
            Some(other) => {
                return Err(ConfigError(format!(
                    "unknown RESTAURANT_PROVIDER {other:?} (expected \"seed\" or \"google\")"
                )))
            }
        };
        let google_places_api_key = get("GOOGLE_PLACES_API_KEY");
        if provider == RestaurantProviderKind::Google && google_places_api_key.is_none() {
            return Err(ConfigError(
                "GOOGLE_PLACES_API_KEY is required when RESTAURANT_PROVIDER=google".to_string(),
            ));
        }
        let cors_allowed_origins = get("CORS_ALLOWED_ORIGINS").unwrap_or_else(|| "*".to_string());
        Ok(Config {
            database_url,
            bind_addr,
            provider,
            google_places_api_key,
            cors_allowed_origins,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn getter(vars: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = vars
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |key| map.get(key).cloned()
    }

    #[test]
    fn applies_defaults_when_only_database_url_is_set() {
        let config = Config::from_env(getter(&[("DATABASE_URL", "postgres://x")])).unwrap();
        assert_eq!(
            (
                config.database_url.as_str(),
                config.bind_addr.as_str(),
                config.provider,
                config.google_places_api_key,
                config.cors_allowed_origins.as_str(),
            ),
            ("postgres://x", "0.0.0.0:8080", RestaurantProviderKind::Seed, None, "*"),
        );
    }

    #[test]
    fn missing_database_url_is_an_error() {
        let err = Config::from_env(getter(&[])).unwrap_err();
        assert!(err.to_string().contains("DATABASE_URL"), "got {err}");
    }

    #[test]
    fn google_provider_without_api_key_is_an_error() {
        let err = Config::from_env(getter(&[
            ("DATABASE_URL", "postgres://x"),
            ("RESTAURANT_PROVIDER", "google"),
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("GOOGLE_PLACES_API_KEY"), "got {err}");
    }

    #[test]
    fn google_provider_with_api_key_is_accepted() {
        let config = Config::from_env(getter(&[
            ("DATABASE_URL", "postgres://x"),
            ("RESTAURANT_PROVIDER", "google"),
            ("GOOGLE_PLACES_API_KEY", "secret"),
        ]))
        .unwrap();
        assert_eq!(
            (config.provider, config.google_places_api_key.as_deref()),
            (RestaurantProviderKind::Google, Some("secret")),
        );
    }

    #[test]
    fn unknown_provider_is_an_error() {
        let err = Config::from_env(getter(&[
            ("DATABASE_URL", "postgres://x"),
            ("RESTAURANT_PROVIDER", "yelp"),
        ]))
        .unwrap_err();
        assert!(err.to_string().contains("RESTAURANT_PROVIDER"), "got {err}");
    }

    #[test]
    fn explicit_overrides_are_honored() {
        let config = Config::from_env(getter(&[
            ("DATABASE_URL", "postgres://x"),
            ("BIND_ADDR", "127.0.0.1:9999"),
            ("CORS_ALLOWED_ORIGINS", "https://a.example,https://b.example"),
        ]))
        .unwrap();
        assert_eq!(
            (config.bind_addr.as_str(), config.cors_allowed_origins.as_str()),
            ("127.0.0.1:9999", "https://a.example,https://b.example"),
        );
    }
}
