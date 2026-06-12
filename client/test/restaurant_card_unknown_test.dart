import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/widgets/restaurant_card.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

// Free-form list item: nothing known beyond the name.
const _allNull = Restaurant(
  id: 'list-1',
  name: "Aunt Carmen's",
  address: '',
);

Widget _wrap(Widget card) => MaterialApp(
      home: Scaffold(
        body: Center(
          child: SizedBox(width: 360, height: 520, child: card),
        ),
      ),
    );

void main() {
  testWidgets('front face hides \$, rating and chip for all-null restaurant',
      (tester) async {
    await tester.pumpWidget(_wrap(const RestaurantCard(restaurant: _allNull)));

    expect(find.textContaining('\$'), findsNothing);
    expect(find.textContaining('★'), findsNothing);
    expect(find.byType(CuisineChip), findsNothing);
  });

  testWidgets('front face shows the New to us caption when unrated',
      (tester) async {
    await tester.pumpWidget(_wrap(const RestaurantCard(restaurant: _allNull)));

    expect(find.text('New to us'), findsOneWidget);
  });

  testWidgets('front face falls back to the neutral 🍽️ placeholder',
      (tester) async {
    await tester.pumpWidget(_wrap(const RestaurantCard(restaurant: _allNull)));

    expect(find.text('🍽️'), findsOneWidget);
  });

  testWidgets('back face hides \$, rating and chip and shows New to us',
      (tester) async {
    await tester
        .pumpWidget(_wrap(const RestaurantCardBack(restaurant: _allNull)));

    expect(find.textContaining('\$'), findsNothing);
    expect(find.textContaining('★'), findsNothing);
    expect(find.byType(CuisineChip), findsNothing);
    expect(find.text('New to us'), findsOneWidget);
  });
}
