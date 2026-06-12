import 'package:flutter/foundation.dart';

import '../api/api_client.dart';
import '../api/models.dart';

class ListsState extends ChangeNotifier {
  ListsState(this._api);

  final ApiClient _api;

  List<DinnerList>? mine;
  bool loading = false;
  String? errorMessage;

  Future<void> loadMine() async {
    loading = true;
    errorMessage = null;
    notifyListeners();
    try {
      // Task 7 migrates `mine` to MyList records; project to lists for now.
      mine = (await _api.getMyLists()).map((m) => m.list).toList();
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
    mine = [...?mine, list];
    notifyListeners();
    return list;
  }

  // Task 7 surfaces isMember/isOwner in the detail screen; drop them for now.
  Future<(DinnerList, List<ListItem>)> openByCode(String code) async {
    final (list, items, isMember: _, isOwner: _) = await _api.getList(code);
    return (list, items);
  }

  Future<ListItem> addItem(String code, NewListItem item) =>
      _api.addListItem(code, item);
}
