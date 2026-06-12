import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/widgets/match_tile.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

const _restaurant = Restaurant(
  id: 'seed-001',
  name: 'Taco Cielo',
  cuisine: 'mexican',
  priceLevel: 2,
  rating: 4.5,
  ratingCount: 312,
  address: '12 Main St',
  lat: 40.76,
  lng: -111.89,
);

void main() {
  testWidgets('renders rank, name and like count', (tester) async {
    await tester.pumpWidget(const MaterialApp(
      home: Scaffold(
        body: MatchTile(rank: 1, restaurant: _restaurant, likeCount: 3),
      ),
    ));

    expect(find.text('1'), findsOneWidget);
    expect(find.text('Taco Cielo'), findsOneWidget);
    expect(find.text('3 liked'), findsOneWidget);
  });
}
