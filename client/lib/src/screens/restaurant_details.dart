import 'package:flutter/material.dart';
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';
import 'package:provider/provider.dart';
import 'package:url_launcher/url_launcher.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../widgets/open_badge.dart';
import '../widgets/restaurant_card.dart';
import '../widgets/status_views.dart';

DateTime _defaultNowUtc() => DateTime.now().toUtc();

Widget _osmMap(BuildContext context, Restaurant restaurant) {
  // Only called when both coordinates are known (see _body).
  final point = LatLng(restaurant.lat!, restaurant.lng!);
  return FlutterMap(
    options: MapOptions(
      initialCenter: point,
      initialZoom: 15,
      interactionOptions: const InteractionOptions(flags: InteractiveFlag.none),
    ),
    children: [
      TileLayer(
        urlTemplate: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
        userAgentPackageName: 'co.stolworthy.dinnermate',
      ),
      MarkerLayer(
        markers: [
          Marker(
            point: point,
            width: 40,
            height: 40,
            alignment: Alignment.topCenter,
            child: const Icon(Icons.location_pin,
                size: 40, color: Color(0xFFC4452B)),
          ),
        ],
      ),
    ],
  );
}

class RestaurantDetailsScreen extends StatefulWidget {
  const RestaurantDetailsScreen({
    super.key,
    required this.code,
    required this.restaurantId,
    this.nowUtc = _defaultNowUtc,
    this.mapBuilder = _osmMap,
  });

  final String code;
  final String restaurantId;

  /// Injectable clock; must return a UTC instant.
  final DateTime Function() nowUtc;

  /// Builds the map block. Tests inject a placeholder so no tiles are
  /// fetched over the network.
  final Widget Function(BuildContext, Restaurant) mapBuilder;

  @override
  State<RestaurantDetailsScreen> createState() =>
      _RestaurantDetailsScreenState();
}

class _RestaurantDetailsScreenState extends State<RestaurantDetailsScreen> {
  RestaurantDetails? _details;
  bool _loading = true;
  String? _error;

  static const _dayNames = [
    'Sunday',
    'Monday',
    'Tuesday',
    'Wednesday',
    'Thursday',
    'Friday',
    'Saturday',
  ];

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    final api = context.read<ApiClient>();
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final details =
          await api.getRestaurantDetails(widget.code, widget.restaurantId);
      if (mounted) setState(() => _details = details);
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } on Exception {
      if (mounted) {
        setState(() => _error = "Couldn't load this place. Try again?");
      }
    } finally {
      if (mounted) setState(() => _loading = false);
    }
  }

  Future<void> _open(Uri uri) async {
    final messenger = ScaffoldMessenger.of(context);
    final opened = await launchUrl(uri);
    if (!opened) {
      messenger.showSnackBar(
        const SnackBar(content: Text("Couldn't open that link.")),
      );
    }
  }

  Uri _directionsUri(RestaurantDetails details) {
    final mapsUrl = details.mapsUrl;
    if (mapsUrl != null) return Uri.parse(mapsUrl);
    final restaurant = details.restaurant;
    final lat = restaurant.lat;
    final lng = restaurant.lng;
    final query = lat == null || lng == null
        ? Uri.encodeComponent(restaurant.name)
        : '$lat,$lng';
    return Uri.parse(
        'https://www.google.com/maps/search/?api=1&query=$query');
  }

  /// Day index (0=Sun..6=Sat) in the restaurant's local time when the
  /// offset is known; device-local otherwise.
  int _today(Restaurant restaurant) {
    final offset = restaurant.utcOffsetMinutes;
    final local = offset == null
        ? widget.nowUtc().toLocal()
        : widget.nowUtc().add(Duration(minutes: offset));
    return local.weekday % 7;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(_details?.restaurant.name ?? 'Details')),
      body: _body(),
    );
  }

  Widget _body() {
    if (_loading) return const CenteredLoader();
    if (_error != null) {
      return FriendlyError(message: _error!, onRetry: _load);
    }
    final details = _details!;
    final restaurant = details.restaurant;
    final theme = Theme.of(context);
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: ListView(
          padding: const EdgeInsets.only(bottom: 48),
          children: [
            _heroHeader(restaurant),
            Padding(
              padding: const EdgeInsets.fromLTRB(24, 20, 24, 0),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: [
                      Expanded(
                        child: Text(
                          restaurant.name,
                          style: theme.textTheme.headlineSmall
                              ?.copyWith(fontWeight: FontWeight.w800),
                        ),
                      ),
                      const SizedBox(width: 12),
                      OpenBadge(
                        restaurant.hours,
                        restaurant.utcOffsetMinutes,
                        nowUtc: widget.nowUtc,
                      ),
                    ],
                  ),
                  const SizedBox(height: 6),
                  Text(
                    restaurant.address,
                    style: theme.textTheme.bodyLarge
                        ?.copyWith(color: theme.colorScheme.outline),
                  ),
                  // v3 Task 6: dedicated no-coords layout; interim hide.
                  if (restaurant.lat != null && restaurant.lng != null) ...[
                    const SizedBox(height: 16),
                    ClipRRect(
                      borderRadius: BorderRadius.circular(20),
                      child: SizedBox(
                        height: 200,
                        child: widget.mapBuilder(context, restaurant),
                      ),
                    ),
                  ],
                  _actionRow(details),
                  if (restaurant.hours != null) _hoursBlock(restaurant),
                  if (details.reviews.isNotEmpty) _reviews(details.reviews),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _heroHeader(Restaurant restaurant) {
    // v3 Task 6: null cuisine gets the dedicated neutral warm gradient.
    final cuisine = restaurant.cuisine ?? '';
    return Container(
      height: 160,
      decoration: BoxDecoration(gradient: gradientForCuisine(cuisine)),
      child: Center(
        child: Text(
          emojiForCuisine(cuisine),
          style: const TextStyle(fontSize: 72),
        ),
      ),
    );
  }

  Widget _actionRow(RestaurantDetails details) {
    final website = details.website;
    final phone = details.phone;
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: Wrap(
        spacing: 12,
        runSpacing: 12,
        children: [
          if (website != null)
            OutlinedButton.icon(
              onPressed: () => _open(Uri.parse(website)),
              icon: const Icon(Icons.language_rounded),
              label: const Text('Website'),
            ),
          if (phone != null)
            OutlinedButton.icon(
              onPressed: () => _open(Uri(scheme: 'tel', path: phone)),
              icon: const Icon(Icons.call_rounded),
              label: const Text('Call'),
            ),
          OutlinedButton.icon(
            onPressed: () => _open(_directionsUri(details)),
            icon: const Icon(Icons.directions_rounded),
            label: const Text('Directions'),
          ),
        ],
      ),
    );
  }

  Widget _hoursBlock(Restaurant restaurant) {
    final theme = Theme.of(context);
    final hours = restaurant.hours!;
    final today = _today(restaurant);
    return Padding(
      padding: const EdgeInsets.only(top: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Hours',
            style: theme.textTheme.titleMedium
                ?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 8),
          for (var day = 0; day < 7; day++)
            _hoursRow(day, hours, bold: day == today),
        ],
      ),
    );
  }

  Widget _hoursRow(int day, List<HoursPeriod> hours, {required bool bold}) {
    final theme = Theme.of(context);
    final periods = hours.where((p) => p.day == day).toList()
      ..sort((a, b) => a.open.compareTo(b.open));
    final spans = periods.isEmpty
        ? 'Closed'
        : periods.map((p) => '${p.open}–${p.close}').join(', ');
    final style = theme.textTheme.bodyLarge?.copyWith(
      fontWeight: bold ? FontWeight.w800 : FontWeight.w400,
    );
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Row(
        children: [
          SizedBox(width: 120, child: Text(_dayNames[day], style: style)),
          Expanded(child: Text(spans, style: style)),
        ],
      ),
    );
  }

  Widget _reviews(List<Review> reviews) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(top: 24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Reviews',
            style: theme.textTheme.titleMedium
                ?.copyWith(fontWeight: FontWeight.w700),
          ),
          const SizedBox(height: 4),
          for (final review in reviews)
            Card(
              margin: const EdgeInsets.only(top: 8),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        Expanded(
                          child: Text(
                            review.author,
                            style: theme.textTheme.titleSmall
                                ?.copyWith(fontWeight: FontWeight.w700),
                          ),
                        ),
                        Text(
                          '★ ${review.rating}',
                          style: theme.textTheme.titleSmall?.copyWith(
                            color: const Color(0xFFB7791F),
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    Text(review.text, style: theme.textTheme.bodyMedium),
                    if (review.relativeTime != null) ...[
                      const SizedBox(height: 6),
                      Text(
                        review.relativeTime!,
                        style: theme.textTheme.bodySmall
                            ?.copyWith(color: theme.colorScheme.outline),
                      ),
                    ],
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}
