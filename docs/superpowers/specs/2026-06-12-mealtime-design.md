# Meal-time selection + OSM timezones — Design

**Date:** 2026-06-12 · **Mode:** delegated (FUTURE.md items picked per Spencer's standing instruction). Implements two FUTURE.md entries together because the second is a no-op without the first.

## 1. Timezone offsets for OSM restaurants

OSM provides hours but no timezone, so `utc_offset_minutes` is `None` → no live open/closed badge and no fair open-at-time filtering. Fix: resolve lat/lng → IANA tz → offset.

- Deps (api crate only; core stays dependency-light): `tzf-rs` (lat/lng → tz name, pure Rust, embedded data) + `chrono-tz` (tz name → offset at an instant).
- New `dinnermate-api/src/tz.rs`: `pub fn utc_offset_minutes(lat: f64, lng: f64, at: DateTime<Utc>) -> Option<i32>` — `OnceLock<DefaultFinder>` (built once, ~lazy); tz name parse failures → `None`.
- `osm.rs` mapping sets `utc_offset_minutes` for every entry with coords, evaluated at search time. **[DECISION]** The stored offset is a snapshot (DST boundary mid-room-life shifts it by an hour); rooms are short-lived (30-day cap, used same-day), accepted.
- Google/seed unchanged (both already provide offsets).
- Side effect: OSM cards regain the live "Open · until …" badge.

## 2. Meal-time selection ("When are you eating?")

- `RoomParams` gains `eat_at_utc: Option<DateTime<Utc>>` (serde default; no range validation — YAGNI).
- `filter::apply`: when `eat_at_utc` is `Some`, exclude restaurants whose `open_status(hours, offset, eat_at)` is `Closed`; `Open` and `Unknown` both pass (**fairness rule**: never punish missing data).
- Wire: `CreateRoomRequest`/`RoomDto` gain `eat_at` (ISO8601 UTC, nullable). Migration `0004`: `rooms ADD COLUMN eat_at TIMESTAMPTZ`.
- From-list rooms: no meal-time (curated decks aren't filtered) — field stays null.
- Client (create room form, between location and radius): "When are you eating?" SegmentedButton — **Anytime** (default, no filter) / **Tonight** (today 19:00 local → UTC) / **Pick a time** (showTimePicker, today at chosen local time → UTC; picking a time earlier than now rolls to tomorrow). Room screen shows "🕖 Eating at 7:00 PM" tag when set (rendered in the *user's* local time).
- Matches/swipe behavior unchanged; the card badge keeps showing *current* open status (the deck was already filtered for the meal time).

## Testing

Core: filter eat_at table (closed-at-time excluded; open passes; unknown hours passes; unknown offset passes; eat_at None = no exclusion; midnight-crossing span at eat_at). API: tz.rs known coords (SLC → America/Denver, June → -360; London +60 BST; mid-ocean → None acceptable as Some/None just don't panic — assert SLC/London exact); integration: create room with eat_at → 201 echoes eat_at, room roundtrips; osm stub mapping now yields Some offset. DB: eat_at roundtrip. Client: when-picker modes produce expected UTC instants (fixed clock), tag renders, anytime sends null. Smoke: create room with eat_at, assert echo.
