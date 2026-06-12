use chrono::{DateTime, Duration, Utc};
use dinnermate_core::{
    List, ListItem, ListRepo, RepoError, Restaurant, Room, RoomParams, RoomRepo,
};
use dinnermate_db::{connect_and_migrate, PgListRepo, PgRoomRepo};
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
        cuisine: "thai".into(),
        price_level: 2,
        rating: 4.2,
        rating_count: 137,
        address: "123 Main St".into(),
        photo_url: Some(format!("https://example.com/{id}.jpg")),
        lat: 40.7601,
        lng: -111.8902,
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
        },
        created_by: Uuid::new_v4(),
        created_at: now_micros(),
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
async fn lists_for_owner_returns_only_owned_ordered_desc() {
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

    let lists = repo.lists_for_owner(owner).await.expect("lists_for_owner");
    assert_eq!(lists, vec![newer, older]);
}
