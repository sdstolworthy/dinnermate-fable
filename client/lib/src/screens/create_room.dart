import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../api/api_client.dart';
import '../api/models.dart';
import '../time_format.dart';
import '../widgets/restaurant_card.dart';
import '../widgets/room_created_view.dart';

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

enum TravelMode { walking, driving }

enum EatTime { anytime, tonight, pickTime }

/// Resolves the when-picker selection to a UTC instant; null = no meal time.
/// A picked wall-clock time at or before [now] means tomorrow.
DateTime? resolveEatAt(EatTime mode, TimeOfDay? picked, DateTime now) {
  switch (mode) {
    case EatTime.anytime:
      return null;
    case EatTime.tonight:
      return DateTime(now.year, now.month, now.day, 19).toUtc();
    case EatTime.pickTime:
      if (picked == null) return null;
      var candidate =
          DateTime(now.year, now.month, now.day, picked.hour, picked.minute);
      if (!candidate.isAfter(now)) {
        // Keep the wall-clock time when rolling over (day+1 normalizes),
        // rather than adding 24h, which would drift across DST boundaries.
        candidate = DateTime(
            now.year, now.month, now.day + 1, picked.hour, picked.minute);
      }
      return candidate.toUtc();
  }
}

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
  const CreateRoomScreen({super.key, this.now = DateTime.now});

  /// Injectable clock so tests can pin "Tonight" and roll-over resolution.
  final DateTime Function() now;

  @override
  State<CreateRoomScreen> createState() => _CreateRoomScreenState();
}

class _CreateRoomScreenState extends State<CreateRoomScreen> {
  final _name = TextEditingController();
  final _lat = TextEditingController();
  final _lng = TextEditingController();

  _CityPreset _city = _presets.first;
  TravelMode _travelMode = TravelMode.walking;
  EatTime _eatTime = EatTime.anytime;
  TimeOfDay? _pickedTime;
  double _radiusM = 1000;
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
    super.dispose();
  }

  // Walking minutes at 80 m/min; `~/` matches the spec's "1.0 km · ~12 min
  // walk" example (1000/80 = 12.5).
  String get _radiusLabel => switch (_travelMode) {
        TravelMode.walking => '${(_radiusM / 1000).toStringAsFixed(1)} km '
            '· ~${_radiusM ~/ 80} min walk',
        TravelMode.driving => '${(_radiusM / 1000).round()} km',
      };

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
        eatAt: resolveEatAt(_eatTime, _pickedTime, widget.now()),
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

  @override
  Widget build(BuildContext context) {
    final created = _created;
    return Scaffold(
      appBar: AppBar(title: Text(created == null ? 'Start a room' : 'Room ready!')),
      body: created == null ? _form() : RoomCreatedView(room: created),
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

  Future<void> _onEatTimeChanged(Set<EatTime> selection) async {
    final mode = selection.first;
    if (mode != EatTime.pickTime) {
      setState(() => _eatTime = mode);
      return;
    }
    final picked = await showTimePicker(
      context: context,
      initialTime: _pickedTime ?? TimeOfDay.fromDateTime(widget.now()),
    );
    if (!mounted) return;
    setState(() {
      if (picked != null) _pickedTime = picked;
      // Cancelling keeps the previous selection unless a time already exists.
      if (_pickedTime != null) _eatTime = EatTime.pickTime;
    });
  }

  String _eatAtChipLabel(DateTime eatAtUtc) {
    final local = eatAtUtc.toLocal();
    final now = widget.now();
    final isToday = local.year == now.year &&
        local.month == now.month &&
        local.day == now.day;
    final time = formatClockTime(local);
    return isToday ? time : 'Tomorrow $time';
  }

  Widget _form() {
    final theme = Theme.of(context);
    final eatAt = resolveEatAt(_eatTime, _pickedTime, widget.now());
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
            _section('When are you eating?'),
            SegmentedButton<EatTime>(
              segments: const [
                ButtonSegment(value: EatTime.anytime, label: Text('Anytime')),
                ButtonSegment(value: EatTime.tonight, label: Text('Tonight')),
                ButtonSegment(
                  value: EatTime.pickTime,
                  label: Text('Pick a time'),
                ),
              ],
              selected: {_eatTime},
              onSelectionChanged: _onEatTimeChanged,
            ),
            if (eatAt != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: Chip(
                    avatar: const Text('🕖'),
                    label: Text(_eatAtChipLabel(eatAt)),
                  ),
                ),
              ),
            _section('How far'),
            SegmentedButton<TravelMode>(
              segments: const [
                ButtonSegment(
                  value: TravelMode.walking,
                  label: Text('🚶 Walking'),
                ),
                ButtonSegment(
                  value: TravelMode.driving,
                  label: Text('🚗 Driving'),
                ),
              ],
              selected: {_travelMode},
              onSelectionChanged: (modes) => setState(() {
                _travelMode = modes.first;
                _radiusM = _travelMode == TravelMode.walking ? 1000 : 5000;
              }),
            ),
            const SizedBox(height: 12),
            Text(
              'Within $_radiusLabel',
              style: theme.textTheme.bodyMedium
                  ?.copyWith(color: theme.colorScheme.outline),
            ),
            Slider(
              value: _radiusM,
              min: _travelMode == TravelMode.walking ? 250 : 2000,
              max: _travelMode == TravelMode.walking ? 2000 : 40000,
              divisions: _travelMode == TravelMode.walking ? 7 : 38,
              label: _radiusLabel,
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

}
