import 'dart:convert';

import 'package:http/http.dart' as http;

import '../identity.dart';
import 'models.dart';

class ApiException implements Exception {
  const ApiException(this.code, this.message, this.status);

  final String code;
  final String message;
  final int status;

  @override
  String toString() => 'ApiException($status $code): $message';
}

class RoomDetail {
  const RoomDetail({required this.room, required this.deck, this.me});

  final Room room;
  final List<Restaurant> deck;
  final Participant? me;
}

class MatchesResult {
  const MatchesResult({required this.entries, required this.participantCount});

  final List<MatchEntry> entries;
  final int participantCount;
}

class ApiClient {
  ApiClient(this.baseUrl, this.httpClient, this.identity);

  final String baseUrl;
  final http.Client httpClient;
  final Identity identity;

  Future<(Room, List<Restaurant>)> createRoom(CreateRoomRequest request) async {
    final json = await _send('POST', '/rooms', body: request.toJson());
    return (_room(json['room']), _deck(json['deck']));
  }

  Future<RoomDetail> getRoom(String code) async {
    final json = await _send('GET', '/rooms/$code');
    return RoomDetail(
      room: _room(json['room']),
      deck: _deck(json['deck']),
      me: json['me'] == null
          ? null
          : Participant.fromJson(json['me'] as Map<String, dynamic>),
    );
  }

  Future<Participant> joinRoom(String code, String displayName) async {
    final json = await _send('POST', '/rooms/$code/join',
        body: {'display_name': displayName});
    return Participant.fromJson(json['participant'] as Map<String, dynamic>);
  }

  Future<void> swipe(String code, String restaurantId, bool liked) async {
    await _send('POST', '/rooms/$code/swipes',
        body: {'restaurant_id': restaurantId, 'liked': liked});
  }

  Future<MatchesResult> getMatches(String code) async {
    final json = await _send('GET', '/rooms/$code/matches');
    return MatchesResult(
      entries: (json['matches'] as List)
          .map((e) => MatchEntry.fromJson(e as Map<String, dynamic>))
          .toList(),
      participantCount: json['participant_count'] as int,
    );
  }

  Future<DinnerList> createList(String name) async {
    final json = await _send('POST', '/lists', body: {'name': name});
    return DinnerList.fromJson(json['list'] as Map<String, dynamic>);
  }

  Future<List<MyList>> getMyLists() async {
    final json = await _send('GET', '/lists');
    return (json['lists'] as List)
        .map((e) => MyList.fromJson(e as Map<String, dynamic>))
        .toList();
  }

  Future<(DinnerList, List<ListItem>, {bool isMember, bool isOwner})> getList(
      String code) async {
    final json = await _send('GET', '/lists/$code');
    return (
      DinnerList.fromJson(json['list'] as Map<String, dynamic>),
      (json['items'] as List)
          .map((e) => ListItem.fromJson(e as Map<String, dynamic>))
          .toList(),
      isMember: json['is_member'] as bool,
      isOwner: json['is_owner'] as bool,
    );
  }

  Future<(DinnerList, bool isOwner)> joinList(String code) async {
    final json = await _send('POST', '/lists/$code/join');
    return (
      DinnerList.fromJson(json['list'] as Map<String, dynamic>),
      json['is_owner'] as bool,
    );
  }

  Future<void> leaveList(String code) async {
    await _send('DELETE', '/lists/$code/members/me');
  }

  Future<RestaurantDetails> getRestaurantDetails(
      String roomCode, String restaurantId) async {
    final json =
        await _send('GET', '/rooms/$roomCode/restaurants/$restaurantId/details');
    return RestaurantDetails.fromJson(json);
  }

  Future<ListItem> addListItem(String code, NewListItem item) async {
    final json =
        await _send('POST', '/lists/$code/items', body: item.toJson());
    return ListItem.fromJson(json['item'] as Map<String, dynamic>);
  }

  Room _room(Object? json) => Room.fromJson(json as Map<String, dynamic>);

  List<Restaurant> _deck(Object? json) => (json as List)
      .map((e) => Restaurant.fromJson(e as Map<String, dynamic>))
      .toList();

  Future<Map<String, dynamic>> _send(String method, String path,
      {Map<String, dynamic>? body}) async {
    final request = http.Request(method, Uri.parse('$baseUrl$path'));
    request.headers['X-Dinnermate-User'] = await identity.userId;
    if (body != null) {
      request.headers['Content-Type'] = 'application/json';
      request.body = jsonEncode(body);
    }
    final response =
        await http.Response.fromStream(await httpClient.send(request));
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw _toException(response);
    }
    if (response.body.isEmpty) return const {};
    return jsonDecode(response.body) as Map<String, dynamic>;
  }

  ApiException _toException(http.Response response) {
    final status = response.statusCode;
    try {
      final envelope = jsonDecode(response.body) as Map<String, dynamic>;
      final error = envelope['error'] as Map<String, dynamic>;
      return ApiException(
          error['code'] as String, error['message'] as String, status);
    } on Object {
      return ApiException(
          'HTTP_$status', response.reasonPhrase ?? 'HTTP $status', status);
    }
  }
}
