import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../api/api_client.dart';
import '../api/models.dart';

/// Standalone page wrapper for [RoomCreatedView], for entry points that
/// navigate to the success flow (e.g. "Swipe this list").
class RoomCreatedScreen extends StatelessWidget {
  const RoomCreatedScreen({super.key, required this.room});

  final Room room;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Room ready!')),
      body: RoomCreatedView(room: room),
    );
  }
}

/// Post-create success view: big shareable code, copy-link chip, and the
/// name prompt that joins the creator and takes them to the room. Reused by
/// the create-room form and the from-list flow.
class RoomCreatedView extends StatefulWidget {
  const RoomCreatedView({super.key, required this.room});

  final Room room;

  @override
  State<RoomCreatedView> createState() => _RoomCreatedViewState();
}

class _RoomCreatedViewState extends State<RoomCreatedView> {
  final _name = TextEditingController();
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _startSwiping() async {
    final room = widget.room;
    final name = _name.text.trim();
    if (name.isEmpty) {
      setState(() => _error = 'Tell us your name first');
      return;
    }
    final api = context.read<ApiClient>();
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await api.joinRoom(room.code, name);
      if (!mounted) return;
      context.go('/r/${room.code}');
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } on Exception {
      if (mounted) {
        setState(() => _error = "Couldn't join. Check your connection?");
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  void _copyLink(String link) {
    Clipboard.setData(ClipboardData(text: link));
    ScaffoldMessenger.of(context)
        .showSnackBar(const SnackBar(content: Text('Link copied!')));
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final room = widget.room;
    final origin = Uri.base.scheme.startsWith('http') ? Uri.base.origin : '';
    final link = '$origin/#/r/${room.code}';
    return Center(
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 460),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Text('🎉',
                  textAlign: TextAlign.center, style: TextStyle(fontSize: 56)),
              const SizedBox(height: 12),
              Text(
                'Share this code with your table',
                textAlign: TextAlign.center,
                style: theme.textTheme.titleMedium
                    ?.copyWith(color: theme.colorScheme.outline),
              ),
              const SizedBox(height: 8),
              Text(
                room.code,
                textAlign: TextAlign.center,
                style: theme.textTheme.displayMedium?.copyWith(
                  fontWeight: FontWeight.w800,
                  letterSpacing: 10,
                ),
              ),
              const SizedBox(height: 16),
              Center(
                child: ActionChip(
                  avatar: const Icon(Icons.copy_rounded, size: 18),
                  label: const Text('Copy link'),
                  onPressed: () => _copyLink(link),
                ),
              ),
              const SizedBox(height: 8),
              Text(
                link,
                textAlign: TextAlign.center,
                style: theme.textTheme.bodySmall
                    ?.copyWith(color: theme.colorScheme.outline),
              ),
              const SizedBox(height: 40),
              TextField(
                controller: _name,
                textCapitalization: TextCapitalization.words,
                decoration: const InputDecoration(labelText: 'Your name'),
                onSubmitted: (_) => _startSwiping(),
              ),
              if (_error != null) ...[
                const SizedBox(height: 12),
                Text(
                  _error!,
                  textAlign: TextAlign.center,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ],
              const SizedBox(height: 16),
              SizedBox(
                height: 64,
                child: FilledButton(
                  onPressed: _busy ? null : _startSwiping,
                  child: const Text('Start swiping'),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
