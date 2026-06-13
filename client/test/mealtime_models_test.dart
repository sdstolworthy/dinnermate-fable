import 'package:dinnermate/src/api/models.dart';
import 'package:flutter_test/flutter_test.dart';

const _roomJson = <String, dynamic>{
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
};

const _request = CreateRoomRequest(
  locationLabel: 'Salt Lake City',
  lat: 40.76,
  lng: -111.89,
  radiusM: 5000,
  priceMin: 1,
  priceMax: 4,
  minRating: 0.0,
);

void main() {
  group('Room.eatAt', () {
    test('roundtrips when set', () {
      final json = {..._roomJson, 'eat_at': '2026-06-13T01:00:00.000Z'};

      expect(Room.fromJson(json).toJson(), equals(json));
    });

    test('parses to a UTC instant', () {
      final room =
          Room.fromJson({..._roomJson, 'eat_at': '2026-06-13T01:00:00.000Z'});

      expect(room.eatAt, DateTime.utc(2026, 6, 13, 1));
    });

    test('is null when the key is null', () {
      expect(Room.fromJson({..._roomJson, 'eat_at': null}).eatAt, isNull);
    });

    test('is null when the key is absent (pre-v4 servers)', () {
      expect(Room.fromJson(_roomJson).eatAt, isNull);
    });

    test('toJson omits eat_at when unset', () {
      expect(Room.fromJson(_roomJson).toJson(), isNot(contains('eat_at')));
    });
  });

  group('CreateRoomRequest.eatAt', () {
    test('emits eat_at as UTC ISO8601 when set', () {
      final request = CreateRoomRequest(
        locationLabel: 'Salt Lake City',
        lat: 40.76,
        lng: -111.89,
        radiusM: 5000,
        priceMin: 1,
        priceMax: 4,
        minRating: 0.0,
        eatAt: DateTime.utc(2026, 6, 13, 1),
      );

      expect(request.toJson()['eat_at'], '2026-06-13T01:00:00.000Z');
    });

    test('converts a local-zone instant to UTC on the wire', () {
      final request = CreateRoomRequest(
        locationLabel: 'Salt Lake City',
        lat: 40.76,
        lng: -111.89,
        radiusM: 5000,
        priceMin: 1,
        priceMax: 4,
        minRating: 0.0,
        eatAt: DateTime.utc(2026, 6, 13, 1).toLocal(),
      );

      expect(request.toJson()['eat_at'], '2026-06-13T01:00:00.000Z');
    });

    test('omits eat_at when null', () {
      expect(_request.toJson(), isNot(contains('eat_at')));
    });
  });
}
