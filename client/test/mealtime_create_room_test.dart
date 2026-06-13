import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/screens/create_room.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';

/// Friday 2026-06-12, 17:30 local.
DateTime _fixedNow() => DateTime(2026, 6, 12, 17, 30);

class _CapturingApiClient extends ApiClient {
  _CapturingApiClient()
      : super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  CreateRoomRequest? captured;

  @override
  Future<(Room, List<Restaurant>)> createRoom(
      CreateRoomRequest request) async {
    captured = request;
    return (
      Room(
        id: 'room-1',
        code: 'ABC234',
        locationLabel: 'Salt Lake City',
        lat: 40.76,
        lng: -111.89,
        radiusM: 1000,
        cuisines: const [],
        priceMin: 1,
        priceMax: 4,
        minRating: 0,
        createdAt: DateTime.utc(2026, 6, 12),
      ),
      const <Restaurant>[],
    );
  }
}

Future<void> _pump(WidgetTester tester, _CapturingApiClient api) async {
  tester.view.physicalSize = const Size(800, 2600);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(Provider<ApiClient>.value(
    value: api,
    child: MaterialApp(home: CreateRoomScreen(now: _fixedNow)),
  ));
}

Future<void> _submit(WidgetTester tester) async {
  await tester.ensureVisible(find.text('Create room 🎉'));
  await tester.tap(find.text('Create room 🎉'));
  await tester.pumpAndSettle();
}

void main() {
  group('resolveEatAt', () {
    final now = _fixedNow();
    final cases = <({String name, EatTime mode, TimeOfDay? picked, DateTime? want})>[
      (
        name: 'anytime is null even with a stale picked time',
        mode: EatTime.anytime,
        picked: const TimeOfDay(hour: 18, minute: 0),
        want: null,
      ),
      (
        name: 'tonight is today 19:00 local as UTC',
        mode: EatTime.tonight,
        picked: null,
        want: DateTime(2026, 6, 12, 19).toUtc(),
      ),
      (
        name: 'picked time later today stays today',
        mode: EatTime.pickTime,
        picked: const TimeOfDay(hour: 18, minute: 45),
        want: DateTime(2026, 6, 12, 18, 45).toUtc(),
      ),
      (
        name: 'picked time earlier than now rolls to tomorrow',
        mode: EatTime.pickTime,
        picked: const TimeOfDay(hour: 6, minute: 30),
        want: DateTime(2026, 6, 13, 6, 30).toUtc(),
      ),
      (
        name: 'picked time equal to now rolls to tomorrow',
        mode: EatTime.pickTime,
        picked: const TimeOfDay(hour: 17, minute: 30),
        want: DateTime(2026, 6, 13, 17, 30).toUtc(),
      ),
      (
        name: 'pickTime without a chosen time is null',
        mode: EatTime.pickTime,
        picked: null,
        want: null,
      ),
    ];
    for (final c in cases) {
      test(c.name, () => expect(resolveEatAt(c.mode, c.picked, now), c.want));
    }
  });

  testWidgets('defaults to Anytime and submits a null eat_at', (tester) async {
    final api = _CapturingApiClient();
    await _pump(tester, api);

    await _submit(tester);

    expect(api.captured, isNotNull);
    expect(api.captured!.eatAt, isNull);
  });

  testWidgets('Tonight submits today 19:00 local as the exact UTC instant',
      (tester) async {
    final api = _CapturingApiClient();
    await _pump(tester, api);

    await tester.tap(find.text('Tonight'));
    await tester.pump();
    await _submit(tester);

    expect(api.captured!.eatAt, DateTime(2026, 6, 12, 19).toUtc());
  });

  testWidgets('Tonight shows a 7:00 PM chip', (tester) async {
    final api = _CapturingApiClient();
    await _pump(tester, api);

    await tester.tap(find.text('Tonight'));
    await tester.pump();

    expect(find.text('7:00 PM'), findsOneWidget);
  });

  testWidgets(
      'Pick a time accepting the pre-filled now rolls to tomorrow in the chip',
      (tester) async {
    final api = _CapturingApiClient();
    await _pump(tester, api);

    await tester.tap(find.text('Pick a time'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('OK'));
    await tester.pumpAndSettle();

    expect(find.text('Tomorrow 5:30 PM'), findsOneWidget);
  });

  testWidgets('cancelling the picker without a prior time stays on Anytime',
      (tester) async {
    final api = _CapturingApiClient();
    await _pump(tester, api);

    await tester.tap(find.text('Pick a time'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    await _submit(tester);
    expect(api.captured!.eatAt, isNull);
  });
}
