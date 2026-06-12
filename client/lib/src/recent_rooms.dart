import 'dart:convert';

import 'identity.dart';

class RecentRoomEntry {
  const RecentRoomEntry({
    required this.code,
    required this.label,
    required this.lastSeen,
  });

  final String code;
  final String label;
  final DateTime lastSeen;

  factory RecentRoomEntry.fromJson(Map<String, dynamic> json) =>
      RecentRoomEntry(
        code: json['code'] as String,
        label: json['label'] as String,
        lastSeen: DateTime.parse(json['last_seen'] as String),
      );

  Map<String, dynamic> toJson() => {
        'code': code,
        'label': label,
        'last_seen': lastSeen.toUtc().toIso8601String(),
      };
}

/// Locally persisted "jump back in" history: the last [_maxEntries] rooms
/// this device opened, most-recent-first, deduped by room code.
class RecentRooms {
  RecentRooms(this._store, {DateTime Function()? now})
      : _now = now ?? DateTime.now;

  static const _storageKey = 'dinnermate_recent_rooms';
  static const _maxEntries = 5;

  final KeyValueStore _store;
  final DateTime Function() _now;

  Future<List<RecentRoomEntry>> all() => _load();

  Future<void> record(String code, String label) async {
    final entries = await _load()
      ..removeWhere((e) => e.code == code)
      ..insert(0, RecentRoomEntry(code: code, label: label, lastSeen: _now()));
    await _save(entries.take(_maxEntries).toList());
  }

  Future<void> remove(String code) async {
    final entries = await _load()
      ..removeWhere((e) => e.code == code);
    await _save(entries);
  }

  Future<List<RecentRoomEntry>> _load() async {
    final raw = await _store.read(_storageKey);
    if (raw == null) return [];
    try {
      return (jsonDecode(raw) as List)
          .map((e) => RecentRoomEntry.fromJson(e as Map<String, dynamic>))
          .toList();
    } on Object {
      // Corrupt or legacy payload: drop it rather than break the home screen.
      return [];
    }
  }

  Future<void> _save(List<RecentRoomEntry> entries) =>
      _store.write(_storageKey, jsonEncode([for (final e in entries) e.toJson()]));
}
