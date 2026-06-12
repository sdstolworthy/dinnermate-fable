import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/hours.dart';
import 'package:flutter_test/flutter_test.dart';

typedef HoursCase = ({
  String name,
  List<HoursPeriod>? hours,
  int? utcOffsetMinutes,
  DateTime nowUtc,
  OpenStatus expected,
});

// Reference clock: 2026-06-12 is a Friday (day 5); offset -360 = UTC-6, so
// restaurant-local time is six hours behind the UTC instants below.
const _offset = -360;

const _friStandard = [HoursPeriod(day: 5, open: '11:00', close: '22:00')];

void main() {
  final cases = <HoursCase>[
    (
      name: 'open within span',
      hours: _friStandard,
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 18, 0), // Fri 12:00 local
      expected: const OpenNow(until: '22:00'),
    ),
    (
      name: 'closed before open, opens later today',
      hours: _friStandard,
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 15, 0), // Fri 09:00 local
      expected: const ClosedNow(opensNext: '11:00'),
    ),
    (
      name: 'closed day with no periods, opens next day',
      hours: const [HoursPeriod(day: 6, open: '11:00', close: '22:00')],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 18, 0), // Fri 12:00 local
      expected: const ClosedNow(opensNext: '11:00'),
    ),
    (
      name: 'midnight-crossing period open at 00:30 next day',
      hours: const [HoursPeriod(day: 5, open: '17:00', close: '01:00')],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 13, 6, 30), // Sat 00:30 local
      expected: const OpenNow(until: '01:00'),
    ),
    (
      name: 'midnight-crossing period open during evening leg',
      hours: const [HoursPeriod(day: 5, open: '17:00', close: '01:00')],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 13, 5, 0), // Fri 23:00 local
      expected: const OpenNow(until: '01:00'),
    ),
    (
      name: '24h period 00:00-23:59',
      hours: const [HoursPeriod(day: 5, open: '00:00', close: '23:59')],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 18, 0), // Fri 12:00 local
      expected: const OpenNow(until: '23:59'),
    ),
    (
      name: 'unknown when hours are null',
      hours: null,
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 18, 0),
      expected: const UnknownHours(),
    ),
    (
      name: 'unknown when utc offset is null',
      hours: _friStandard,
      utcOffsetMinutes: null,
      nowUtc: DateTime.utc(2026, 6, 12, 18, 0),
      expected: const UnknownHours(),
    ),
    (
      name: 'boundary: exactly at open is open',
      hours: _friStandard,
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 17, 0), // Fri 11:00 local
      expected: const OpenNow(until: '22:00'),
    ),
    (
      name: 'boundary: exactly at close is closed',
      hours: _friStandard,
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 13, 4, 0), // Fri 22:00 local
      expected: const ClosedNow(opensNext: '11:00'),
    ),
    (
      name: 'opens_next finds later-today period between lunch and dinner',
      hours: const [
        HoursPeriod(day: 5, open: '11:00', close: '14:30'),
        HoursPeriod(day: 5, open: '17:00', close: '22:00'),
      ],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 21, 0), // Fri 15:00 local
      expected: const ClosedNow(opensNext: '17:00'),
    ),
    (
      name: 'opens_next falls through to next-day period',
      hours: const [HoursPeriod(day: 6, open: '09:00', close: '14:00')],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 13, 2, 0), // Fri 20:00 local
      expected: const ClosedNow(opensNext: '09:00'),
    ),
    (
      name: 'no periods at all yields closed with no opens_next',
      hours: const <HoursPeriod>[],
      utcOffsetMinutes: _offset,
      nowUtc: DateTime.utc(2026, 6, 12, 18, 0),
      expected: const ClosedNow(),
    ),
  ];

  group('openStatusFor', () {
    for (final c in cases) {
      test(
        c.name,
        () => expect(
          openStatusFor(c.hours, c.utcOffsetMinutes, c.nowUtc),
          equals(c.expected),
        ),
      );
    }
  });
}
