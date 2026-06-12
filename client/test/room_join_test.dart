import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/screens/room.dart';
import 'package:dinnermate/src/state/room_state.dart';
import 'package:dinnermate/src/widgets/swipe_deck.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';

class _FakeApiClient extends ApiClient {
  _FakeApiClient()
      : super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  final joinNames = <String>[];

  @override
  Future<RoomDetail> getRoom(String code) async => RoomDetail(
        room: Room(
          id: 'room-1',
          code: code,
          locationLabel: 'Salt Lake City',
          lat: 40.76,
          lng: -111.89,
          radiusM: 5000,
          cuisines: const [],
          priceMin: 1,
          priceMax: 4,
          minRating: 0,
          createdAt: DateTime.utc(2026, 6, 11),
        ),
        deck: const [
          Restaurant(
            id: 'seed-001',
            name: 'Taco Cielo',
            cuisine: 'mexican',
            priceLevel: 2,
            rating: 4.5,
            ratingCount: 312,
            address: '12 Main St',
            lat: 40.76,
            lng: -111.89,
          ),
        ],
      );

  @override
  Future<Participant> joinRoom(String code, String displayName) async {
    joinNames.add(displayName);
    return Participant(
      id: 'p1',
      roomId: 'room-1',
      userId: 'u1',
      displayName: displayName,
    );
  }

  @override
  Future<MatchesResult> getMatches(String code) async =>
      const MatchesResult(entries: [], participantCount: 1);

  @override
  Future<void> swipe(String code, String restaurantId, bool liked) async {}
}

Widget _app(_FakeApiClient api) => MultiProvider(
      providers: [
        Provider<ApiClient>.value(value: api),
        ChangeNotifierProvider(create: (_) => RoomState(api, 'ABC234')..load()),
      ],
      child: const MaterialApp(home: RoomScreen(code: 'ABC234')),
    );

void main() {
  testWidgets('empty display name shows validation and does not join',
      (tester) async {
    final api = _FakeApiClient();
    await tester.pumpWidget(_app(api));
    await tester.pump();

    await tester.tap(find.text('Join & swipe'));
    await tester.pump();

    expect(find.text('Enter your name'), findsOneWidget);
    expect(api.joinNames, isEmpty);
  });

  testWidgets('valid name joins and shows the swipe deck', (tester) async {
    final api = _FakeApiClient();
    await tester.pumpWidget(_app(api));
    await tester.pump();

    await tester.enterText(find.byType(TextFormField), 'Spencer');
    await tester.tap(find.text('Join & swipe'));
    await tester.pump();
    await tester.pump();

    expect(api.joinNames, ['Spencer']);
    expect(find.byType(SwipeDeck), findsOneWidget);

    // Dispose the tree so RoomState cancels its polling timer.
    await tester.pumpWidget(const SizedBox());
  });
}
