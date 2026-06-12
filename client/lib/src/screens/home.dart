import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../recent_rooms.dart';
import '../widgets/big_button.dart';

final _upperCaseFormatter = TextInputFormatter.withFunction(
  (oldValue, newValue) => newValue.copyWith(text: newValue.text.toUpperCase()),
);

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  List<RecentRoomEntry> _recent = const [];

  @override
  void initState() {
    super.initState();
    _loadRecent();
  }

  Future<void> _loadRecent() async {
    final entries = await context.read<RecentRooms>().all();
    if (mounted) setState(() => _recent = entries);
  }

  void _showJoinSheet(BuildContext context) {
    showModalBottomSheet<void>(
      context: context,
      isScrollControlled: true,
      showDragHandle: true,
      builder: (sheetContext) => const _JoinSheet(),
    );
  }

  Widget _jumpBackIn(ThemeData theme) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Jump back in',
          style: theme.textTheme.titleMedium
              ?.copyWith(fontWeight: FontWeight.w700),
        ),
        const SizedBox(height: 10),
        Wrap(
          spacing: 8,
          runSpacing: 8,
          children: [
            for (final entry in _recent)
              ActionChip(
                avatar: const Text('🍽️'),
                label: Text('${entry.label} · ${entry.code}'),
                onPressed: () => context.push('/r/${entry.code}'),
              ),
          ],
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Scaffold(
      body: SafeArea(
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(28),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 420),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Text(
                    '🍽️',
                    textAlign: TextAlign.center,
                    style: TextStyle(fontSize: 72),
                  ),
                  const SizedBox(height: 16),
                  Text(
                    'Dinnermate',
                    textAlign: TextAlign.center,
                    style: theme.textTheme.displaySmall
                        ?.copyWith(fontWeight: FontWeight.w800),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Swipe together. Eat together.',
                    textAlign: TextAlign.center,
                    style: theme.textTheme.titleMedium
                        ?.copyWith(color: theme.colorScheme.outline),
                  ),
                  const SizedBox(height: 56),
                  BigButton(
                    emoji: '🍽️',
                    label: 'Start a room',
                    onPressed: () => context.push('/create'),
                  ),
                  const SizedBox(height: 16),
                  BigButton(
                    emoji: '🔑',
                    label: 'Join a room',
                    tonal: true,
                    onPressed: () => _showJoinSheet(context),
                  ),
                  if (_recent.isNotEmpty) ...[
                    const SizedBox(height: 28),
                    _jumpBackIn(theme),
                  ],
                  const SizedBox(height: 16),
                  BigButton(
                    emoji: '📋',
                    label: 'My lists',
                    tonal: true,
                    onPressed: () => context.push('/lists'),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _JoinSheet extends StatefulWidget {
  const _JoinSheet();

  @override
  State<_JoinSheet> createState() => _JoinSheetState();
}

class _JoinSheetState extends State<_JoinSheet> {
  final _formKey = GlobalKey<FormState>();
  final _code = TextEditingController();
  final _name = TextEditingController();

  @override
  void dispose() {
    _code.dispose();
    _name.dispose();
    super.dispose();
  }

  void _join() {
    if (_formKey.currentState?.validate() != true) return;
    final router = GoRouter.of(context);
    final code = _code.text.trim().toUpperCase();
    final name = _name.text.trim();
    Navigator.of(context).pop();
    router.go('/r/$code', extra: name);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.fromLTRB(
        24,
        8,
        24,
        24 + MediaQuery.viewInsetsOf(context).bottom,
      ),
      child: Form(
        key: _formKey,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              'Join a room',
              textAlign: TextAlign.center,
              style: Theme.of(context)
                  .textTheme
                  .titleLarge
                  ?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 20),
            TextFormField(
              controller: _code,
              autofocus: true,
              textCapitalization: TextCapitalization.characters,
              inputFormatters: [_upperCaseFormatter],
              decoration: const InputDecoration(
                labelText: 'Room code',
                hintText: 'ABC234',
              ),
              style: const TextStyle(
                letterSpacing: 4,
                fontWeight: FontWeight.w700,
              ),
              validator: (value) => (value == null || value.trim().isEmpty)
                  ? 'Enter the room code'
                  : null,
            ),
            const SizedBox(height: 16),
            TextFormField(
              controller: _name,
              textCapitalization: TextCapitalization.words,
              decoration: const InputDecoration(labelText: 'Your name'),
              validator: (value) => (value == null || value.trim().isEmpty)
                  ? 'Enter your name'
                  : null,
              onFieldSubmitted: (_) => _join(),
            ),
            const SizedBox(height: 24),
            FilledButton(onPressed: _join, child: const Text('Join')),
          ],
        ),
      ),
    );
  }
}
