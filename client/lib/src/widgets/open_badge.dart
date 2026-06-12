import 'package:flutter/material.dart';

import '../api/models.dart';
import '../hours.dart';

DateTime _defaultNowUtc() => DateTime.now().toUtc();

/// Pill chip showing the live open/closed state computed from snapshot
/// hours. Renders nothing when the hours are unknown.
class OpenBadge extends StatelessWidget {
  const OpenBadge(
    this.hours,
    this.utcOffsetMinutes, {
    super.key,
    this.nowUtc = _defaultNowUtc,
  });

  final List<HoursPeriod>? hours;
  final int? utcOffsetMinutes;

  /// Injectable clock; must return a UTC instant.
  final DateTime Function() nowUtc;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return switch (openStatusFor(hours, utcOffsetMinutes, nowUtc())) {
      OpenNow(:final until) => _chip(
          context,
          'Open · until $until',
          background: const Color(0xFFDCEFE2),
          foreground: const Color(0xFF1E6B43),
        ),
      ClosedNow(:final opensNext) => _chip(
          context,
          opensNext == null ? 'Closed' : 'Closed · opens $opensNext',
          background: scheme.surfaceContainerHighest,
          foreground: scheme.onSurfaceVariant,
        ),
      UnknownHours() => const SizedBox.shrink(),
    };
  }

  Widget _chip(
    BuildContext context,
    String label, {
    required Color background,
    required Color foreground,
  }) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: background,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        label,
        style: Theme.of(context)
            .textTheme
            .labelLarge
            ?.copyWith(color: foreground, fontWeight: FontWeight.w600),
      ),
    );
  }
}
