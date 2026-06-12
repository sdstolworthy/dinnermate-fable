import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/widgets/open_badge.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

// 2026-06-15 is a Monday (day index 1).
const _mondayHours = [HoursPeriod(day: 1, open: '11:00', close: '22:00')];

Widget _wrap(Widget child) =>
    MaterialApp(home: Scaffold(body: Center(child: child)));

void main() {
  testWidgets('within an open span renders Open · until close', (tester) async {
    await tester.pumpWidget(_wrap(OpenBadge(
      _mondayHours,
      0,
      nowUtc: () => DateTime.utc(2026, 6, 15, 12, 0),
    )));

    expect(find.text('Open · until 22:00'), findsOneWidget);
  });

  testWidgets('closed before opening renders Closed · opens time',
      (tester) async {
    await tester.pumpWidget(_wrap(OpenBadge(
      _mondayHours,
      0,
      nowUtc: () => DateTime.utc(2026, 6, 15, 9, 0),
    )));

    expect(find.text('Closed · opens 11:00'), findsOneWidget);
  });

  testWidgets('closed with no upcoming opening renders plain Closed',
      (tester) async {
    await tester.pumpWidget(_wrap(OpenBadge(
      const <HoursPeriod>[],
      0,
      nowUtc: () => DateTime.utc(2026, 6, 15, 12, 0),
    )));

    expect(find.text('Closed'), findsOneWidget);
  });

  testWidgets('unknown hours renders nothing', (tester) async {
    await tester.pumpWidget(_wrap(OpenBadge(
      null,
      null,
      nowUtc: () => DateTime.utc(2026, 6, 15, 12, 0),
    )));

    expect(find.byType(Text), findsNothing);
  });
}
