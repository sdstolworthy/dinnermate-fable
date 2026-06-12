import 'package:flutter/material.dart';
import 'package:http/http.dart' as http;
import 'package:provider/provider.dart';

import 'src/api/api_client.dart';
import 'src/identity.dart';
import 'src/recent_rooms.dart';
import 'src/router.dart';
import 'src/theme.dart';

void main() {
  runApp(DinnermateApp());
}

class DinnermateApp extends StatelessWidget {
  DinnermateApp({super.key});

  final _router = buildRouter();

  @override
  Widget build(BuildContext context) {
    return MultiProvider(
      providers: [
        Provider<KeyValueStore>(create: (_) => SharedPrefsStore()),
        Provider<Identity>(
            create: (context) => Identity(context.read<KeyValueStore>())),
        Provider<RecentRooms>(
            create: (context) => RecentRooms(context.read<KeyValueStore>())),
        Provider<ApiClient>(
          create: (context) => ApiClient(
            '${const String.fromEnvironment('API_BASE_URL', defaultValue: '/api')}/v1',
            http.Client(),
            context.read<Identity>(),
          ),
        ),
      ],
      child: MaterialApp.router(
        title: 'Dinnermate',
        theme: buildTheme(),
        routerConfig: _router,
      ),
    );
  }
}
