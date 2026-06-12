import 'package:dinnermate/src/api/models.dart';
import 'package:flutter_test/flutter_test.dart';

typedef RoundTripCase = ({
  String name,
  Map<String, dynamic> json,
  Map<String, dynamic> Function(Map<String, dynamic>) roundTrip,
});

const _restaurantJson = <String, dynamic>{
  'id': 'seed-001',
  'name': 'Taco Cielo',
  'cuisine': 'mexican',
  'price_level': 2,
  'rating': 4.5,
  'rating_count': 312,
  'address': '12 Main St',
  'photo_url': 'https://example.com/p.jpg',
  'lat': 40.76,
  'lng': -111.89,
};

void main() {
  final cases = <RoundTripCase>[
    (
      name: 'Restaurant',
      json: _restaurantJson,
      roundTrip: (j) => Restaurant.fromJson(j).toJson(),
    ),
    (
      name: 'Restaurant with null photo_url',
      json: {..._restaurantJson, 'photo_url': null},
      roundTrip: (j) => Restaurant.fromJson(j).toJson(),
    ),
    (
      name: 'Room',
      json: {
        'id': '6f1f7a3e-0000-4000-8000-000000000001',
        'code': 'ABC234',
        'name': 'Friday dinner',
        'location_label': 'Salt Lake City',
        'lat': 40.76,
        'lng': -111.89,
        'radius_m': 5000,
        'cuisines': ['thai', 'mexican'],
        'price_min': 1,
        'price_max': 3,
        'min_rating': 4.0,
        'created_at': '2026-06-11T10:00:00.000Z',
      },
      roundTrip: (j) => Room.fromJson(j).toJson(),
    ),
    (
      name: 'Room with null name and empty cuisines',
      json: {
        'id': '6f1f7a3e-0000-4000-8000-000000000001',
        'code': 'ABC234',
        'name': null,
        'location_label': 'Salt Lake City',
        'lat': 40.76,
        'lng': -111.89,
        'radius_m': 5000,
        'cuisines': <String>[],
        'price_min': 1,
        'price_max': 4,
        'min_rating': 0.0,
        'created_at': '2026-06-11T10:00:00.000Z',
      },
      roundTrip: (j) => Room.fromJson(j).toJson(),
    ),
    (
      name: 'Participant',
      json: {
        'id': '6f1f7a3e-0000-4000-8000-000000000002',
        'room_id': '6f1f7a3e-0000-4000-8000-000000000001',
        'user_id': '6f1f7a3e-0000-4000-8000-000000000003',
        'display_name': 'Spencer',
      },
      roundTrip: (j) => Participant.fromJson(j).toJson(),
    ),
    (
      name: 'MatchEntry',
      json: {
        'restaurant': _restaurantJson,
        'like_count': 3,
        'last_liked_at': '2026-06-11T10:05:00.000Z',
      },
      roundTrip: (j) => MatchEntry.fromJson(j).toJson(),
    ),
    (
      name: 'DinnerList',
      json: {
        'id': '6f1f7a3e-0000-4000-8000-000000000004',
        'code': 'XYZ789',
        'name': 'Date nights',
        'owner_user_id': '6f1f7a3e-0000-4000-8000-000000000003',
      },
      roundTrip: (j) => DinnerList.fromJson(j).toJson(),
    ),
    (
      name: 'ListItem',
      json: {
        'id': '6f1f7a3e-0000-4000-8000-000000000005',
        'list_id': '6f1f7a3e-0000-4000-8000-000000000004',
        'name': 'Taco Cielo',
        'cuisine': 'mexican',
        'price_level': 2,
        'rating': 4.5,
        'address': '12 Main St',
        'photo_url': 'https://example.com/p.jpg',
        'added_by_user_id': '6f1f7a3e-0000-4000-8000-000000000003',
        'source_restaurant_id': 'seed-001',
      },
      roundTrip: (j) => ListItem.fromJson(j).toJson(),
    ),
    (
      name: 'ListItem with all optional fields null',
      json: {
        'id': '6f1f7a3e-0000-4000-8000-000000000005',
        'list_id': '6f1f7a3e-0000-4000-8000-000000000004',
        'name': 'That ramen place downtown',
        'cuisine': null,
        'price_level': null,
        'rating': null,
        'address': null,
        'photo_url': null,
        'added_by_user_id': '6f1f7a3e-0000-4000-8000-000000000003',
        'source_restaurant_id': null,
      },
      roundTrip: (j) => ListItem.fromJson(j).toJson(),
    ),
  ];

  group('JSON roundtrip', () {
    for (final c in cases) {
      test(c.name, () => expect(c.roundTrip(c.json), equals(c.json)));
    }
  });

  group('Request serialization', () {
    test('CreateRoomRequest omits null name and uses snake_case keys', () {
      const request = CreateRoomRequest(
        locationLabel: 'Salt Lake City',
        lat: 40.76,
        lng: -111.89,
        radiusM: 5000,
        cuisines: ['thai'],
        priceMin: 1,
        priceMax: 3,
        minRating: 4.0,
      );
      expect(request.toJson(), {
        'location_label': 'Salt Lake City',
        'lat': 40.76,
        'lng': -111.89,
        'radius_m': 5000,
        'cuisines': ['thai'],
        'price_min': 1,
        'price_max': 3,
        'min_rating': 4.0,
      });
    });

    test('NewListItem omits unset optional fields', () {
      const item = NewListItem(name: 'Pho 88', cuisine: 'vietnamese');
      expect(item.toJson(), {'name': 'Pho 88', 'cuisine': 'vietnamese'});
    });
  });
}
