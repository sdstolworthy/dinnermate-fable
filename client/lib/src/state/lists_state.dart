import 'package:flutter/foundation.dart';

import '../api/api_client.dart';
import '../api/models.dart';

class ListsState extends ChangeNotifier {
  ListsState(this._api);

  final ApiClient _api;

  List<MyList>? mine;
  bool loading = false;
  String? errorMessage;

  Future<void> loadMine() async {
    loading = true;
    errorMessage = null;
    notifyListeners();
    try {
      mine = await _api.getMyLists();
    } on ApiException catch (e) {
      errorMessage = e.message;
    } on Exception {
      errorMessage = "Couldn't load your lists. Check your connection?";
    }
    loading = false;
    notifyListeners();
  }

  Future<DinnerList> createList(String name) async {
    final list = await _api.createList(name);
    mine = [...?mine, MyList(list: list, isOwner: true)];
    notifyListeners();
    return list;
  }

  Future<(DinnerList, List<ListItem>, {bool isMember, bool isOwner})>
      openByCode(String code) => _api.getList(code);

  Future<(DinnerList, bool isOwner)> join(String code) async {
    final result = await _api.joinList(code);
    final (list, isOwner) = result;
    if (mine != null && !mine!.any((m) => m.list.code == list.code)) {
      mine = [...mine!, MyList(list: list, isOwner: isOwner)];
      notifyListeners();
    }
    return result;
  }

  Future<void> leave(String code) async {
    await _api.leaveList(code);
    if (mine != null) {
      mine = mine!.where((m) => m.list.code != code).toList();
      notifyListeners();
    }
  }

  Future<ListItem> addItem(String code, NewListItem item) =>
      _api.addListItem(code, item);
}
