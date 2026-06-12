import 'dart:convert';

import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/recent_rooms.dart';
import 'package:flutter_test/flutter_test.dart';

/// Deterministic clock: each call advances one minute from a fixed epoch.
class _TickingClock {
  int _ticks = 0;

  DateTime call() =>
      DateTime.utc(2026, 6, 12, 10).add(Duration(minutes: _ticks++));
}

(RecentRooms, InMemoryStore) _setUp() {
  final store = InMemoryStore();
  return (RecentRooms(store, now: _TickingClock().call), store);
}

void main() {
  test('all() is empty when nothing has been recorded', () async {
    final (rooms, _) = _setUp();

    expect(await rooms.all(), isEmpty);
  });

  test('record stores code, label and the injected clock time', () async {
    final (rooms, store) = _setUp();

    await rooms.record('ABC234', 'Friday dinner');

    final stored =
        jsonDecode((await store.read('dinnermate_recent_rooms'))!) as List;
    expect(stored, [
      {
        'code': 'ABC234',
        'label': 'Friday dinner',
        'last_seen': '2026-06-12T10:00:00.000Z',
      },
    ]);
  });

  test('all() returns entries most-recent-first', () async {
    final (rooms, _) = _setUp();

    await rooms.record('AAA111', 'First');
    await rooms.record('BBB222', 'Second');
    await rooms.record('CCC333', 'Third');

    final codes = (await rooms.all()).map((e) => e.code).toList();
    expect(codes, ['CCC333', 'BBB222', 'AAA111']);
  });

  test('re-recording a code dedupes and bumps it to the front', () async {
    final (rooms, _) = _setUp();
    await rooms.record('AAA111', 'First');
    await rooms.record('BBB222', 'Second');

    await rooms.record('AAA111', 'First again');

    final entries = await rooms.all();
    expect(entries.map((e) => e.code).toList(), ['AAA111', 'BBB222']);
    expect(entries.first.label, 'First again');
    expect(entries.first.lastSeen, DateTime.utc(2026, 6, 12, 10, 2));
  });

  test('record caps the history at 5, dropping the oldest', () async {
    final (rooms, _) = _setUp();

    for (var i = 1; i <= 6; i++) {
      await rooms.record('ROOM$i', 'Room $i');
    }

    final codes = (await rooms.all()).map((e) => e.code).toList();
    expect(codes, ['ROOM6', 'ROOM5', 'ROOM4', 'ROOM3', 'ROOM2']);
  });

  test('remove drops only the matching code', () async {
    final (rooms, _) = _setUp();
    await rooms.record('AAA111', 'First');
    await rooms.record('BBB222', 'Second');

    await rooms.remove('AAA111');

    expect((await rooms.all()).map((e) => e.code).toList(), ['BBB222']);
  });

  test('remove of an unknown code is a no-op', () async {
    final (rooms, _) = _setUp();
    await rooms.record('AAA111', 'First');

    await rooms.remove('ZZZ999');

    expect(await rooms.all(), hasLength(1));
  });

  test('a corrupt stored value is treated as empty', () async {
    final store = InMemoryStore();
    await store.write('dinnermate_recent_rooms', 'not json');
    final rooms = RecentRooms(store, now: _TickingClock().call);

    expect(await rooms.all(), isEmpty);

    await rooms.record('ABC234', 'Friday dinner');
    expect((await rooms.all()).single.code, 'ABC234');
  });
}
