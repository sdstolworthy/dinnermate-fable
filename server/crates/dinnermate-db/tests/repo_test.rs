use chrono::{DateTime, Duration, Utc};
use dinnermate_core::{
    DetailsCacheRepo, HoursPeriod, List, ListItem, ListRepo, ProviderDetails, RepoError,
    Restaurant, Review, Room, RoomParams, RoomRepo,
};
use dinnermate_db::{connect_and_migrate, PgDetailsCacheRepo, PgListRepo, PgRoomRepo};
use sqlx::PgPool;
use uuid::Uuid;

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL is not set — run these tests via server/scripts/test-db.sh, \
         which starts a disposable postgres and sets the variable",
    );
    connect_and_migrate(&url)
        .await
        .expect("connect and migrate test database")
}

/// Postgres TIMESTAMPTZ stores microseconds; truncate so roundtrip equality holds.
fn now_micros() -> DateTime<Utc> {
    DateTime::from_timestamp_micros(Utc::now().timestamp_micros()).unwrap()
}

fn unique_code() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_uppercase()
}

fn restaurant(id: &str, name: &str) -> Restaurant {
    Restaurant {
        id: id.into(),
        name: name.into(),
        cuisine: Some("thai".into()),
        price_level: Some(2),
        rating: Some(4.2),
        rating_count: Some(137),
        address: "123 Main St".into(),
        photo_url: Some(format!("https://example.com/{id}.jpg")),
        lat: Some(40.7601),
        lng: Some(-111.8902),
        hours: None,
        utc_offset_minutes: None,
    }
}

fn room(code: &str) -> Room {
    Room {
        id: Uuid::new_v4(),
        code: code.into(),
        name: Some("Friday dinner".into()),
        params: RoomParams {
            lat: 40.76,
            lng: -111.89,
            location_label: "Downtown SLC".into(),
            radius_m: 5_000,
            cuisines: vec!["thai".into(), "mexican".into()],
            price_min: 1,
            price_max: 3,
            min_rating: 3.5,
            eat_at_utc: None,
        },
        created_by: Uuid::new_v4(),
        created_at: now_micros(),
        source_list_name: None,
    }
}

fn list(code: &str, owner: Uuid, created_at: DateTime<Utc>) -> List {
    List {
        id: Uuid::new_v4(),
        code: code.into(),
        name: "Favorites".into(),
        owner_user_id: owner,
        created_at,
    }
}

fn list_item(list_id: Uuid, name: &str, created_at: DateTime<Utc>) -> ListItem {
    ListItem {
        id: Uuid::new_v4(),
        list_id,
        name: name.into(),
        cuisine: Some("thai".into()),
        price_level: Some(2),
        rating: Some(4.4),
        address: Some("456 Side St".into()),
        photo_url: None,
        added_by_user_id: Uuid::new_v4(),
        source_restaurant_id: Some("seed-001".into()),
        created_at,
    }
}

#[tokio::test]
async fn room_create_and_find_by_code_roundtrip_preserves_deck_order() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    // Insertion order deliberately differs from alphabetical id/name order
    // so an ordering bug cannot pass by accident.
    let deck = vec![
        restaurant("r-charlie", "Charlie's Curry"),
        restaurant("r-alpha", "Alpha Thai"),
        restaurant("r-bravo", "Bravo Bowls"),
    ];

    repo.create(&room, &deck).await.expect("create room");
    let (found_room, found_deck) = repo
        .find_by_code(&room.code)
        .await
        .expect("find_by_code")
        .expect("room should exist");

    assert_eq!(found_room, room);
    assert_eq!(found_deck, deck);
}

#[tokio::test]
async fn room_eat_at_roundtrips_some_and_none() {
    let repo = PgRoomRepo::new(pool().await);
    let eat_at: DateTime<Utc> = "2026-06-13T01:00:00Z".parse().unwrap();

    for (name, value) in [("set", Some(eat_at)), ("unset", None)] {
        let mut room = room(&unique_code());
        room.params.eat_at_utc = value;

        repo.create(&room, &[restaurant("r-1", "One")]).await.expect("create room");
        let (found, _) = repo
            .find_by_code(&room.code)
            .await
            .expect("find_by_code")
            .expect("room should exist");

        assert_eq!(found.params.eat_at_utc, value, "eat_at {name}");
    }
}

#[tokio::test]
async fn find_by_code_unknown_returns_none() {
    let repo = PgRoomRepo::new(pool().await);
    let result = repo.find_by_code("ZZZZ99").await.expect("find_by_code");
    assert!(result.is_none());
}

#[tokio::test]
async fn join_then_find_participant_and_duplicate_join_conflicts() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    repo.create(&room, &[restaurant("r-1", "One")])
        .await
        .expect("create room");
    let user = Uuid::new_v4();

    let joined = repo.join(room.id, user, "Alice").await.expect("join");
    let found = repo
        .find_participant(room.id, user)
        .await
        .expect("find_participant")
        .expect("participant should exist");
    assert_eq!(found, joined);

    let err = repo
        .join(room.id, user, "Alice again")
        .await
        .expect_err("second join for same (room, user) should fail");
    assert!(matches!(err, RepoError::Conflict), "got {err:?}");
}

#[tokio::test]
async fn record_swipe_duplicate_returns_conflict() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    repo.create(&room, &[restaurant("r-1", "One")])
        .await
        .expect("create room");
    let participant = repo
        .join(room.id, Uuid::new_v4(), "Bob")
        .await
        .expect("join");

    repo.record_swipe(room.id, participant.id, "r-1", true)
        .await
        .expect("first swipe");
    let err = repo
        .record_swipe(room.id, participant.id, "r-1", false)
        .await
        .expect_err("duplicate swipe should fail");
    assert!(matches!(err, RepoError::Conflict), "got {err:?}");
}

#[tokio::test]
async fn matches_orders_by_like_count_and_excludes_unliked() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    let deck = vec![
        restaurant("r-1", "One"),
        restaurant("r-2", "Two"),
        restaurant("r-3", "Three"),
    ];
    repo.create(&room, &deck).await.expect("create room");

    let mut participants = Vec::new();
    for name in ["Alice", "Bob", "Carol"] {
        participants.push(
            repo.join(room.id, Uuid::new_v4(), name)
                .await
                .expect("join"),
        );
    }

    for p in &participants {
        repo.record_swipe(room.id, p.id, "r-1", true)
            .await
            .expect("like r-1");
    }
    repo.record_swipe(room.id, participants[0].id, "r-2", true)
        .await
        .expect("like r-2");
    repo.record_swipe(room.id, participants[0].id, "r-3", false)
        .await
        .expect("dislike r-3");

    let matches = repo.matches(room.id).await.expect("matches");
    let summary: Vec<(&str, i64)> = matches
        .iter()
        .map(|m| (m.restaurant.id.as_str(), m.like_count))
        .collect();
    assert_eq!(summary, vec![("r-1", 3), ("r-2", 1)]);
    assert_eq!(matches[0].restaurant, deck[0]);

    let count = repo
        .participant_count(room.id)
        .await
        .expect("participant_count");
    assert_eq!(count, 3);
}

#[tokio::test]
async fn list_create_and_find_by_code_roundtrip_with_items_ordered_asc() {
    let repo = PgListRepo::new(pool().await);
    let list = list(&unique_code(), Uuid::new_v4(), now_micros());
    repo.create(&list).await.expect("create list");

    let base = now_micros();
    let older = list_item(list.id, "Older item", base - Duration::seconds(10));
    let newer = list_item(list.id, "Newer item", base);
    // Insert newest first to prove ordering comes from created_at, not insertion.
    repo.add_item(&newer).await.expect("add newer");
    repo.add_item(&older).await.expect("add older");

    let (found_list, items) = repo
        .find_by_code(&list.code)
        .await
        .expect("find_by_code")
        .expect("list should exist");

    assert_eq!(found_list, list);
    assert_eq!(items, vec![older, newer]);
}

#[tokio::test]
async fn list_find_by_code_unknown_returns_none() {
    let repo = PgListRepo::new(pool().await);
    let result = repo.find_by_code("ZZZZ98").await.expect("find_by_code");
    assert!(result.is_none());
}

#[tokio::test]
async fn lists_for_member_returns_only_own_lists_ordered_desc() {
    let repo = PgListRepo::new(pool().await);
    let owner = Uuid::new_v4();
    let other_owner = Uuid::new_v4();

    let base = now_micros();
    let older = list(&unique_code(), owner, base - Duration::seconds(10));
    let newer = list(&unique_code(), owner, base);
    let foreign = list(&unique_code(), other_owner, base);
    repo.create(&older).await.expect("create older");
    repo.create(&newer).await.expect("create newer");
    repo.create(&foreign).await.expect("create foreign");

    let memberships = repo.lists_for_member(owner).await.expect("lists_for_member");
    let lists: Vec<_> = memberships.iter().map(|m| m.list.clone()).collect();
    assert_eq!(lists, vec![newer, older]);
    assert!(memberships.iter().all(|m| m.is_owner));
}

#[tokio::test]
async fn list_create_makes_owner_a_member_immediately() {
    let repo = PgListRepo::new(pool().await);
    let owner = Uuid::new_v4();
    let list = list(&unique_code(), owner, now_micros());
    repo.create(&list).await.expect("create list");

    assert!(repo.is_member(list.id, owner).await.expect("is_member"));
    let memberships = repo.lists_for_member(owner).await.expect("lists_for_member");
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].list, list);
    assert!(memberships[0].is_owner);
}

#[tokio::test]
async fn list_join_is_idempotent_and_grants_non_owner_membership() {
    let repo = PgListRepo::new(pool().await);
    let list = list(&unique_code(), Uuid::new_v4(), now_micros());
    repo.create(&list).await.expect("create list");
    let joiner = Uuid::new_v4();

    repo.join(list.id, joiner).await.expect("first join");
    repo.join(list.id, joiner).await.expect("second join is a no-op");

    assert!(repo.is_member(list.id, joiner).await.expect("is_member"));
    let memberships = repo.lists_for_member(joiner).await.expect("lists_for_member");
    assert_eq!(memberships.len(), 1);
    assert_eq!(memberships[0].list, list);
    assert!(!memberships[0].is_owner);
}

#[tokio::test]
async fn list_leave_removes_membership() {
    let repo = PgListRepo::new(pool().await);
    let list = list(&unique_code(), Uuid::new_v4(), now_micros());
    repo.create(&list).await.expect("create list");
    let joiner = Uuid::new_v4();

    repo.join(list.id, joiner).await.expect("join");
    repo.leave(list.id, joiner).await.expect("leave");

    assert!(!repo.is_member(list.id, joiner).await.expect("is_member"));
    assert!(repo
        .lists_for_member(joiner)
        .await
        .expect("lists_for_member")
        .is_empty());
}

#[tokio::test]
async fn lists_for_member_orders_by_joined_at_desc_with_ownership_flags() {
    let repo = PgListRepo::new(pool().await);
    let user = Uuid::new_v4();

    // Own list created (and thus joined) in the past; foreign list joined now,
    // so the foreign membership is the most recent.
    let own = list(&unique_code(), user, now_micros() - Duration::seconds(10));
    let foreign = list(&unique_code(), Uuid::new_v4(), now_micros());
    repo.create(&own).await.expect("create own");
    repo.create(&foreign).await.expect("create foreign");
    repo.join(foreign.id, user).await.expect("join foreign");

    let memberships = repo.lists_for_member(user).await.expect("lists_for_member");
    let summary: Vec<(Uuid, bool)> = memberships.iter().map(|m| (m.list.id, m.is_owner)).collect();
    assert_eq!(summary, vec![(foreign.id, false), (own.id, true)]);
}

fn hours() -> Vec<HoursPeriod> {
    vec![
        HoursPeriod { day: 1, open: "11:00".into(), close: "14:00".into() },
        HoursPeriod { day: 5, open: "17:00".into(), close: "01:00".into() },
    ]
}

#[tokio::test]
async fn room_deck_roundtrips_hours_and_utc_offset_through_find_by_code() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    let with_hours = Restaurant {
        hours: Some(hours()),
        utc_offset_minutes: Some(-360),
        ..restaurant("r-hours", "Hourly House")
    };
    let without_hours = restaurant("r-nohours", "Mystery Meals");
    let deck = vec![with_hours, without_hours];

    repo.create(&room, &deck).await.expect("create room");
    let (_, found_deck) = repo
        .find_by_code(&room.code)
        .await
        .expect("find_by_code")
        .expect("room should exist");

    assert_eq!(found_deck, deck);
}

#[tokio::test]
async fn matches_carry_hours_and_utc_offset() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    let liked = Restaurant {
        hours: Some(hours()),
        utc_offset_minutes: Some(120),
        ..restaurant("r-liked", "Liked Lounge")
    };
    repo.create(&room, std::slice::from_ref(&liked))
        .await
        .expect("create room");
    let participant = repo
        .join(room.id, Uuid::new_v4(), "Dana")
        .await
        .expect("join");
    repo.record_swipe(room.id, participant.id, "r-liked", true)
        .await
        .expect("swipe");

    let matches = repo.matches(room.id).await.expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].restaurant, liked);
}

/// A restaurant with only the always-present fields: id and name.
/// Everything optional is None and address is "" (the model keeps String).
fn minimal_restaurant(id: &str, name: &str) -> Restaurant {
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

#[tokio::test]
async fn all_none_restaurant_roundtrips_and_appears_in_matches() {
    let repo = PgRoomRepo::new(pool().await);
    let room = room(&unique_code());
    let minimal = minimal_restaurant("list-abc", "Mystery Spot");

    repo.create(&room, std::slice::from_ref(&minimal))
        .await
        .expect("create room");
    let (_, found_deck) = repo
        .find_by_code(&room.code)
        .await
        .expect("find_by_code")
        .expect("room should exist");
    assert_eq!(found_deck, vec![minimal.clone()]);

    let participant = repo
        .join(room.id, Uuid::new_v4(), "Eve")
        .await
        .expect("join");
    repo.record_swipe(room.id, participant.id, "list-abc", true)
        .await
        .expect("swipe");
    let matches = repo.matches(room.id).await.expect("matches");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].restaurant, minimal);
}

#[tokio::test]
async fn null_address_in_db_reads_back_as_empty_string() {
    let db = pool().await;
    let repo = PgRoomRepo::new(db.clone());
    let room = room(&unique_code());
    repo.create(&room, &[restaurant("r-null-addr", "No Fixed Abode")])
        .await
        .expect("create room");

    sqlx::query("UPDATE room_restaurants SET address = NULL WHERE room_id = $1")
        .bind(room.id)
        .execute(&db)
        .await
        .expect("null out address");

    let (_, deck) = repo
        .find_by_code(&room.code)
        .await
        .expect("find_by_code")
        .expect("room should exist");
    assert_eq!(deck[0].address, "");
}

#[tokio::test]
async fn room_source_list_name_roundtrips() {
    let repo = PgRoomRepo::new(pool().await);
    let with_list = Room {
        source_list_name: Some("Date Night Favorites".into()),
        ..room(&unique_code())
    };

    repo.create(&with_list, &[restaurant("r-1", "One")])
        .await
        .expect("create room");
    let (found, _) = repo
        .find_by_code(&with_list.code)
        .await
        .expect("find_by_code")
        .expect("room should exist");

    assert_eq!(found.source_list_name, Some("Date Night Favorites".into()));
    assert_eq!(found, with_list);
}

#[tokio::test]
async fn delete_older_than_removes_expired_rooms_and_cascades() {
    let db = pool().await;
    let repo = PgRoomRepo::new(db.clone());

    let old_room = room(&unique_code());
    repo.create(&old_room, &[restaurant("r-old", "Old Place")])
        .await
        .expect("create old room");
    let old_participant = repo
        .join(old_room.id, Uuid::new_v4(), "Olive")
        .await
        .expect("join old");
    repo.record_swipe(old_room.id, old_participant.id, "r-old", true)
        .await
        .expect("swipe old");

    let fresh_room = room(&unique_code());
    repo.create(&fresh_room, &[restaurant("r-fresh", "Fresh Place")])
        .await
        .expect("create fresh room");
    let fresh_participant = repo
        .join(fresh_room.id, Uuid::new_v4(), "Fern")
        .await
        .expect("join fresh");
    repo.record_swipe(fresh_room.id, fresh_participant.id, "r-fresh", true)
        .await
        .expect("swipe fresh");

    sqlx::query("UPDATE rooms SET created_at = now() - interval '31 days' WHERE id = $1")
        .bind(old_room.id)
        .execute(&db)
        .await
        .expect("backdate old room");

    let deleted = repo
        .delete_older_than(Utc::now() - Duration::days(30))
        .await
        .expect("delete_older_than");
    assert_eq!(deleted, 1);

    assert!(repo
        .find_by_code(&old_room.code)
        .await
        .expect("find old")
        .is_none());
    for (table, count_expected) in [
        ("room_restaurants", 0i64),
        ("participants", 0),
        ("swipes", 0),
    ] {
        let count: i64 =
            sqlx::query_scalar(&format!("SELECT count(*) FROM {table} WHERE room_id = $1"))
                .bind(old_room.id)
                .fetch_one(&db)
                .await
                .expect("count cascaded rows");
        assert_eq!(count, count_expected, "{table} should be empty");
    }

    let (found_fresh, fresh_deck) = repo
        .find_by_code(&fresh_room.code)
        .await
        .expect("find fresh")
        .expect("fresh room intact");
    assert_eq!(found_fresh, fresh_room);
    assert_eq!(fresh_deck.len(), 1);
    let fresh_matches = repo.matches(fresh_room.id).await.expect("fresh matches");
    assert_eq!(fresh_matches.len(), 1);
    assert_eq!(
        repo.participant_count(fresh_room.id)
            .await
            .expect("participant_count"),
        1
    );
}

#[tokio::test]
async fn participants_returned_in_joined_at_order() {
    let db = pool().await;
    let repo = PgRoomRepo::new(db.clone());
    let room = room(&unique_code());
    repo.create(&room, &[restaurant("r-1", "One")])
        .await
        .expect("create room");

    // Bob joins first; Alice is then backdated to before Bob so the expected
    // order can only come from joined_at, not insertion order.
    repo.join(room.id, Uuid::new_v4(), "Bob").await.expect("join Bob");
    let alice = repo
        .join(room.id, Uuid::new_v4(), "Alice")
        .await
        .expect("join Alice");
    sqlx::query("UPDATE participants SET joined_at = joined_at - interval '1 hour' WHERE id = $1")
        .bind(alice.id)
        .execute(&db)
        .await
        .expect("backdate Alice");

    let participants = repo.participants(room.id).await.expect("participants");
    let names: Vec<&str> = participants.iter().map(|p| p.display_name.as_str()).collect();
    assert_eq!(names, vec!["Alice", "Bob"]);
}

fn provider_details(website: &str) -> ProviderDetails {
    ProviderDetails {
        website: Some(website.into()),
        phone: Some("+1 801-555-0123".into()),
        maps_url: Some("https://maps.example.com/p".into()),
        reviews: vec![Review {
            author: "Ada".into(),
            rating: 5,
            text: "Great noodles".into(),
            relative_time: Some("2 weeks ago".into()),
        }],
    }
}

#[tokio::test]
async fn details_cache_get_miss_returns_none() {
    let repo = PgDetailsCacheRepo::new(pool().await);
    let hit = repo.get("seed-never-cached").await.expect("get");
    assert!(hit.is_none());
}

#[tokio::test]
async fn details_cache_put_then_get_roundtrips_with_recent_fetched_at() {
    let repo = PgDetailsCacheRepo::new(pool().await);
    let id = format!("seed-{}", unique_code());
    let details = provider_details("https://noodles.example.com");

    let before = Utc::now() - Duration::seconds(30);
    repo.put(&id, &details).await.expect("put");
    let after = Utc::now() + Duration::seconds(30);

    let (cached, fetched_at) = repo.get(&id).await.expect("get").expect("cache hit");
    assert_eq!(cached, details);
    assert!(
        fetched_at > before && fetched_at < after,
        "fetched_at {fetched_at} not within sane bounds"
    );
}

#[tokio::test]
async fn details_cache_put_twice_updates_payload() {
    let repo = PgDetailsCacheRepo::new(pool().await);
    let id = format!("seed-{}", unique_code());

    repo.put(&id, &provider_details("https://old.example.com"))
        .await
        .expect("first put");
    let updated = provider_details("https://new.example.com");
    repo.put(&id, &updated).await.expect("second put");

    let (cached, _) = repo.get(&id).await.expect("get").expect("cache hit");
    assert_eq!(cached, updated);
}
