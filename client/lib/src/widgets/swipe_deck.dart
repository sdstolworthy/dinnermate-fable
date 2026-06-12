import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../api/models.dart';
import 'restaurant_card.dart';

/// Draggable card stack. The top card follows the finger and rotates up to
/// ±8°; releasing past 30% of the deck width flings it off and commits the
/// swipe, otherwise it springs back. [SwipeDeckState.like]/[SwipeDeckState.nope]
/// drive the same animation from buttons. Tapping the top card flips it over
/// (Y-rotation) to a details back face; advancing the deck resets to front.
class SwipeDeck extends StatefulWidget {
  const SwipeDeck({
    super.key,
    required this.restaurants,
    required this.onSwipe,
    required this.onDeckEnd,
  });

  final List<Restaurant> restaurants;
  final void Function(Restaurant restaurant, bool liked) onSwipe;
  final VoidCallback onDeckEnd;

  @override
  State<SwipeDeck> createState() => SwipeDeckState();
}

class SwipeDeckState extends State<SwipeDeck>
    with TickerProviderStateMixin {
  static const _maxAngle = 8 * math.pi / 180;

  late final AnimationController _controller;
  late final AnimationController _flip;
  double _dragX = 0;
  double _width = 1;
  int _index = 0;

  Restaurant? get _top =>
      _index < widget.restaurants.length ? widget.restaurants[_index] : null;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 220),
    );
    _flip = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 300),
    );
  }

  void _toggleFlip() {
    if (_top == null) return;
    switch (_flip.status) {
      case AnimationStatus.dismissed:
      case AnimationStatus.reverse:
        _flip.forward();
      case AnimationStatus.completed:
      case AnimationStatus.forward:
        _flip.reverse();
    }
  }

  void like() => _flingOff(true);

  void nope() => _flingOff(false);

  void _flingOff(bool liked) {
    if (_top == null || _controller.isAnimating) return;
    _animateTo(liked ? _width * 1.3 : -_width * 1.3, thenCommit: liked);
  }

  void _onDragUpdate(DragUpdateDetails details) {
    if (_controller.isAnimating) return;
    setState(() => _dragX += details.delta.dx);
  }

  void _onDragEnd(DragEndDetails details) {
    if (_controller.isAnimating) return;
    if (_dragX.abs() > _width * 0.3) {
      _flingOff(_dragX > 0);
    } else {
      _animateTo(0);
    }
  }

  void _animateTo(double target, {bool? thenCommit}) {
    final animation = _controller
        .drive(CurveTween(curve: Curves.easeOut))
        .drive(Tween(begin: _dragX, end: target));
    void tick() => setState(() => _dragX = animation.value);
    animation.addListener(tick);
    _controller.forward(from: 0).whenComplete(() {
      animation.removeListener(tick);
      if (thenCommit != null) _commit(thenCommit);
    });
  }

  void _commit(bool liked) {
    final swiped = widget.restaurants[_index];
    _flip.value = 0;
    setState(() {
      _index++;
      _dragX = 0;
    });
    widget.onSwipe(swiped, liked);
    if (_index >= widget.restaurants.length) widget.onDeckEnd();
  }

  @override
  void dispose() {
    _controller.dispose();
    _flip.dispose();
    super.dispose();
  }

  /// 3D Y-rotation between front and back; the child swaps at the halfway
  /// point (and the back is pre-mirrored) so text is never shown reversed.
  Widget _flippableCard(Restaurant restaurant) {
    return AnimatedBuilder(
      animation: _flip,
      builder: (context, _) {
        final showBack = _flip.value >= 0.5;
        return Transform(
          alignment: Alignment.center,
          transform: Matrix4.identity()
            ..setEntry(3, 2, 0.001)
            ..rotateY(_flip.value * math.pi),
          child: showBack
              ? Transform(
                  alignment: Alignment.center,
                  transform: Matrix4.identity()..rotateY(math.pi),
                  child: RestaurantCardBack(restaurant: restaurant),
                )
              : RestaurantCard(restaurant: restaurant),
        );
      },
    );
  }

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(builder: (context, constraints) {
      _width = constraints.maxWidth;
      final top = _top;
      if (top == null) return const SizedBox.expand();
      final next = _index + 1 < widget.restaurants.length
          ? widget.restaurants[_index + 1]
          : null;
      final progress = _dragX / (_width * 0.3);
      final likeOpacity = progress > 0 ? math.min(progress, 1.0) : 0.0;
      final nopeOpacity = progress < 0 ? math.min(-progress, 1.0) : 0.0;
      final angle = (_dragX / _width).clamp(-1.0, 1.0).toDouble() * _maxAngle;
      return Stack(
        fit: StackFit.expand,
        children: [
          if (next != null)
            Transform.scale(
              scale: 0.95,
              child: RestaurantCard(restaurant: next),
            ),
          GestureDetector(
            onTap: _toggleFlip,
            onHorizontalDragUpdate: _onDragUpdate,
            onHorizontalDragEnd: _onDragEnd,
            child: Transform.translate(
              offset: Offset(_dragX, 0),
              child: Transform.rotate(
                angle: angle,
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    _flippableCard(top),
                    Positioned(
                      top: 24,
                      left: 24,
                      child: _SwipeBadge(
                        label: 'LIKE',
                        color: const Color(0xFF4CAF7D),
                        opacity: likeOpacity,
                      ),
                    ),
                    Positioned(
                      top: 24,
                      right: 24,
                      child: _SwipeBadge(
                        label: 'NOPE',
                        color: const Color(0xFFE5604C),
                        opacity: nopeOpacity,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        ],
      );
    });
  }
}

class _SwipeBadge extends StatelessWidget {
  const _SwipeBadge({
    required this.label,
    required this.color,
    required this.opacity,
  });

  final String label;
  final Color color;
  final double opacity;

  @override
  Widget build(BuildContext context) {
    return Opacity(
      opacity: opacity,
      child: Transform.rotate(
        angle: label == 'LIKE' ? -0.2 : 0.2,
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 6),
          decoration: BoxDecoration(
            border: Border.all(color: color, width: 4),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Text(
            label,
            style: TextStyle(
              color: color,
              fontSize: 28,
              fontWeight: FontWeight.w900,
              letterSpacing: 2,
            ),
          ),
        ),
      ),
    );
  }
}
