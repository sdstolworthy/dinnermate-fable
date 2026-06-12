import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../state/lists_state.dart';
import '../widgets/restaurant_card.dart';
import '../widgets/room_created_view.dart';
import '../widgets/status_views.dart';

class ListDetailScreen extends StatefulWidget {
  const ListDetailScreen({super.key, required this.code});

  final String code;

  @override
  State<ListDetailScreen> createState() => _ListDetailScreenState();
}

class _ListDetailScreenState extends State<ListDetailScreen> {
  DinnerList? _list;
  List<ListItem> _items = const [];
  bool _isMember = false;
  bool _isOwner = false;
  bool _loading = true;
  bool _joining = false;
  bool _creatingRoom = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final lists = context.read<ListsState>();
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final (list, items, :isMember, :isOwner) =
          await lists.openByCode(widget.code);
      if (!mounted) return;
      setState(() {
        _list = list;
        _items = items;
        _isMember = isMember;
        _isOwner = isOwner;
      });
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } on Exception {
      if (mounted) {
        setState(() => _error = "Couldn't load this list. Check the code?");
      }
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _join() async {
    final lists = context.read<ListsState>();
    final messenger = ScaffoldMessenger.of(context);
    setState(() => _joining = true);
    try {
      await lists.join(widget.code);
      if (!mounted) return;
      await _load();
    } on Exception {
      messenger.showSnackBar(
        const SnackBar(content: Text("Couldn't join the list. Try again?")),
      );
    } finally {
      if (mounted) setState(() => _joining = false);
    }
  }

  Future<void> _confirmLeave() async {
    final lists = context.read<ListsState>();
    final router = GoRouter.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Leave this list?'),
        content: const Text(
            "You'll stop seeing it in My Lists. You can rejoin anytime "
            'with the code.'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('Leave'),
          ),
        ],
      ),
    );
    if (confirmed != true) return;
    try {
      await lists.leave(widget.code);
      if (router.canPop()) {
        router.pop();
      } else {
        router.go('/lists');
      }
    } on Exception {
      messenger.showSnackBar(
        const SnackBar(content: Text("Couldn't leave the list. Try again?")),
      );
    }
  }

  void _showShareSheet() {
    final inviteLink = '${Uri.base.origin}/#/l/${widget.code}';
    showModalBottomSheet<void>(
      context: context,
      showDragHandle: true,
      builder: (sheetContext) {
        final theme = Theme.of(sheetContext);
        void copy(String text, String confirmation) {
          Clipboard.setData(ClipboardData(text: text));
          Navigator.pop(sheetContext);
          ScaffoldMessenger.of(context)
              .showSnackBar(SnackBar(content: Text(confirmation)));
        }

        return SafeArea(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  'Share this list',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.titleMedium
                      ?.copyWith(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 8),
                Text(
                  'Friends with the link or code can join and add spots.',
                  textAlign: TextAlign.center,
                  style: theme.textTheme.bodyMedium
                      ?.copyWith(color: theme.colorScheme.outline),
                ),
                const SizedBox(height: 16),
                ListTile(
                  leading: const Icon(Icons.link_rounded),
                  title: const Text('Copy invite link'),
                  subtitle: Text(
                    inviteLink,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  onTap: () => copy(inviteLink, 'Invite link copied!'),
                ),
                ListTile(
                  leading: const Icon(Icons.copy_rounded),
                  title: const Text('Copy code'),
                  subtitle: Text(widget.code),
                  onTap: () => copy(widget.code, 'List code copied!'),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  void _copyCode() {
    Clipboard.setData(ClipboardData(text: widget.code));
    ScaffoldMessenger.of(context)
        .showSnackBar(const SnackBar(content: Text('List code copied!')));
  }

  /// Spins a swipe room out of this list and hands off to the shared
  /// share/join success flow.
  Future<void> _swipeThisList() async {
    final api = context.read<ApiClient>();
    final navigator = Navigator.of(context);
    final messenger = ScaffoldMessenger.of(context);
    setState(() => _creatingRoom = true);
    try {
      final (room, _) =
          await api.createRoomFromList(widget.code, name: _list?.name);
      if (!mounted) return;
      await navigator.push(MaterialPageRoute<void>(
        builder: (_) => RoomCreatedScreen(room: room),
      ));
    } on ApiException catch (e) {
      messenger.showSnackBar(SnackBar(content: Text(e.message)));
    } on Exception {
      messenger.showSnackBar(
        const SnackBar(content: Text("Couldn't start a room. Try again?")),
      );
    } finally {
      if (mounted) setState(() => _creatingRoom = false);
    }
  }

  Future<void> _addItemDialog() async {
    final lists = context.read<ListsState>();
    final messenger = ScaffoldMessenger.of(context);
    final nameController = TextEditingController();
    final cuisineController = TextEditingController();
    final formKey = GlobalKey<FormState>();
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Add a spot'),
        content: Form(
          key: formKey,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              TextFormField(
                controller: nameController,
                autofocus: true,
                textCapitalization: TextCapitalization.words,
                decoration: const InputDecoration(labelText: 'Name'),
                validator: (value) => (value == null || value.trim().isEmpty)
                    ? 'Enter a name'
                    : null,
              ),
              const SizedBox(height: 12),
              TextFormField(
                controller: cuisineController,
                decoration:
                    const InputDecoration(labelText: 'Cuisine (optional)'),
              ),
            ],
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () {
              if (formKey.currentState?.validate() == true) {
                Navigator.pop(dialogContext, true);
              }
            },
            child: const Text('Add'),
          ),
        ],
      ),
    );
    final name = nameController.text.trim();
    final cuisine = cuisineController.text.trim().toLowerCase();
    nameController.dispose();
    cuisineController.dispose();
    if (confirmed != true || name.isEmpty) return;
    try {
      final item = await lists.addItem(
        widget.code,
        NewListItem(name: name, cuisine: cuisine.isEmpty ? null : cuisine),
      );
      if (!mounted) return;
      setState(() => _items = [..._items, item]);
    } on Exception {
      messenger.showSnackBar(
        const SnackBar(content: Text("Couldn't add that. Try again?")),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(_list?.name ?? 'List'),
        actions: [
          if (_isOwner)
            IconButton(
              onPressed: _showShareSheet,
              tooltip: 'Share list',
              icon: const Icon(Icons.ios_share_rounded),
            ),
          if (_isMember && !_isOwner)
            PopupMenuButton<String>(
              onSelected: (value) {
                if (value == 'leave') _confirmLeave();
              },
              itemBuilder: (context) => const [
                PopupMenuItem(value: 'leave', child: Text('Leave list')),
              ],
            ),
        ],
      ),
      floatingActionButton: _list == null || !_isMember
          ? null
          : FloatingActionButton.extended(
              onPressed: _addItemDialog,
              icon: const Icon(Icons.add),
              label: const Text('Add a spot'),
            ),
      bottomNavigationBar: _list == null || _isMember
          ? null
          : SafeArea(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(24, 8, 24, 16),
                child: FilledButton(
                  onPressed: _joining ? null : _join,
                  style: FilledButton.styleFrom(
                    minimumSize: const Size.fromHeight(52),
                  ),
                  child: const Text('Join this list'),
                ),
              ),
            ),
      body: _body(),
    );
  }

  Widget _body() {
    final theme = Theme.of(context);
    if (_loading) return const CenteredLoader();
    if (_error != null) {
      return FriendlyError(message: _error!, onRetry: _load);
    }
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: ListView(
          padding: const EdgeInsets.fromLTRB(24, 16, 24, 96),
          children: [
            Center(
              child: ActionChip(
                avatar: const Icon(Icons.copy_rounded, size: 18),
                label: Text(
                  'Code ${widget.code}',
                  style: const TextStyle(
                    fontWeight: FontWeight.w700,
                    letterSpacing: 1,
                  ),
                ),
                onPressed: _copyCode,
              ),
            ),
            if (_isMember) ...[
              const SizedBox(height: 16),
              FilledButton(
                onPressed: _creatingRoom ? null : _swipeThisList,
                style: FilledButton.styleFrom(
                  minimumSize: const Size.fromHeight(52),
                ),
                child: _creatingRoom
                    ? const SizedBox(
                        width: 22,
                        height: 22,
                        child: CircularProgressIndicator(strokeWidth: 3),
                      )
                    : const Text('Swipe this list 🍽️'),
              ),
            ],
            const SizedBox(height: 16),
            if (_items.isEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 56),
                child: Column(
                  children: [
                    const Text('🍽️', style: TextStyle(fontSize: 48)),
                    const SizedBox(height: 12),
                    Text(
                      _isMember
                          ? 'Nothing here yet — add your first spot.'
                          : 'Nothing here yet.',
                      textAlign: TextAlign.center,
                      style: theme.textTheme.bodyLarge,
                    ),
                  ],
                ),
              )
            else
              for (final item in _items)
                Card(
                  child: ListTile(
                    contentPadding:
                        const EdgeInsets.symmetric(horizontal: 20, vertical: 6),
                    leading: Text(
                      emojiForCuisine(item.cuisine),
                      style: const TextStyle(fontSize: 26),
                    ),
                    title: Text(
                      item.name,
                      style: theme.textTheme.titleMedium
                          ?.copyWith(fontWeight: FontWeight.w600),
                    ),
                    trailing: item.cuisine == null
                        ? null
                        : CuisineChip(cuisine: item.cuisine!),
                  ),
                ),
          ],
        ),
      ),
    );
  }
}
