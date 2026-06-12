import 'package:flutter/material.dart';

import '../api/models.dart';
import 'restaurant_card.dart';

/// Ranked match row: rank, avatar, name, "N liked" pill and an optional
/// add-to-list action.
class MatchTile extends StatelessWidget {
  const MatchTile({
    super.key,
    required this.rank,
    required this.restaurant,
    required this.likeCount,
    this.onAddToList,
    this.onTap,
  });

  final int rank;
  final Restaurant restaurant;
  final int likeCount;
  final VoidCallback? onAddToList;

  /// Fired when the tile body is tapped (the add-to-list button stays its
  /// own tap target).
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return Card(
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 14),
          child: Row(
            children: [
              SizedBox(
                width: 28,
                child: Text(
                  '$rank',
                  style: theme.textTheme.titleLarge?.copyWith(
                    color: scheme.primary,
                    fontWeight: FontWeight.w800,
                  ),
                ),
              ),
              _avatar(),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      restaurant.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleMedium
                          ?.copyWith(fontWeight: FontWeight.w700),
                    ),
                    const SizedBox(height: 2),
                    Text(
                      // v3 Task 6: interim — show only the known parts.
                      [
                        if (restaurant.cuisine != null) restaurant.cuisine!,
                        if (restaurant.priceLevel != null)
                          '\$' * restaurant.priceLevel!,
                      ].join(' · '),
                      style: theme.textTheme.bodySmall
                          ?.copyWith(color: scheme.outline),
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                decoration: BoxDecoration(
                  color: scheme.primaryContainer,
                  borderRadius: BorderRadius.circular(999),
                ),
                child: Text(
                  '$likeCount liked',
                  style: theme.textTheme.labelLarge
                      ?.copyWith(color: scheme.onPrimaryContainer),
                ),
              ),
              if (onAddToList != null)
                IconButton(
                  onPressed: onAddToList,
                  tooltip: 'Add to a list',
                  icon: const Icon(Icons.playlist_add_rounded),
                ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _avatar() {
    final cuisine = restaurant.cuisine ?? '';
    final emoji = Center(
      child: Text(
        emojiForCuisine(cuisine),
        style: const TextStyle(fontSize: 26),
      ),
    );
    final url = restaurant.photoUrl;
    return Container(
      width: 52,
      height: 52,
      clipBehavior: Clip.antiAlias,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(16),
        gradient: gradientForCuisine(cuisine),
      ),
      child: url == null
          ? emoji
          : Image.network(
              url,
              fit: BoxFit.cover,
              errorBuilder: (context, error, stackTrace) => emoji,
            ),
    );
  }
}
