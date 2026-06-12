/// Open/closed computation mirroring dinnermate-core's `hours.rs` exactly.
library;

import 'api/models.dart';

sealed class OpenStatus {
  const OpenStatus();
}

class OpenNow extends OpenStatus {
  const OpenNow({required this.until});

  /// "HH:MM" local closing time of the period currently open.
  final String until;

  @override
  bool operator ==(Object other) => other is OpenNow && other.until == until;

  @override
  int get hashCode => Object.hash(OpenNow, until);
}

class ClosedNow extends OpenStatus {
  const ClosedNow({this.opensNext});

  /// "HH:MM" of the next opening, or null if there is none at all.
  final String? opensNext;

  @override
  bool operator ==(Object other) =>
      other is ClosedNow && other.opensNext == opensNext;

  @override
  int get hashCode => Object.hash(ClosedNow, opensNext);
}

class UnknownHours extends OpenStatus {
  const UnknownHours();

  @override
  bool operator ==(Object other) => other is UnknownHours;

  @override
  int get hashCode => (UnknownHours).hashCode;
}

/// Computes the status at [nowUtc] in the restaurant's local time
/// (`nowUtc + utcOffsetMinutes`). Open is inclusive, close exclusive;
/// `close < open` means the period spans midnight into the next day.
OpenStatus openStatusFor(
  List<HoursPeriod>? hours,
  int? utcOffsetMinutes,
  DateTime nowUtc,
) {
  if (hours == null || utcOffsetMinutes == null) return const UnknownHours();

  final local = nowUtc.toUtc().add(Duration(minutes: utcOffsetMinutes));
  final today = local.weekday % 7; // DateTime: Mon=1..Sun=7 -> 0=Sun..6=Sat
  final nowMinutes = local.hour * 60 + local.minute;

  for (final period in hours) {
    final open = _toMinutes(period.open);
    final close = _toMinutes(period.close);
    final spansMidnight = close < open;
    if (period.day == today) {
      final openNow = spansMidnight
          ? nowMinutes >= open
          : nowMinutes >= open && nowMinutes < close;
      if (openNow) return OpenNow(until: period.close);
    }
    // The tail of yesterday's midnight-crossing period.
    if (spansMidnight &&
        period.day == (today + 6) % 7 &&
        nowMinutes < close) {
      return OpenNow(until: period.close);
    }
  }

  return ClosedNow(opensNext: _nextOpening(hours, today, nowMinutes));
}

String? _nextOpening(List<HoursPeriod> hours, int today, int nowMinutes) {
  String? earliestOn(int day, {required int after}) {
    String? best;
    int? bestMinutes;
    for (final period in hours) {
      if (period.day != day) continue;
      final open = _toMinutes(period.open);
      if (open <= after) continue;
      if (bestMinutes == null || open < bestMinutes) {
        bestMinutes = open;
        best = period.open;
      }
    }
    return best;
  }

  final laterToday = earliestOn(today, after: nowMinutes);
  if (laterToday != null) return laterToday;
  for (var offset = 1; offset <= 7; offset++) {
    final next = earliestOn((today + offset) % 7, after: -1);
    if (next != null) return next;
  }
  return null;
}

int _toMinutes(String hhmm) {
  final parts = hhmm.split(':');
  return int.parse(parts[0]) * 60 + int.parse(parts[1]);
}
