import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/widgets/restaurant_card.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

// 2026-06-15 is a Monday (day index 1).
DateTime _mondayNoonUtc() => DateTime.utc(2026, 6, 15, 12, 0);

Restaurant _restaurant({List<HoursPeriod>? hours, int? utcOffsetMinutes}) =>
    Restaurant(
      id: 'r1',
      name: 'Casa Verde',
      cuisine: 'mexican',
      priceLevel: 2,
      rating: 4.6,
      ratingCount: 1204,
      address: '500 Taco Lane, Salt Lake City',
      lat: 40.76,
      lng: -111.89,
      hours: hours,
      utcOffsetMinutes: utcOffsetMinutes,
    );

Widget _wrap(Restaurant restaurant) => MaterialApp(
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: 360,
            height: 520,
            child: RestaurantCardBack(
              restaurant: restaurant,
              nowUtc: _mondayNoonUtc,
            ),
          ),
        ),
      ),
    );

void main() {
  testWidgets('shows today hours, badge, address and formatted stats',
      (tester) async {
    await tester.pumpWidget(_wrap(_restaurant(
      hours: const [HoursPeriod(day: 1, open: '11:00', close: '22:00')],
      utcOffsetMinutes: 0,
    )));

    expect(find.text('Today: 11:00–22:00'), findsOneWidget);
    expect(find.text('Open · until 22:00'), findsOneWidget);
    expect(find.text('500 Taco Lane, Salt Lake City'), findsOneWidget);
    expect(find.text('★ 4.6 (1,204 ratings)'), findsOneWidget);
    expect(find.text('\$\$'), findsOneWidget);
  });

  testWidgets('shows Today: closed when no periods fall on today',
      (tester) async {
    await tester.pumpWidget(_wrap(_restaurant(
      hours: const [HoursPeriod(day: 2, open: '11:00', close: '22:00')],
      utcOffsetMinutes: 0,
    )));

    expect(find.text('Today: closed'), findsOneWidget);
    expect(find.text('Closed · opens 11:00'), findsOneWidget);
  });

  testWidgets('omits hours line and badge when hours are unknown',
      (tester) async {
    await tester.pumpWidget(_wrap(_restaurant()));

    expect(find.textContaining('Today:'), findsNothing);
    expect(find.textContaining('Open'), findsNothing);
    expect(find.textContaining('Closed'), findsNothing);
  });
}
