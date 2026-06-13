import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/recent_rooms.dart';
import 'package:dinnermate/src/screens/room.dart';
import 'package:dinnermate/src/state/room_state.dart';
import 'package:dinnermate/src/time_format.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';

RoomDetail _detail(String code, {DateTime? eatAt}) => RoomDetail(
      room: Room(
        id: 'room-1',
        code: code,
        locationLabel: 'Salt Lake City',
        lat: 40.76,
        lng: -111.89,
        radiusM: 1000,
        cuisines: const [],
        priceMin: 1,
        priceMax: 4,
        minRating: 0,
        createdAt: DateTime.utc(2026, 6, 12),
        eatAt: eatAt,
      ),
      deck: const [
        Restaurant(id: 'seed-001', name: 'Taco Cielo', address: ''),
      ],
      participants: const ['Alice'],
    );

class _FakeApiClient extends ApiClient {
  _FakeApiClient(this.detail)
      : super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  final RoomDetail Function(String code) detail;

  @override
  Future<RoomDetail> getRoom(String code) async => detail(code);
}

Widget _app(_FakeApiClient api) => MultiProvider(
      providers: [
        Provider<ApiClient>.value(value: api),
        ChangeNotifierProvider(
          create: (_) => RoomState(api, 'ABC234',
              recentRooms: RecentRooms(InMemoryStore()))
            ..load(),
        ),
      ],
      child: const MaterialApp(home: RoomScreen(code: 'ABC234')),
    );

void main() {
  group('formatClockTime', () {
    final cases = <({String name, DateTime local, String want})>[
      (name: 'evening', local: DateTime(2026, 6, 12, 19), want: '7:00 PM'),
      (name: 'morning', local: DateTime(2026, 6, 13, 6, 30), want: '6:30 AM'),
      (name: 'midnight', local: DateTime(2026, 6, 12, 0, 5), want: '12:05 AM'),
      (name: 'noon', local: DateTime(2026, 6, 12, 12), want: '12:00 PM'),
    ];
    for (final c in cases) {
      test(c.name, () => expect(formatClockTime(c.local), c.want));
    }
  });

  testWidgets('header shows the eating-at tag in local time', (tester) async {
    final eatAt = DateTime.utc(2026, 6, 13, 1);
    final api = _FakeApiClient((code) => _detail(code, eatAt: eatAt));
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    // Expected label is computed through the same local conversion the
    // widget uses, so the test holds in any host timezone.
    expect(
      find.text('🕖 Eating at ${formatClockTime(eatAt.toLocal())}'),
      findsOneWidget,
    );
    expect(find.text('👥 Alice'), findsOneWidget);
  });

  testWidgets('header omits the tag when eat_at is null', (tester) async {
    final api = _FakeApiClient((code) => _detail(code));
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    expect(find.textContaining('Eating at'), findsNothing);
  });
}
