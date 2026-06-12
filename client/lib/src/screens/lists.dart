import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../state/lists_state.dart';
import '../widgets/status_views.dart';

class ListsScreen extends StatefulWidget {
  const ListsScreen({super.key});

  @override
  State<ListsScreen> createState() => _ListsScreenState();
}

class _ListsScreenState extends State<ListsScreen> {
  final _code = TextEditingController();

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  void _openByCode() {
    final code = _code.text.trim().toUpperCase();
    if (code.isEmpty) return;
    context.push('/l/$code');
  }

  Future<void> _createDialog() async {
    final lists = context.read<ListsState>();
    final router = GoRouter.of(context);
    final messenger = ScaffoldMessenger.of(context);
    final controller = TextEditingController();
    final formKey = GlobalKey<FormState>();
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('New list'),
        content: Form(
          key: formKey,
          child: TextFormField(
            controller: controller,
            autofocus: true,
            textCapitalization: TextCapitalization.sentences,
            decoration: const InputDecoration(
              labelText: 'List name',
              hintText: 'Date night spots',
            ),
            validator: (value) =>
                (value == null || value.trim().isEmpty) ? 'Enter a name' : null,
            onFieldSubmitted: (_) {
              if (formKey.currentState?.validate() == true) {
                Navigator.pop(dialogContext, true);
              }
            },
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
            child: const Text('Create'),
          ),
        ],
      ),
    );
    final name = controller.text.trim();
    controller.dispose();
    if (confirmed != true || name.isEmpty) return;
    try {
      final list = await lists.createList(name);
      router.push('/l/${list.code}');
    } on Exception {
      messenger.showSnackBar(
        const SnackBar(content: Text("Couldn't create the list. Try again?")),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<ListsState>();
    return Scaffold(
      appBar: AppBar(title: const Text('My lists')),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: _createDialog,
        icon: const Icon(Icons.add),
        label: const Text('New list'),
      ),
      body: _body(state),
    );
  }

  Widget _body(ListsState state) {
    final theme = Theme.of(context);
    if (state.loading && state.mine == null) return const CenteredLoader();
    if (state.errorMessage != null && state.mine == null) {
      return FriendlyError(
        message: state.errorMessage!,
        onRetry: state.loadMine,
      );
    }
    final mine = state.mine ?? const [];
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: ListView(
          padding: const EdgeInsets.fromLTRB(24, 16, 24, 96),
          children: [
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _code,
                    textCapitalization: TextCapitalization.characters,
                    decoration: const InputDecoration(
                      labelText: 'Have a list code?',
                      hintText: 'ABC234',
                    ),
                    onSubmitted: (_) => _openByCode(),
                  ),
                ),
                const SizedBox(width: 12),
                FilledButton.tonal(
                  onPressed: _openByCode,
                  child: const Text('Open'),
                ),
              ],
            ),
            const SizedBox(height: 20),
            if (mine.isEmpty)
              Padding(
                padding: const EdgeInsets.only(top: 56),
                child: Column(
                  children: [
                    const Text('📋', style: TextStyle(fontSize: 48)),
                    const SizedBox(height: 12),
                    Text(
                      'No lists yet — start one for your favorite spots.',
                      textAlign: TextAlign.center,
                      style: theme.textTheme.bodyLarge,
                    ),
                  ],
                ),
              )
            else
              for (final myList in mine)
                Card(
                  child: ListTile(
                    contentPadding:
                        const EdgeInsets.symmetric(horizontal: 20, vertical: 8),
                    leading: const Text('📋', style: TextStyle(fontSize: 26)),
                    title: Row(
                      children: [
                        Flexible(
                          child: Text(
                            myList.list.name,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.titleMedium
                                ?.copyWith(fontWeight: FontWeight.w700),
                          ),
                        ),
                        if (!myList.isOwner) ...[
                          const SizedBox(width: 8),
                          const _SharedBadge(),
                        ],
                      ],
                    ),
                    subtitle: Text('Code ${myList.list.code}'),
                    trailing: const Icon(Icons.chevron_right_rounded),
                    onTap: () => context.push('/l/${myList.list.code}'),
                  ),
                ),
          ],
        ),
      ),
    );
  }
}

class _SharedBadge extends StatelessWidget {
  const _SharedBadge();

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: scheme.secondaryContainer,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        'shared',
        style: Theme.of(context)
            .textTheme
            .labelSmall
            ?.copyWith(color: scheme.onSecondaryContainer),
      ),
    );
  }
}
