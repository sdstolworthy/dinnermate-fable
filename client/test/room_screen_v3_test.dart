import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/recent_rooms.dart';
import 'package:dinnermate/src/screens/room.dart';
import 'package:dinnermate/src/state/room_state.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';

RoomDetail _detail(String code, {List<String> participants = const []}) =>
    RoomDetail(
      room: Room(
        id: 'room-1',
        code: code,
        locationLabel: 'Date night spots',
        lat: 0,
        lng: 0,
        radiusM: 1000,
        cuisines: const [],
        priceMin: 1,
        priceMax: 4,
        minRating: 0,
        createdAt: DateTime.utc(2026, 6, 12),
        sourceListName: 'Date night spots',
      ),
      deck: const [
        Restaurant(id: 'list-1', name: "Aunt Carmen's", address: ''),
      ],
      participants: participants,
    );

class _FakeApiClient extends ApiClient {
  _FakeApiClient({this.detail, this.error})
      : super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  final RoomDetail Function(String code)? detail;
  final ApiException? error;

  @override
  Future<RoomDetail> getRoom(String code) async {
    final err = error;
    if (err != null) throw err;
    return detail!(code);
  }
}

Widget _app(_FakeApiClient api, RecentRooms recents) => MultiProvider(
      providers: [
        Provider<ApiClient>.value(value: api),
        ChangeNotifierProvider(
          create: (_) =>
              RoomState(api, 'ABC234', recentRooms: recents)..load(),
        ),
      ],
      child: const MaterialApp(home: RoomScreen(code: 'ABC234')),
    );

void main() {
  testWidgets('shows the participants line under the code chip',
      (tester) async {
    final api = _FakeApiClient(
        detail: (code) => _detail(code, participants: ['Alice', 'Bob']));
    await tester.pumpWidget(_app(api, RecentRooms(InMemoryStore())));
    await tester.pumpAndSettle();

    expect(find.text('👥 Alice, Bob'), findsOneWidget);
  });

  testWidgets('collapses more than three participants into +N',
      (tester) async {
    final api = _FakeApiClient(
        detail: (code) =>
            _detail(code, participants: ['Alice', 'Bob', 'Cleo', 'Dan']));
    await tester.pumpWidget(_app(api, RecentRooms(InMemoryStore())));
    await tester.pumpAndSettle();

    expect(find.text('👥 Alice, Bob +2'), findsOneWidget);
  });

  testWidgets('shows the From list tag for list-born rooms', (tester) async {
    final api = _FakeApiClient(detail: _detail);
    await tester.pumpWidget(_app(api, RecentRooms(InMemoryStore())));
    await tester.pumpAndSettle();

    expect(find.text('From list: Date night spots'), findsOneWidget);
  });

  testWidgets('records the room into recents on successful load',
      (tester) async {
    final recents = RecentRooms(InMemoryStore());
    await tester.pumpWidget(_app(_FakeApiClient(detail: _detail), recents));
    await tester.pumpAndSettle();

    final entries = await recents.all();
    expect(entries.single.code, 'ABC234');
    // Label falls back to the source list name when the room is unnamed.
    expect(entries.single.label, 'Date night spots');
  });

  testWidgets('404 shows the room-has-ended state without a retry',
      (tester) async {
    final api = _FakeApiClient(
        error: const ApiException('NOT_FOUND', 'room not found', 404));
    await tester.pumpWidget(_app(api, RecentRooms(InMemoryStore())));
    await tester.pumpAndSettle();

    expect(find.text('This room has ended'), findsOneWidget);
    expect(find.text('Back home'), findsOneWidget);
    expect(find.text('Try again'), findsNothing);
  });

  testWidgets('404 drops the room from recents', (tester) async {
    final recents = RecentRooms(InMemoryStore());
    await recents.record('ABC234', 'Friday crew');
    final api = _FakeApiClient(
        error: const ApiException('NOT_FOUND', 'room not found', 404));

    await tester.pumpWidget(_app(api, recents));
    await tester.pumpAndSettle();

    expect(await recents.all(), isEmpty);
  });
}
