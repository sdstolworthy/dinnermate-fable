import 'dart:convert';

import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

const _restaurantJson = <String, dynamic>{
  'id': 'seed-001',
  'name': 'Taco Cielo',
  'cuisine': 'mexican',
  'price_level': 2,
  'rating': 4.5,
  'rating_count': 312,
  'address': '12 Main St',
  'photo_url': null,
  'lat': 40.76,
  'lng': -111.89,
};

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

const _createRoomRequest = CreateRoomRequest(
  locationLabel: 'Salt Lake City',
  lat: 40.76,
  lng: -111.89,
  radiusM: 5000,
  priceMin: 1,
  priceMax: 4,
  minRating: 0.0,
);

ApiClient _client(MockClient httpClient, {KeyValueStore? store}) =>
    ApiClient('http://api.test/api/v1', httpClient,
        Identity(store ?? InMemoryStore()));

void main() {
  test('sends X-Dinnermate-User header with the persisted identity', () async {
    final store = InMemoryStore();
    await store.write('dinnermate_user_id', 'user-123');
    late http.Request captured;
    final mock = MockClient((request) async {
      captured = request;
      return http.Response(jsonEncode({'matches': [], 'participant_count': 0}),
          200);
    });

    await _client(mock, store: store).getMatches('ABC234');

    expect(captured.headers['X-Dinnermate-User'], 'user-123');
  });

  test('createRoom parses room and deck from a 201 response', () async {
    final mock = MockClient((request) async {
      expect(request.method, 'POST');
      expect(request.url.path, '/api/v1/rooms');
      expect(jsonDecode(request.body), _createRoomRequest.toJson());
      return http.Response(
          jsonEncode({
            'room': _roomJson,
            'deck': [_restaurantJson],
          }),
          201);
    });

    final (room, deck) = await _client(mock).createRoom(_createRoomRequest);

    expect(room.code, 'ABC234');
    expect(deck, hasLength(1));
    expect(deck.single.id, 'seed-001');
  });

  test('getMatches parses entries and participant_count', () async {
    final mock = MockClient((request) async {
      expect(request.url.path, '/api/v1/rooms/ABC234/matches');
      return http.Response(
          jsonEncode({
            'matches': [
              {
                'restaurant': _restaurantJson,
                'like_count': 3,
                'last_liked_at': '2026-06-11T10:05:00.000Z',
              },
            ],
            'participant_count': 2,
          }),
          200);
    });

    final result = await _client(mock).getMatches('ABC234');

    expect(result.participantCount, 2);
    expect(result.entries.single.likeCount, 3);
    expect(result.entries.single.restaurant.name, 'Taco Cielo');
  });

  test('404 with error envelope throws ApiException with server code',
      () async {
    final mock = MockClient((request) async => http.Response(
        jsonEncode({
          'error': {'code': 'ROOM_NOT_FOUND', 'message': 'no such room'}
        }),
        404));

    expect(
      () => _client(mock).getRoom('NOPE99'),
      throwsA(isA<ApiException>()
          .having((e) => e.code, 'code', 'ROOM_NOT_FOUND')
          .having((e) => e.message, 'message', 'no such room')
          .having((e) => e.status, 'status', 404)),
    );
  });

  test('malformed error body falls back to status-based ApiException',
      () async {
    final mock =
        MockClient((request) async => http.Response('<html>oops</html>', 500));

    expect(
      () => _client(mock).getMyLists(),
      throwsA(isA<ApiException>()
          .having((e) => e.code, 'code', 'HTTP_500')
          .having((e) => e.status, 'status', 500)
          .having((e) => e.message, 'message', isNotEmpty)),
    );
  });
}
