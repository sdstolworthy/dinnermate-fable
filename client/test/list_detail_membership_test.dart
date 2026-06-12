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
  _FakeApiClient({required bool isMember, required bool isOwner})
      : _isMember = isMember,
        _isOwner = isOwner,
        super(
          'http://test/api/v1',
          MockClient((request) async => http.Response('{}', 200)),
          Identity(InMemoryStore()),
        );

  bool _isMember;
  final bool _isOwner;
  int joinCalls = 0;

  DinnerList _list(String code) => DinnerList(
        id: 'l1',
        code: code,
        name: 'Date night spots',
        ownerUserId: 'owner-1',
      );

  @override
  Future<(DinnerList, List<ListItem>, {bool isMember, bool isOwner})> getList(
          String code) async =>
      (_list(code), const <ListItem>[], isMember: _isMember, isOwner: _isOwner);

  @override
  Future<(DinnerList, bool isOwner)> joinList(String code) async {
    joinCalls++;
    _isMember = true;
    return (_list(code), false);
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
  testWidgets('non-member sees Join button and no add FAB', (tester) async {
    final api = _FakeApiClient(isMember: false, isOwner: false);
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    expect(find.text('Join this list'), findsOneWidget);
    expect(find.byType(FloatingActionButton), findsNothing);
  });

  testWidgets('joining makes the list editable', (tester) async {
    final api = _FakeApiClient(isMember: false, isOwner: false);
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Join this list'));
    await tester.pumpAndSettle();

    expect(api.joinCalls, 1);
    expect(find.text('Join this list'), findsNothing);
    expect(find.byType(FloatingActionButton), findsOneWidget);
  });

  testWidgets('owner sees the share action', (tester) async {
    final api = _FakeApiClient(isMember: true, isOwner: true);
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.ios_share_rounded), findsOneWidget);
    expect(find.text('Join this list'), findsNothing);
    expect(find.byType(FloatingActionButton), findsOneWidget);
  });

  testWidgets('member-not-owner gets a leave menu instead of share',
      (tester) async {
    final api = _FakeApiClient(isMember: true, isOwner: false);
    await tester.pumpWidget(_app(api));
    await tester.pumpAndSettle();

    expect(find.byIcon(Icons.ios_share_rounded), findsNothing);
    await tester.tap(find.byType(PopupMenuButton<String>));
    await tester.pumpAndSettle();
    expect(find.text('Leave list'), findsOneWidget);
  });
}
