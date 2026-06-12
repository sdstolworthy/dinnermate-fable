import 'package:dinnermate/src/screens/create_room.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

Future<void> _pump(WidgetTester tester) async {
  tester.view.physicalSize = const Size(800, 1800);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.reset);
  await tester.pumpWidget(const MaterialApp(home: CreateRoomScreen()));
}

// The radius slider is the first Slider in the form (min-rating comes later).
Slider _radiusSlider(WidgetTester tester) =>
    tester.widgetList<Slider>(find.byType(Slider)).first;

Future<void> _setRadius(WidgetTester tester, double meters) async {
  _radiusSlider(tester).onChanged!(meters);
  await tester.pump();
}

void main() {
  testWidgets('defaults to Walking at 1.0 km with walk-minutes label',
      (tester) async {
    await _pump(tester);

    expect(find.text('Within 1.0 km · ~12 min walk'), findsOneWidget);
    final slider = _radiusSlider(tester);
    expect(slider.value, 1000);
    expect(slider.min, 250);
    expect(slider.max, 2000);
  });

  testWidgets('toggling to Driving clamps to 5 km with km-only label',
      (tester) async {
    await _pump(tester);

    await tester.tap(find.text('🚗 Driving'));
    await tester.pumpAndSettle();

    expect(find.text('Within 5 km'), findsOneWidget);
    expect(find.textContaining('min walk'), findsNothing);
    final slider = _radiusSlider(tester);
    expect(slider.value, 5000);
    expect(slider.min, 2000);
    expect(slider.max, 40000);
  });

  testWidgets('toggling back to Walking clamps to 1.0 km', (tester) async {
    await _pump(tester);

    await tester.tap(find.text('🚗 Driving'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('🚶 Walking'));
    await tester.pumpAndSettle();

    expect(find.text('Within 1.0 km · ~12 min walk'), findsOneWidget);
    expect(_radiusSlider(tester).value, 1000);
  });

  testWidgets('walking slider extremes render correct labels', (tester) async {
    await _pump(tester);

    await _setRadius(tester, 250);
    expect(find.text('Within 0.3 km · ~3 min walk'), findsOneWidget);

    await _setRadius(tester, 2000);
    expect(find.text('Within 2.0 km · ~25 min walk'), findsOneWidget);
  });

  testWidgets('driving slider extremes render correct labels', (tester) async {
    await _pump(tester);
    await tester.tap(find.text('🚗 Driving'));
    await tester.pumpAndSettle();

    await _setRadius(tester, 2000);
    expect(find.text('Within 2 km'), findsOneWidget);

    await _setRadius(tester, 40000);
    expect(find.text('Within 40 km'), findsOneWidget);
  });
}
