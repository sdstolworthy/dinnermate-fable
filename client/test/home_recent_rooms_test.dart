import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/recent_rooms.dart';
import 'package:dinnermate/src/screens/home.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:provider/provider.dart';

Widget _app(RecentRooms recents) => Provider<RecentRooms>.value(
      value: recents,
      child: const MaterialApp(home: HomeScreen()),
    );

void main() {
  testWidgets('shows Jump back in with the recorded room labels',
      (tester) async {
    final recents = RecentRooms(InMemoryStore());
    await recents.record('ABC234', 'Friday crew');
    await recents.record('XYZ789', 'Date night');

    await tester.pumpWidget(_app(recents));
    await tester.pumpAndSettle();

    expect(find.text('Jump back in'), findsOneWidget);
    expect(find.text('Friday crew · ABC234'), findsOneWidget);
    expect(find.text('Date night · XYZ789'), findsOneWidget);
  });

  testWidgets('hides the section entirely when nothing was recorded',
      (tester) async {
    await tester.pumpWidget(_app(RecentRooms(InMemoryStore())));
    await tester.pumpAndSettle();

    expect(find.text('Jump back in'), findsNothing);
  });
}
