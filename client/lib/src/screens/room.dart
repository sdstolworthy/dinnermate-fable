import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../state/room_state.dart';
import '../time_format.dart';
import '../widgets/status_views.dart';
import '../widgets/swipe_deck.dart';

class RoomScreen extends StatefulWidget {
  const RoomScreen({super.key, required this.code, this.initialDisplayName});

  final String code;

  /// Set when arriving from the home join sheet: joins automatically.
  final String? initialDisplayName;

  @override
  State<RoomScreen> createState() => _RoomScreenState();
}

class _RoomScreenState extends State<RoomScreen> {
  final _deckKey = GlobalKey<SwipeDeckState>();
  final _formKey = GlobalKey<FormState>();
  final _name = TextEditingController();
  bool _autoJoinTried = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _maybeAutoJoin(RoomState state) {
    if (_autoJoinTried || state.loading || state.room == null || state.joined) {
      return;
    }
    final name = widget.initialDisplayName?.trim() ?? '';
    if (name.isEmpty) return;
    _autoJoinTried = true;
    Future.microtask(() => state.join(name));
  }

  void _join(RoomState state) {
    if (_formKey.currentState?.validate() != true) return;
    state.join(_name.text.trim());
  }

  void _copyCode() {
    Clipboard.setData(ClipboardData(text: widget.code));
    ScaffoldMessenger.of(context)
        .showSnackBar(const SnackBar(content: Text('Room code copied!')));
  }

  @override
  Widget build(BuildContext context) {
    final state = context.watch<RoomState>();
    _maybeAutoJoin(state);
    return Scaffold(
      appBar: AppBar(
        title: Text(state.room?.name ?? 'Dinnermate'),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 12),
            child: ActionChip(
              avatar: const Icon(Icons.copy_rounded, size: 16),
              label: Text(
                widget.code,
                style: const TextStyle(
                  fontWeight: FontWeight.w700,
                  letterSpacing: 2,
                ),
              ),
              onPressed: _copyCode,
            ),
          ),
        ],
      ),
      body: _body(state),
    );
  }

  Widget _body(RoomState state) {
    if (state.loading) return const CenteredLoader();
    if (state.notFound) return _endedView();
    if (state.errorMessage != null) {
      return FriendlyError(message: state.errorMessage!, onRetry: state.load);
    }
    final content = !state.joined
        ? _joinView(state)
        : state.deckDone
            ? _doneView()
            : _swipeView(state);
    final header = _roomHeader(state);
    if (header == null) return content;
    return Column(children: [header, Expanded(child: content)]);
  }

  /// Compact context strip under the app bar's code chip: who's here, and
  /// which list this room came from.
  Widget? _roomHeader(RoomState state) {
    final theme = Theme.of(context);
    final participants = _participantsLine(state.participants);
    final sourceListName = state.room?.sourceListName;
    final eatAt = state.room?.eatAt;
    if (participants == null && sourceListName == null && eatAt == null) {
      return null;
    }
    final style =
        theme.textTheme.bodySmall?.copyWith(color: theme.colorScheme.outline);
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 10, 20, 0),
      child: Column(
        children: [
          if (participants != null)
            Text('👥 $participants', textAlign: TextAlign.center, style: style),
          if (sourceListName != null)
            Padding(
              padding: EdgeInsets.only(top: participants == null ? 0 : 2),
              child: Text(
                'From list: $sourceListName',
                textAlign: TextAlign.center,
                style: style?.copyWith(fontStyle: FontStyle.italic),
              ),
            ),
          if (eatAt != null)
            Padding(
              padding: EdgeInsets.only(
                top: participants == null && sourceListName == null ? 0 : 2,
              ),
              child: Text(
                '🕖 Eating at ${formatClockTime(eatAt.toLocal())}',
                textAlign: TextAlign.center,
                style: style,
              ),
            ),
        ],
      ),
    );
  }

  String? _participantsLine(List<String> names) {
    if (names.isEmpty) return null;
    if (names.length <= 3) return names.join(', ');
    return '${names.take(2).join(', ')} +${names.length - 2}';
  }

  Widget _endedView() {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('🌙', style: TextStyle(fontSize: 48)),
            const SizedBox(height: 12),
            Text(
              'This room has ended',
              textAlign: TextAlign.center,
              style: theme.textTheme.headlineSmall
                  ?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            Text(
              'Rooms quietly wind down after a while. Start a fresh one '
              'whenever you’re hungry.',
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyLarge
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            const SizedBox(height: 24),
            FilledButton.tonal(
              onPressed: () => context.go('/'),
              child: const Text('Back home'),
            ),
          ],
        ),
      ),
    );
  }

  Widget _joinView(RoomState state) {
    final theme = Theme.of(context);
    final room = state.room!;
    final radiusKm = (room.radiusM / 1000).toStringAsFixed(1);
    final cuisines =
        room.cuisines.isEmpty ? 'all cuisines' : room.cuisines.join(' · ');
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 440),
          child: Card(
            child: Padding(
              padding: const EdgeInsets.all(28),
              child: Form(
                key: _formKey,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Text(
                      room.name ?? 'Dinner at ${room.locationLabel}',
                      textAlign: TextAlign.center,
                      style: theme.textTheme.headlineSmall
                          ?.copyWith(fontWeight: FontWeight.w700),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      '${room.locationLabel} · within $radiusKm km · $cuisines',
                      textAlign: TextAlign.center,
                      style: theme.textTheme.bodyMedium
                          ?.copyWith(color: theme.colorScheme.outline),
                    ),
                    const SizedBox(height: 24),
                    TextFormField(
                      controller: _name,
                      autofocus: true,
                      textCapitalization: TextCapitalization.words,
                      decoration:
                          const InputDecoration(labelText: 'Your name'),
                      validator: (value) =>
                          (value == null || value.trim().isEmpty)
                              ? 'Enter your name'
                              : null,
                      onFieldSubmitted: (_) => _join(state),
                    ),
                    if (state.joinError != null) ...[
                      const SizedBox(height: 12),
                      Text(
                        state.joinError!,
                        textAlign: TextAlign.center,
                        style: TextStyle(color: theme.colorScheme.error),
                      ),
                    ],
                    const SizedBox(height: 24),
                    FilledButton(
                      onPressed: state.joining ? null : () => _join(state),
                      child: const Text('Join & swipe'),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _swipeView(RoomState state) {
    return Column(
      children: [
        Expanded(
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 480),
              child: Padding(
                padding: const EdgeInsets.fromLTRB(20, 12, 20, 4),
                child: SwipeDeck(
                  key: _deckKey,
                  restaurants: state.deck,
                  onSwipe: (restaurant, liked) =>
                      state.swipe(restaurant.id, liked),
                  // RoomState.deckIndex already flips the screen to the done
                  // view after the last swipe.
                  onDeckEnd: () {},
                ),
              ),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 14),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              _ActionCircle(
                emoji: '✖️',
                onTap: () => _deckKey.currentState?.nope(),
              ),
              const SizedBox(width: 28),
              _ActionCircle(
                emoji: '❤️',
                onTap: () => _deckKey.currentState?.like(),
              ),
            ],
          ),
        ),
        _MatchTicker(
          state: state,
          onTap: () => context.push('/r/${widget.code}/matches'),
        ),
      ],
    );
  }

  Widget _doneView() {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(32),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text('🎉', style: TextStyle(fontSize: 64)),
            const SizedBox(height: 16),
            Text(
              "You're all swiped out!",
              textAlign: TextAlign.center,
              style: theme.textTheme.headlineSmall
                  ?.copyWith(fontWeight: FontWeight.w700),
            ),
            const SizedBox(height: 8),
            Text(
              'See what the table loved.',
              textAlign: TextAlign.center,
              style: theme.textTheme.bodyLarge
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            const SizedBox(height: 28),
            FilledButton(
              onPressed: () => context.push('/r/${widget.code}/matches'),
              child: const Padding(
                padding: EdgeInsets.symmetric(horizontal: 24),
                child: Text('See matches'),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _ActionCircle extends StatelessWidget {
  const _ActionCircle({required this.emoji, required this.onTap});

  final String emoji;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Material(
      color: Theme.of(context).colorScheme.surface,
      shape: const CircleBorder(),
      elevation: 3,
      shadowColor: const Color(0x33000000),
      child: InkWell(
        onTap: onTap,
        customBorder: const CircleBorder(),
        child: SizedBox(
          width: 64,
          height: 64,
          child: Center(
            child: Text(emoji, style: const TextStyle(fontSize: 26)),
          ),
        ),
      ),
    );
  }
}

class _MatchTicker extends StatelessWidget {
  const _MatchTicker({required this.state, required this.onTap});

  final RoomState state;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final top = state.matches.isEmpty ? null : state.matches.first;
    final count = state.matches.length;
    return Material(
      color: scheme.primaryContainer,
      child: InkWell(
        onTap: onTap,
        child: SafeArea(
          top: false,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
            child: Row(
              children: [
                const Text('🏆', style: TextStyle(fontSize: 22)),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    top == null
                        ? 'No matches yet — keep swiping!'
                        : '${top.restaurant.name} · ${top.likeCount} liked',
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: theme.textTheme.titleSmall?.copyWith(
                      color: scheme.onPrimaryContainer,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                ),
                if (top != null) ...[
                  const SizedBox(width: 8),
                  Text(
                    '$count ${count == 1 ? 'match' : 'matches'}',
                    style: theme.textTheme.bodySmall
                        ?.copyWith(color: scheme.onPrimaryContainer),
                  ),
                ],
                Icon(Icons.chevron_right_rounded,
                    color: scheme.onPrimaryContainer),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
