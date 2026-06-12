import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/widgets/swipe_deck.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

Restaurant _restaurant(String id, String name) => Restaurant(
      id: id,
      name: name,
      cuisine: 'thai',
      priceLevel: 2,
      rating: 4.4,
      ratingCount: 120,
      address: '12 Noodle Way',
      lat: 40.76,
      lng: -111.89,
    );

class _Harness {
  final swipes = <(String, bool)>[];
  bool ended = false;
  final key = GlobalKey<SwipeDeckState>();

  Widget build() => MaterialApp(
        home: Scaffold(
          body: SwipeDeck(
            key: key,
            restaurants: [
              _restaurant('one', 'First Bite'),
              _restaurant('two', 'Second Helping'),
            ],
            onSwipe: (restaurant, liked) => swipes.add((restaurant.id, liked)),
            onDeckEnd: () => ended = true,
          ),
        ),
      );
}

void main() {
  testWidgets('drag right past threshold fires onSwipe(liked) and advances',
      (tester) async {
    final harness = _Harness();
    await tester.pumpWidget(harness.build());

    await tester.drag(find.byType(SwipeDeck), const Offset(420, 0));
    await tester.pumpAndSettle();

    expect(harness.swipes, [('one', true)]);
    expect(find.text('First Bite'), findsNothing);
    expect(find.text('Second Helping'), findsOneWidget);
  });

  testWidgets('short drag springs back without swiping', (tester) async {
    final harness = _Harness();
    await tester.pumpWidget(harness.build());

    await tester.drag(find.byType(SwipeDeck), const Offset(80, 0));
    await tester.pumpAndSettle();

    expect(harness.swipes, isEmpty);
    expect(find.text('First Bite'), findsOneWidget);
  });

  testWidgets('programmatic nope/like advance the deck and fire onDeckEnd',
      (tester) async {
    final harness = _Harness();
    await tester.pumpWidget(harness.build());

    harness.key.currentState!.nope();
    await tester.pumpAndSettle();
    expect(harness.swipes, [('one', false)]);

    harness.key.currentState!.like();
    await tester.pumpAndSettle();

    expect(harness.swipes, [('one', false), ('two', true)]);
    expect(harness.ended, isTrue);
  });

  // The back face is identified by its "(N ratings)" stat, which the front
  // never renders. Harness restaurants have no hours, so the back face is
  // deterministic without a clock.
  testWidgets('tap flips the top card to its back face and back to front',
      (tester) async {
    final harness = _Harness();
    await tester.pumpWidget(harness.build());

    expect(find.textContaining('ratings'), findsNothing);

    await tester.tap(find.byType(SwipeDeck));
    await tester.pumpAndSettle();
    expect(find.textContaining('ratings'), findsOneWidget);

    await tester.tap(find.byType(SwipeDeck));
    await tester.pumpAndSettle();
    expect(find.textContaining('ratings'), findsNothing);
  });

  testWidgets('drag past threshold while flipped still swipes and resets flip',
      (tester) async {
    final harness = _Harness();
    await tester.pumpWidget(harness.build());

    await tester.tap(find.byType(SwipeDeck));
    await tester.pumpAndSettle();
    expect(find.textContaining('ratings'), findsOneWidget);

    await tester.drag(find.byType(SwipeDeck), const Offset(420, 0));
    await tester.pumpAndSettle();

    expect(harness.swipes, [('one', true)]);
    expect(find.text('Second Helping'), findsOneWidget);
    expect(find.textContaining('ratings'), findsNothing);
  });

  testWidgets('programmatic like works while flipped', (tester) async {
    final harness = _Harness();
    await tester.pumpWidget(harness.build());

    await tester.tap(find.byType(SwipeDeck));
    await tester.pumpAndSettle();

    harness.key.currentState!.like();
    await tester.pumpAndSettle();

    expect(harness.swipes, [('one', true)]);
    expect(find.textContaining('ratings'), findsNothing);
  });
}
