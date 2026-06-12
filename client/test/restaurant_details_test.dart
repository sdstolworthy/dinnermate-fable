import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/screens/restaurant_details.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';

const _restaurant = Restaurant(
  id: 'seed-001',
  name: 'Taco Cielo',
  cuisine: 'mexican',
  priceLevel: 2,
  rating: 4.5,
  ratingCount: 312,
  address: '12 Main St, Salt Lake City',
  lat: 40.76,
  lng: -111.89,
  // Closed Sundays (day 0 has no period).
  hours: [
    HoursPeriod(day: 1, open: '11:00', close: '22:00'),
    HoursPeriod(day: 2, open: '11:00', close: '22:00'),
    HoursPeriod(day: 3, open: '11:00', close: '22:00'),
    HoursPeriod(day: 4, open: '11:00', close: '22:00'),
    HoursPeriod(day: 5, open: '11:00', close: '23:00'),
    HoursPeriod(day: 6, open: '11:00', close: '23:00'),
  ],
  utcOffsetMinutes: 0,
);

class _FakeApiClient extends ApiClient {
  _FakeApiClient(this.details)
      : super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  final RestaurantDetails details;

  @override
  Future<RestaurantDetails> getRestaurantDetails(
          String roomCode, String restaurantId) async =>
      details;
}

Widget _app(RestaurantDetails details) => Provider<ApiClient>(
      create: (_) => _FakeApiClient(details),
      child: MaterialApp(
        home: RestaurantDetailsScreen(
          code: 'ABC234',
          restaurantId: 'seed-001',
          // Wednesday 2026-06-10 18:00 UTC.
          nowUtc: () => DateTime.utc(2026, 6, 10, 18),
          // Keeps tests offline: no tile/network access.
          mapBuilder: (_, __) => const ColoredBox(color: Colors.black12),
        ),
      ),
    );

void main() {
  testWidgets('renders name, address, weekly hours; hides empty reviews',
      (tester) async {
    await tester.pumpWidget(_app(const RestaurantDetails(
      restaurant: _restaurant,
      website: 'https://tacocielo.example',
      phone: '+1 801 555 0123',
      reviews: [],
    )));
    await tester.pumpAndSettle();

    // AppBar title + headline both show the name.
    expect(find.text('Taco Cielo'), findsNWidgets(2));
    expect(find.text('12 Main St, Salt Lake City'), findsOneWidget);

    // All seven day rows; Sunday has no periods -> Closed.
    for (final day in [
      'Sunday',
      'Monday',
      'Tuesday',
      'Wednesday',
      'Thursday',
      'Friday',
      'Saturday',
    ]) {
      expect(find.text(day), findsOneWidget);
    }
    expect(find.text('Closed'), findsOneWidget);
    expect(find.text('11:00–22:00'), findsNWidgets(4));

    // Today (Wednesday) is bolded.
    final wednesday = tester.widget<Text>(find.text('Wednesday'));
    expect(wednesday.style?.fontWeight, FontWeight.w800);
    final monday = tester.widget<Text>(find.text('Monday'));
    expect(monday.style?.fontWeight, FontWeight.w400);

    expect(find.text('Website'), findsOneWidget);
    expect(find.text('Call'), findsOneWidget);
    expect(find.text('Directions'), findsOneWidget);
    expect(find.text('Reviews'), findsNothing);
  });

  testWidgets('shows reviews when present and hides null actions',
      (tester) async {
    await tester.pumpWidget(_app(const RestaurantDetails(
      restaurant: _restaurant,
      reviews: [
        Review(
          author: 'Ana',
          rating: 5,
          text: 'Best al pastor in town.',
          relativeTime: '2 months ago',
        ),
      ],
    )));
    await tester.pumpAndSettle();

    expect(find.text('Reviews'), findsOneWidget);
    expect(find.text('Ana'), findsOneWidget);
    expect(find.text('★ 5'), findsOneWidget);
    expect(find.text('Best al pastor in town.'), findsOneWidget);
    expect(find.text('2 months ago'), findsOneWidget);

    // website/phone are null -> their buttons are hidden.
    expect(find.text('Website'), findsNothing);
    expect(find.text('Call'), findsNothing);
    expect(find.text('Directions'), findsOneWidget);
  });
}
