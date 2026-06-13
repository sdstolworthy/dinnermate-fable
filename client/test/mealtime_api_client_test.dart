import 'dart:convert';

import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';

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

ApiClient _client(MockClient httpClient) =>
    ApiClient('http://api.test/api/v1', httpClient, Identity(InMemoryStore()));

CreateRoomRequest _request({DateTime? eatAt}) => CreateRoomRequest(
      locationLabel: 'Salt Lake City',
      lat: 40.76,
      lng: -111.89,
      radiusM: 5000,
      priceMin: 1,
      priceMax: 4,
      minRating: 0.0,
      eatAt: eatAt,
    );

MockClient _capturing(void Function(Map<String, dynamic> body) onBody) =>
    MockClient((request) async {
      onBody(jsonDecode(request.body) as Map<String, dynamic>);
      return http.Response(
          jsonEncode({'room': _roomJson, 'deck': <Map<String, dynamic>>[]}),
          201);
    });

void main() {
  test('createRoom sends eat_at as ISO8601 UTC when set', () async {
    late Map<String, dynamic> body;
    final mock = _capturing((b) => body = b);

    await _client(mock)
        .createRoom(_request(eatAt: DateTime.utc(2026, 6, 13, 1)));

    expect(body['eat_at'], '2026-06-13T01:00:00.000Z');
  });

  test('createRoom omits eat_at when null', () async {
    late Map<String, dynamic> body;
    final mock = _capturing((b) => body = b);

    await _client(mock).createRoom(_request());

    expect(body, isNot(contains('eat_at')));
  });

  test('getRoom parses eat_at on the room', () async {
    final mock = MockClient((request) async => http.Response(
        jsonEncode({
          'room': {..._roomJson, 'eat_at': '2026-06-13T01:00:00.000Z'},
          'deck': <Map<String, dynamic>>[],
          'me': null,
        }),
        200));

    final detail = await _client(mock).getRoom('ABC234');

    expect(detail.room.eatAt, DateTime.utc(2026, 6, 13, 1));
  });
}
