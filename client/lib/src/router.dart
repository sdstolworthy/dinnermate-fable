import 'package:go_router/go_router.dart';

import 'screens/placeholders.dart';

GoRouter buildRouter() {
  return GoRouter(
    routes: [
      GoRoute(path: '/', builder: (context, state) => const HomeScreen()),
      GoRoute(
          path: '/create',
          builder: (context, state) => const CreateRoomScreen()),
      GoRoute(
        path: '/r/:code',
        builder: (context, state) =>
            RoomScreen(code: state.pathParameters['code']!),
      ),
      GoRoute(
        path: '/r/:code/matches',
        builder: (context, state) =>
            MatchesScreen(code: state.pathParameters['code']!),
      ),
      GoRoute(path: '/lists', builder: (context, state) => const ListsScreen()),
      GoRoute(
        path: '/l/:code',
        builder: (context, state) =>
            ListDetailScreen(code: state.pathParameters['code']!),
      ),
    ],
  );
}
