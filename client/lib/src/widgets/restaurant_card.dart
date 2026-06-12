import 'package:flutter/material.dart';

import '../api/models.dart';

const Map<String, String> _cuisineEmoji = {
  'mexican': '🌮',
  'thai': '🍜',
  'italian': '🍕',
  'japanese': '🍣',
  'indian': '🍛',
  'american': '🍔',
  'chinese': '🥡',
  'mediterranean': '🥙',
  'korean': '🥘',
  'vietnamese': '🍲',
};

const Map<String, List<Color>> _cuisineGradients = {
  'mexican': [Color(0xFFFFE2C8), Color(0xFFFFB59E)],
  'thai': [Color(0xFFE6F4D7), Color(0xFFB7E0A8)],
  'italian': [Color(0xFFFFE0E0), Color(0xFFF6B0A8)],
  'japanese': [Color(0xFFE3EEFF), Color(0xFFB6CCF2)],
  'indian': [Color(0xFFFFF0C2), Color(0xFFF7CD8A)],
  'american': [Color(0xFFFFE8D1), Color(0xFFEFB68F)],
  'chinese': [Color(0xFFFFE3DB), Color(0xFFF3A99B)],
  'mediterranean': [Color(0xFFE0F4F1), Color(0xFFA8DDD3)],
  'korean': [Color(0xFFFBE2EC), Color(0xFFEFAFC8)],
  'vietnamese': [Color(0xFFE8F1DC), Color(0xFFC2DFAE)],
};

const List<Color> _fallbackGradient = [Color(0xFFF1E8DF), Color(0xFFD9C8B8)];

String emojiForCuisine(String cuisine) => _cuisineEmoji[cuisine] ?? '🍽️';

LinearGradient gradientForCuisine(String cuisine) => LinearGradient(
      begin: Alignment.topLeft,
      end: Alignment.bottomRight,
      colors: _cuisineGradients[cuisine] ?? _fallbackGradient,
    );

class CuisineChip extends StatelessWidget {
  const CuisineChip({super.key, required this.cuisine});

  final String cuisine;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: theme.colorScheme.secondaryContainer,
        borderRadius: BorderRadius.circular(999),
      ),
      child: Text(
        '${emojiForCuisine(cuisine)} $cuisine',
        style: theme.textTheme.labelLarge
            ?.copyWith(color: theme.colorScheme.onSecondaryContainer),
      ),
    );
  }
}

/// Big, soft deck card: photo (or a cuisine-keyed gradient with a large
/// emoji), name, cuisine chip, price, rating and address.
class RestaurantCard extends StatelessWidget {
  const RestaurantCard({super.key, required this.restaurant});

  final Restaurant restaurant;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Container(
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        borderRadius: BorderRadius.circular(28),
        boxShadow: const [
          BoxShadow(
            color: Color(0x1A000000),
            blurRadius: 24,
            offset: Offset(0, 8),
          ),
        ],
      ),
      clipBehavior: Clip.antiAlias,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Expanded(child: _photo()),
          Padding(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  restaurant.name,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.headlineSmall
                      ?.copyWith(fontWeight: FontWeight.w700),
                ),
                const SizedBox(height: 10),
                Wrap(
                  spacing: 10,
                  runSpacing: 8,
                  crossAxisAlignment: WrapCrossAlignment.center,
                  children: [
                    CuisineChip(cuisine: restaurant.cuisine),
                    Text(
                      '\$' * restaurant.priceLevel,
                      style: theme.textTheme.titleMedium
                          ?.copyWith(fontWeight: FontWeight.w700),
                    ),
                    Text(
                      '★ ${restaurant.rating.toStringAsFixed(1)} '
                      '(${restaurant.ratingCount})',
                      style: theme.textTheme.titleSmall,
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Text(
                  restaurant.address,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodySmall
                      ?.copyWith(color: theme.colorScheme.outline),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _photo() {
    final placeholder = DecoratedBox(
      decoration: BoxDecoration(gradient: gradientForCuisine(restaurant.cuisine)),
      child: Center(
        child: Text(
          emojiForCuisine(restaurant.cuisine),
          style: const TextStyle(fontSize: 96),
        ),
      ),
    );
    final url = restaurant.photoUrl;
    if (url == null) return placeholder;
    return Image.network(
      url,
      fit: BoxFit.cover,
      errorBuilder: (context, error, stackTrace) => placeholder,
    );
  }
}
