import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../state/lists_state.dart';
import '../widgets/restaurant_card.dart';
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
  bool _loading = true;
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
      final (list, items) = await lists.openByCode(widget.code);
      if (!mounted) return;
      setState(() {
        _list = list;
        _items = items;
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

  void _copyCode() {
    Clipboard.setData(ClipboardData(text: widget.code));
    ScaffoldMessenger.of(context)
        .showSnackBar(const SnackBar(content: Text('List code copied!')));
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
      appBar: AppBar(title: Text(_list?.name ?? 'List')),
      floatingActionButton: _list == null
          ? null
          : FloatingActionButton.extended(
              onPressed: _addItemDialog,
              icon: const Icon(Icons.add),
              label: const Text('Add a spot'),
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
            const SizedBox(height: 16),
            if (_items.isEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 56),
                child: Column(
                  children: [
                    const Text('🍽️', style: TextStyle(fontSize: 48)),
                    const SizedBox(height: 12),
                    Text(
                      'Nothing here yet — add your first spot.',
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
                    contentPadding: const EdgeInsets.symmetric(
                        horizontal: 20, vertical: 6),
                    leading: Text(
                      emojiForCuisine(item.cuisine ?? ''),
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
