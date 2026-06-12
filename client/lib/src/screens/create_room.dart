import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:go_router/go_router.dart';
import 'package:provider/provider.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../widgets/restaurant_card.dart';

class _CityPreset {
  const _CityPreset(this.label, this.lat, this.lng);

  final String label;
  final double lat;
  final double lng;
}

const _presets = [
  _CityPreset('Salt Lake City', 40.760, -111.890),
  _CityPreset('San Francisco', 37.7749, -122.4194),
  _CityPreset('New York', 40.7128, -74.0060),
  _CityPreset('Austin', 30.2672, -97.7431),
];

const _cuisines = [
  'mexican',
  'thai',
  'italian',
  'japanese',
  'indian',
  'american',
  'chinese',
  'mediterranean',
  'korean',
  'vietnamese',
];

class CreateRoomScreen extends StatefulWidget {
  const CreateRoomScreen({super.key});

  @override
  State<CreateRoomScreen> createState() => _CreateRoomScreenState();
}

class _CreateRoomScreenState extends State<CreateRoomScreen> {
  final _name = TextEditingController();
  final _lat = TextEditingController();
  final _lng = TextEditingController();
  final _shareName = TextEditingController();

  _CityPreset _city = _presets.first;
  double _radiusM = 5000;
  final Set<String> _selectedCuisines = {};
  RangeValues _price = const RangeValues(1, 4);
  double _minRating = 0;
  bool _busy = false;
  String? _error;
  Room? _created;

  @override
  void dispose() {
    _name.dispose();
    _lat.dispose();
    _lng.dispose();
    _shareName.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    final api = context.read<ApiClient>();
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      final (room, _) = await api.createRoom(CreateRoomRequest(
        name: _name.text.trim().isEmpty ? null : _name.text.trim(),
        locationLabel: _city.label,
        lat: double.tryParse(_lat.text.trim()) ?? _city.lat,
        lng: double.tryParse(_lng.text.trim()) ?? _city.lng,
        radiusM: _radiusM.round(),
        cuisines: _selectedCuisines.toList()..sort(),
        priceMin: _price.start.round(),
        priceMax: _price.end.round(),
        minRating: _minRating,
      ));
      if (!mounted) return;
      setState(() => _created = room);
    } on ApiException catch (e) {
      if (mounted) setState(() => _error = e.message);
    } on Exception {
      if (mounted) {
        setState(
            () => _error = "Couldn't create the room. Check your connection?");
      }
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _startSwiping() async {
    final room = _created!;
    final name = _shareName.text.trim();
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
    final created = _created;
    return Scaffold(
      appBar: AppBar(title: Text(created == null ? 'Start a room' : 'Room ready!')),
      body: created == null ? _form() : _share(created),
    );
  }

  Widget _section(String title) {
    return Padding(
      padding: const EdgeInsets.only(top: 28, bottom: 8),
      child: Text(
        title,
        style: Theme.of(context)
            .textTheme
            .titleMedium
            ?.copyWith(fontWeight: FontWeight.w700),
      ),
    );
  }

  Widget _form() {
    final theme = Theme.of(context);
    final priceLabel =
        '${'\$' * _price.start.round()} – ${'\$' * _price.end.round()}';
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: ListView(
          padding: const EdgeInsets.fromLTRB(24, 16, 24, 40),
          children: [
            TextField(
              controller: _name,
              textCapitalization: TextCapitalization.sentences,
              decoration: const InputDecoration(
                labelText: 'Room name (optional)',
                hintText: 'Friday night crew',
              ),
            ),
            _section('Where'),
            DropdownMenu<_CityPreset>(
              initialSelection: _city,
              expandedInsets: EdgeInsets.zero,
              label: const Text('City'),
              dropdownMenuEntries: [
                for (final preset in _presets)
                  DropdownMenuEntry(value: preset, label: preset.label),
              ],
              onSelected: (preset) {
                if (preset != null) setState(() => _city = preset);
              },
            ),
            ExpansionTile(
              tilePadding: EdgeInsets.zero,
              shape: const Border(),
              title: Text('Advanced: exact coordinates',
                  style: theme.textTheme.bodyMedium),
              children: [
                Padding(
                  padding: const EdgeInsets.only(bottom: 12),
                  child: Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _lat,
                          keyboardType: const TextInputType.numberWithOptions(
                              decimal: true, signed: true),
                          decoration:
                              const InputDecoration(labelText: 'Latitude'),
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: TextField(
                          controller: _lng,
                          keyboardType: const TextInputType.numberWithOptions(
                              decimal: true, signed: true),
                          decoration:
                              const InputDecoration(labelText: 'Longitude'),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            _section('How far'),
            Text(
              'Within ${(_radiusM / 1000).toStringAsFixed(1)} km',
              style: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            Slider(
              value: _radiusM,
              min: 500,
              max: 25000,
              divisions: 49,
              label: '${(_radiusM / 1000).toStringAsFixed(1)} km',
              onChanged: (value) => setState(() => _radiusM = value),
            ),
            _section('Cuisines'),
            Text(
              'Pick none for everything',
              style: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final cuisine in _cuisines)
                  FilterChip(
                    label: Text('${emojiForCuisine(cuisine)} $cuisine'),
                    selected: _selectedCuisines.contains(cuisine),
                    onSelected: (selected) => setState(() {
                      if (selected) {
                        _selectedCuisines.add(cuisine);
                      } else {
                        _selectedCuisines.remove(cuisine);
                      }
                    }),
                  ),
              ],
            ),
            _section('Price'),
            Text(
              priceLabel,
              style: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            RangeSlider(
              values: _price,
              min: 1,
              max: 4,
              divisions: 3,
              labels: RangeLabels(
                '\$' * _price.start.round(),
                '\$' * _price.end.round(),
              ),
              onChanged: (values) => setState(() => _price = values),
            ),
            _section('Minimum rating'),
            Text(
              _minRating == 0 ? 'Any rating' : '★ ${_minRating.toStringAsFixed(1)}+',
              style: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            Slider(
              value: _minRating,
              min: 0,
              max: 5,
              divisions: 10,
              label: _minRating == 0 ? 'Any' : _minRating.toStringAsFixed(1),
              onChanged: (value) => setState(() => _minRating = value),
            ),
            const SizedBox(height: 24),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: Text(
                  _error!,
                  textAlign: TextAlign.center,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              ),
            SizedBox(
              height: 64,
              child: FilledButton(
                onPressed: _busy ? null : _create,
                child: _busy
                    ? const SizedBox(
                        width: 24,
                        height: 24,
                        child: CircularProgressIndicator(strokeWidth: 3),
                      )
                    : const Text('Create room 🎉'),
              ),
            ),
          ],
        ),
      ),
    );
  }

  Widget _share(Room room) {
    final theme = Theme.of(context);
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
                controller: _shareName,
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
