import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import 'api/api_client.dart';
import 'screens/create_room.dart';
import 'screens/home.dart';
import 'screens/list_detail.dart';
import 'screens/lists.dart';
import 'screens/matches.dart';
import 'screens/room.dart';
import 'state/lists_state.dart';
import 'state/room_state.dart';

GoRouter buildRouter() {
  return GoRouter(
    routes: [
      GoRoute(path: '/', builder: (context, state) => const HomeScreen()),
      GoRoute(
        path: '/create',
        builder: (context, state) => const CreateRoomScreen(),
      ),
      GoRoute(
        path: '/r/:code',
        builder: (context, state) {
          final code = state.pathParameters['code']!.toUpperCase();
          final extra = state.extra;
          return ChangeNotifierProvider(
            create: (context) =>
                RoomState(context.read<ApiClient>(), code)..load(),
            child: RoomScreen(
              code: code,
              initialDisplayName: extra is String ? extra : null,
            ),
          );
        },
      ),
      GoRoute(
        path: '/r/:code/matches',
        builder: (context, state) {
          final code = state.pathParameters['code']!.toUpperCase();
          return ChangeNotifierProvider(
            create: (context) => RoomState(context.read<ApiClient>(), code)
              ..load()
              ..startPolling(),
            child: MatchesScreen(code: code),
          );
        },
      ),
      GoRoute(
        path: '/lists',
        builder: (context, state) => ChangeNotifierProvider(
          create: (context) => ListsState(context.read<ApiClient>())
            ..loadMine(),
          child: const ListsScreen(),
        ),
      ),
      GoRoute(
        path: '/l/:code',
        builder: (context, state) => ChangeNotifierProvider(
          create: (context) => ListsState(context.read<ApiClient>()),
          child: ListDetailScreen(
            code: state.pathParameters['code']!.toUpperCase(),
          ),
        ),
      ),
    ],
  );
}
