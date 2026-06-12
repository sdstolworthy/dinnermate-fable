import 'package:dinnermate/src/api/api_client.dart';
import 'package:dinnermate/src/api/models.dart';
import 'package:dinnermate/src/identity.dart';
import 'package:dinnermate/src/screens/list_detail.dart';
import 'package:dinnermate/src/state/lists_state.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:provider/provider.dart';

class _FakeApiClient extends ApiClient {
  _FakeApiClient({required bool isMember})
      : _isMember = isMember,
        super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  final bool _isMember;
  final fromListCalls = <(String, String?)>[];

  @override
  Future<(DinnerList, List<ListItem>, {bool isMember, bool isOwner})> getList(
          String code) async =>
      (
        DinnerList(
          id: 'l1',
          code: code,
          name: 'Date night spots',
          ownerUserId: 'owner-1',
        ),
        const <ListItem>[],
        isMember: _isMember,
        isOwner: _isMember,
      );

  @override
  Future<(Room, List<Restaurant>)> createRoomFromList(String listCode,
      {String? name}) async {
    fromListCalls.add((listCode, name));
    return (
      Room(
        id: 'room-1',
        code: 'ROOM01',
        name: name,
        locationLabel: 'Date night spots',
        lat: 0,
        lng: 0,
        radiusM: 1000,
        cuisines: const [],
        priceMin: 1,
        priceMax: 4,
        minRating: 0,
        createdAt: DateTime.utc(2026, 6, 12),
        sourceListName: 'Date night spots',
      ),
      const <Restaurant>[],
    );
  }
}

Widget _app(_FakeApiClient api) => MultiProvider(
      providers: [
        Provider<ApiClient>.value(value: api),
        ChangeNotifierProvider(create: (_) => ListsState(api)),
      ],
      child: const MaterialApp(home: ListDetailScreen(code: 'ABC234')),
    );

void main() {
  testWidgets('member sees the Swipe this list button', (tester) async {
    await tester.pumpWidget(_app(_FakeApiClient(isMember: true)));
    await tester.pumpAndSettle();

    expect(find.text('Swipe this list 🍽️'), findsOneWidget);
  });

  testWidgets('non-member does not see the Swipe this list button',
      (tester) async {
    await tester.pumpWidget(_app(_FakeApiClient(isMember: false)));
    await tester.pumpAndSettle();

    expect(find.text('Swipe this list 🍽️'), findsNothing);
  });

  testWidgets('tapping it creates a room from the list and shows the code',
      (tester) async {
    final api = _FakeApiClient(isMember: true);
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Swipe this list 🍽️'));
    await tester.pumpAndSettle();

    expect(api.fromListCalls, [('ABC234', 'Date night spots')]);
    expect(find.text('Room ready!'), findsOneWidget);
    expect(find.text('ROOM01'), findsOneWidget);
  });
}
