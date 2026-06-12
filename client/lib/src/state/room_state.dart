import 'dart:async';

import 'package:flutter/foundation.dart';

import '../api/api_client.dart';
import '../api/models.dart';

/// Per-room session: deck, membership, optimistic swipes, match polling.
class RoomState extends ChangeNotifier {
  RoomState(this._api, this.roomCode,
      {this.pollInterval = const Duration(seconds: 3)});

  final ApiClient _api;
  final String roomCode;
  final Duration pollInterval;

  Room? room;
  List<Restaurant> deck = const [];
  Participant? me;
  int deckIndex = 0;
  List<MatchEntry> matches = const [];
  int participantCount = 0;
  bool loading = true;
  bool joining = false;
  String? errorMessage;
  String? joinError;

  Timer? _timer;
  bool _disposed = false;

  bool get joined => me != null;
  bool get deckDone => deckIndex >= deck.length;

  Future<void> load() async {
    loading = true;
    errorMessage = null;
    _notify();
    try {
      final detail = await _api.getRoom(roomCode);
      room = detail.room;
      deck = detail.deck;
      me = detail.me;
      if (joined) startPolling();
    } on ApiException catch (e) {
      errorMessage = e.message;
    } on Exception {
      errorMessage = "Couldn't reach the kitchen. Check your connection?";
    }
    loading = false;
    _notify();
  }

  Future<void> join(String displayName) async {
    joining = true;
    joinError = null;
    _notify();
    try {
      me = await _api.joinRoom(roomCode, displayName);
      startPolling();
    } on ApiException catch (e) {
      joinError = e.message;
    } on Exception {
      joinError = "Couldn't join. Check your connection?";
    }
    joining = false;
    _notify();
  }

  /// Optimistic: the deck advances immediately, the POST follows.
  Future<void> swipe(String restaurantId, bool liked) async {
    deckIndex++;
    _notify();
    try {
      await _api.swipe(roomCode, restaurantId, liked);
    } on ApiException {
      // 409 means we already swiped this card (safe to ignore). Other
      // failures lose one swipe server-side, which beats blocking the deck.
    } on Exception {
      // Transport hiccup: same trade-off.
    }
  }

  Future<void> refreshMatches() async {
    try {
      final result = await _api.getMatches(roomCode);
      if (_disposed) return;
      matches = result.entries;
      participantCount = result.participantCount;
      _notify();
    } on Exception {
      // Polling is best-effort; the next tick retries.
    }
  }

  void startPolling() {
    _timer ??= Timer.periodic(pollInterval, (_) => refreshMatches());
    refreshMatches();
  }

  void stopPolling() {
    _timer?.cancel();
    _timer = null;
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    stopPolling();
    super.dispose();
  }
}
