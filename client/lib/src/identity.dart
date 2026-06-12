import 'dart:math';

import 'package:shared_preferences/shared_preferences.dart';

abstract class KeyValueStore {
  Future<String?> read(String key);
  Future<void> write(String key, String value);
}

class SharedPrefsStore implements KeyValueStore {
  @override
  Future<String?> read(String key) async =>
      (await SharedPreferences.getInstance()).getString(key);

  @override
  Future<void> write(String key, String value) async {
    await (await SharedPreferences.getInstance()).setString(key, value);
  }
}

class InMemoryStore implements KeyValueStore {
  final Map<String, String> _values = {};

  @override
  Future<String?> read(String key) async => _values[key];

  @override
  Future<void> write(String key, String value) async => _values[key] = value;
}

String uuidV4({Random? random}) {
  final rng = random ?? Random.secure();
  final bytes = List<int>.generate(16, (_) => rng.nextInt(256));
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  final hex =
      bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
  return '${hex.substring(0, 8)}-${hex.substring(8, 12)}-'
      '${hex.substring(12, 16)}-${hex.substring(16, 20)}-${hex.substring(20)}';
}

/// Anonymous device identity: a UUID v4 generated once and persisted.
class Identity {
  Identity(this._store);

  static const _storageKey = 'dinnermate_user_id';

  final KeyValueStore _store;
  Future<String>? _userId;

  Future<String> get userId => _userId ??= _loadOrCreate();

  Future<String> _loadOrCreate() async {
    final existing = await _store.read(_storageKey);
    if (existing != null) return existing;
    final id = uuidV4();
    await _store.write(_storageKey, id);
    return id;
  }
}
