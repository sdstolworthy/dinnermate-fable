import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../state/lists_state.dart';
import '../state/room_state.dart';
import '../widgets/match_tile.dart';
import '../widgets/status_views.dart';

class MatchesScreen extends StatelessWidget {
  const MatchesScreen({super.key, required this.code});

  final String code;

  void _showAddToList(BuildContext context, Restaurant restaurant) {
    final api = context.read<ApiClient>();
    final messenger = ScaffoldMessenger.of(context);
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (_) => ChangeNotifierProvider(
        create: (_) => ListsState(api)..loadMine(),
        child: _AddToListSheet(
          restaurant: restaurant,
          onAdded: (listName) => messenger.showSnackBar(
            SnackBar(content: Text('Added to $listName')),
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<RoomState>();
    return Scaffold(
      appBar: AppBar(title: const Text('Matches')),
      body: _body(context, state),
    );
  }

  Widget _body(BuildContext context, RoomState state) {
    final theme = Theme.of(context);
    if (state.loading && state.matches.isEmpty) return const CenteredLoader();
    if (state.errorMessage != null && state.matches.isEmpty) {
      return FriendlyError(message: state.errorMessage!, onRetry: state.load);
    }
    return RefreshIndicator(
      onRefresh: state.refreshMatches,
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 560),
          child: ListView(
            physics: const AlwaysScrollableScrollPhysics(),
            padding: const EdgeInsets.fromLTRB(12, 8, 12, 32),
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(12, 8, 12, 12),
                child: Text(
                  '${state.participantCount} '
                  '${state.participantCount == 1 ? 'person' : 'people'} swiping',
                  style: theme.textTheme.titleSmall
                      ?.copyWith(color: theme.colorScheme.outline),
                ),
              ),
              if (state.matches.isEmpty)
                Padding(
                  padding: const EdgeInsets.only(top: 72),
                  child: Column(
                    children: [
                      const Text('🤞', style: TextStyle(fontSize: 48)),
                      const SizedBox(height: 12),
                      Text(
                        'No matches yet — keep swiping!',
                        style: theme.textTheme.bodyLarge,
                      ),
                    ],
                  ),
                )
              else
                for (final (index, entry) in state.matches.indexed)
                  MatchTile(
                    rank: index + 1,
                    restaurant: entry.restaurant,
                    likeCount: entry.likeCount,
                    onAddToList: () =>
                        _showAddToList(context, entry.restaurant),
                  ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AddToListSheet extends StatefulWidget {
  const _AddToListSheet({required this.restaurant, required this.onAdded});

  final Restaurant restaurant;
  final ValueChanged<String> onAdded;

  @override
  State<_AddToListSheet> createState() => _AddToListSheetState();
}

class _AddToListSheetState extends State<_AddToListSheet> {
  final _newList = TextEditingController();
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _newList.dispose();
    super.dispose();
  }

  NewListItem get _item {
    final restaurant = widget.restaurant;
    return NewListItem(
      name: restaurant.name,
      cuisine: restaurant.cuisine,
      priceLevel: restaurant.priceLevel,
      rating: restaurant.rating,
      address: restaurant.address,
      photoUrl: restaurant.photoUrl,
      sourceRestaurantId: restaurant.id,
    );
  }

  Future<void> _addTo(DinnerList list) async {
    final lists = context.read<ListsState>();
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await lists.addItem(list.code, _item);
      if (!mounted) return;
      Navigator.of(context).pop();
      widget.onAdded(list.name);
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } on Exception {
      if (mounted) setState(() => _error = "Couldn't add that. Try again?");
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _createAndAdd() async {
    final name = _newList.text.trim();
    if (name.isEmpty) return;
    final lists = context.read<ListsState>();
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final list = await lists.createList(name);
      await lists.addItem(list.code, _item);
      if (!mounted) return;
      Navigator.of(context).pop();
      widget.onAdded(list.name);
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } on Exception {
      if (mounted) {
        setState(() => _error = "Couldn't create the list. Try again?");
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final lists = context.watch<ListsState>();
    final theme = Theme.of(context);
    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        0,
        24,
        24 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Add "${widget.restaurant.name}" to a list',
              textAlign: TextAlign.center,
              style: theme.textTheme.titleMedium
                  ?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 16),
            if (lists.loading)
              const Padding(
                padding: EdgeInsets.all(24),
                child: Center(child: CircularProgressIndicator()),
              )
            else if (lists.errorMessage != null) ...[
              Text(lists.errorMessage!, textAlign: TextAlign.center),
              TextButton(
                onPressed: lists.loadMine,
                child: const Text('Try again'),
              ),
            ] else if (lists.mine?.isEmpty ?? true)
              Text(
                'No lists yet — start one below.',
                textAlign: TextAlign.center,
                style: theme.textTheme.bodyMedium
                    ?.copyWith(color: theme.colorScheme.outline),
              )
            else
              for (final list in lists.mine!)
                ListTile(
                  leading: const Text('📋', style: TextStyle(fontSize: 22)),
                  title: Text(list.name),
                  subtitle: Text('Code ${list.code}'),
                  onTap: _busy ? null : () => _addTo(list),
                ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _newList,
                    textCapitalization: TextCapitalization.sentences,
                    decoration: const InputDecoration(labelText: 'New list'),
                    onSubmitted: (_) => _createAndAdd(),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton.tonal(
                  onPressed: _busy ? null : _createAndAdd,
                  child: const Text('Create'),
                ),
              ],
            ),
            if (_error != null) ...[
              const SizedBox(height: 12),
              Text(
                _error!,
                textAlign: TextAlign.center,
                style: TextStyle(color: theme.colorScheme.error),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
