/// Tiny clock formatting shared by the when-picker chip and the room header
/// tag; avoids pulling in `intl` for one "h:mm a" string.
library;

/// "7:00 PM"-style label. [local] must already be in the display timezone.
String formatClockTime(DateTime local) {
  final hour = local.hour % 12 == 0 ? 12 : local.hour % 12;
  final minute = local.minute.toString().padLeft(2, '0');
  final suffix = local.hour < 12 ? 'AM' : 'PM';
  return '$hour:$minute $suffix';
}
