import 'package:flutter/material.dart';

/// Full-width, 64px-tall action button with a leading emoji. The home
/// screen's primary affordance: big, soft, unmissable.
class BigButton extends StatelessWidget {
  const BigButton({
    super.key,
    required this.emoji,
    required this.label,
    required this.onPressed,
    this.tonal = false,
  });

  final String emoji;
  final String label;
  final VoidCallback onPressed;
  final bool tonal;

  @override
  Widget build(BuildContext context) {
    final child = Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        Text(emoji, style: const TextStyle(fontSize: 26)),
        const SizedBox(width: 12),
        Text(label),
      ],
    );
    return SizedBox(
      width: double.infinity,
      height: 64,
      child: tonal
          ? FilledButton.tonal(onPressed: onPressed, child: child)
          : FilledButton(onPressed: onPressed, child: child),
    );
  }
}
