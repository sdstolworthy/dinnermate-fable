import 'package:flutter/material.dart';

/// Temporary screens so routing is wired end-to-end; Task 9 replaces these.
class _Placeholder extends StatelessWidget {
  const _Placeholder(this.label);

  final String label;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(
        child: Text(label, style: Theme.of(context).textTheme.headlineMedium),
      ),
    );
  }
}

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context) => const _Placeholder('Home');
}

class CreateRoomScreen extends StatelessWidget {
  const CreateRoomScreen({super.key});

  @override
  Widget build(BuildContext context) => const _Placeholder('Create room');
}

class RoomScreen extends StatelessWidget {
  const RoomScreen({super.key, required this.code});

  final String code;

  @override
  Widget build(BuildContext context) => _Placeholder('Room $code');
}

class MatchesScreen extends StatelessWidget {
  const MatchesScreen({super.key, required this.code});

  final String code;

  @override
  Widget build(BuildContext context) => _Placeholder('Matches $code');
}

class ListsScreen extends StatelessWidget {
  const ListsScreen({super.key});

  @override
  Widget build(BuildContext context) => const _Placeholder('Lists');
}

class ListDetailScreen extends StatelessWidget {
  const ListDetailScreen({super.key, required this.code});

  final String code;

  @override
  Widget build(BuildContext context) => _Placeholder('List $code');
}
