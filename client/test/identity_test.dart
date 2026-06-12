import 'package:dinnermate/src/identity.dart';
import 'package:flutter_test/flutter_test.dart';

final _uuidV4Pattern = RegExp(
    r'^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$');

void main() {
  test('two Identity instances over the same store share one id', () async {
    final store = InMemoryStore();
    final first = await Identity(store).userId;
    final second = await Identity(store).userId;
    expect(second, first);
  });

  test('generated id is a valid UUID v4', () async {
    final id = await Identity(InMemoryStore()).userId;
    expect(id, matches(_uuidV4Pattern));
  });

  test('existing stored id is reused, not regenerated', () async {
    final store = InMemoryStore();
    await store.write('dinnermate_user_id', 'pre-existing-id');
    expect(await Identity(store).userId, 'pre-existing-id');
  });
}
