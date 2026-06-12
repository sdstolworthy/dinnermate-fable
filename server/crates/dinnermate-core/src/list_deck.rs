//! Mapping from curated list items to a swipeable room deck.

use crate::model::{ListItem, Restaurant};

/// Builds a deck from list items, preserving list order.
/// Items sourced from a provider keep that restaurant id; free-form items get
/// a synthetic `list-<item-uuid>` id, which details lookups short-circuit.
/// Fields a list item cannot know (rating_count, coords, hours, utc offset)
/// stay `None`.
pub fn deck_from_items(items: &[ListItem]) -> Vec<Restaurant> {
    items
        .iter()
        .map(|item| Restaurant {
            id: item
                .source_restaurant_id
                .clone()
                .unwrap_or_else(|| format!("list-{}", item.id)),
            name: item.name.clone(),
            cuisine: item.cuisine.clone(),
            price_level: item.price_level,
            rating: item.rating,
            rating_count: None,
            address: item.address.clone().unwrap_or_default(),
            photo_url: item.photo_url.clone(),
            lat: None,
            lng: None,
            hours: None,
            utc_offset_minutes: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn item(name: &str, source_restaurant_id: Option<&str>) -> ListItem {
        ListItem {
            id: Uuid::new_v4(),
            list_id: Uuid::new_v4(),
            name: name.into(),
            cuisine: Some("thai".into()),
            price_level: Some(2),
            rating: Some(4.5),
            address: Some("123 Main St".into()),
            photo_url: Some("https://example.com/p.jpg".into()),
            added_by_user_id: Uuid::new_v4(),
            source_restaurant_id: source_restaurant_id.map(Into::into),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn reuses_source_restaurant_id_when_present() {
        let deck = deck_from_items(&[item("Curry House", Some("seed-002"))]);
        assert_eq!(deck[0].id, "seed-002");
    }

    #[test]
    fn free_form_item_gets_list_prefixed_uuid_id() {
        let it = item("Mom's Tacos", None);
        let deck = deck_from_items(std::slice::from_ref(&it));
        assert_eq!(deck[0].id, format!("list-{}", it.id));
    }

    #[test]
    fn preserves_item_order() {
        let items =
            vec![item("First", None), item("Second", Some("seed-001")), item("Third", None)];
        let deck = deck_from_items(&items);
        let names: Vec<&str> = deck.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, ["First", "Second", "Third"]);
    }

    #[test]
    fn copies_known_fields() {
        let deck = deck_from_items(&[item("Curry House", None)]);
        let got = &deck[0];
        assert_eq!(
            (
                got.name.as_str(),
                got.cuisine.as_deref(),
                got.price_level,
                got.rating,
                got.address.as_str(),
                got.photo_url.as_deref(),
            ),
            (
                "Curry House",
                Some("thai"),
                Some(2),
                Some(4.5),
                "123 Main St",
                Some("https://example.com/p.jpg"),
            )
        );
    }

    #[test]
    fn unknowable_fields_are_none() {
        let deck = deck_from_items(&[item("Curry House", None)]);
        let got = &deck[0];
        assert_eq!(
            (got.rating_count, got.lat, got.lng, got.hours.clone(), got.utc_offset_minutes),
            (None, None, None, None, None)
        );
    }

    #[test]
    fn missing_optional_fields_stay_none_and_address_empty() {
        let bare = ListItem {
            cuisine: None,
            price_level: None,
            rating: None,
            address: None,
            photo_url: None,
            ..item("Just A Name", None)
        };
        let deck = deck_from_items(&[bare]);
        let got = &deck[0];
        assert_eq!(
            (got.cuisine.clone(), got.price_level, got.rating, got.address.as_str()),
            (None, None, None, "")
        );
    }
}
