/// Wire DTOs mirroring the server's `/api/v1` contract (snake_case JSON).
library;

double _asDouble(Object? value) => (value as num).toDouble();

/// One opening span; day 0=Sun..6=Sat, times "HH:MM" restaurant-local.
class HoursPeriod {
  const HoursPeriod({
    required this.day,
    required this.open,
    required this.close,
  });

  final int day;
  final String open;
  final String close;

  factory HoursPeriod.fromJson(Map<String, dynamic> json) => HoursPeriod(
        day: json['day'] as int,
        open: json['open'] as String,
        close: json['close'] as String,
      );

  Map<String, dynamic> toJson() => {'day': day, 'open': open, 'close': close};
}

class Restaurant {
  const Restaurant({
    required this.id,
    required this.name,
    required this.cuisine,
    required this.priceLevel,
    required this.rating,
    required this.ratingCount,
    required this.address,
    this.photoUrl,
    required this.lat,
    required this.lng,
    this.hours,
    this.utcOffsetMinutes,
  });

  final String id;
  final String name;
  final String cuisine;
  final int priceLevel;
  final double rating;
  final int ratingCount;
  final String address;
  final String? photoUrl;
  final double lat;
  final double lng;

  /// Null = unknown (also when a v1 server omits the key entirely).
  final List<HoursPeriod>? hours;
  final int? utcOffsetMinutes;

  factory Restaurant.fromJson(Map<String, dynamic> json) => Restaurant(
        id: json['id'] as String,
        name: json['name'] as String,
        cuisine: json['cuisine'] as String,
        priceLevel: json['price_level'] as int,
        rating: _asDouble(json['rating']),
        ratingCount: json['rating_count'] as int,
        address: json['address'] as String,
        photoUrl: json['photo_url'] as String?,
        lat: _asDouble(json['lat']),
        lng: _asDouble(json['lng']),
        hours: (json['hours'] as List?)
            ?.map((e) => HoursPeriod.fromJson(e as Map<String, dynamic>))
            .toList(),
        utcOffsetMinutes: json['utc_offset_minutes'] as int?,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'name': name,
        'cuisine': cuisine,
        'price_level': priceLevel,
        'rating': rating,
        'rating_count': ratingCount,
        'address': address,
        'photo_url': photoUrl,
        'lat': lat,
        'lng': lng,
        'hours': hours?.map((p) => p.toJson()).toList(),
        'utc_offset_minutes': utcOffsetMinutes,
      };
}

class Review {
  const Review({
    required this.author,
    required this.rating,
    required this.text,
    this.relativeTime,
  });

  final String author;
  final int rating;
  final String text;
  final String? relativeTime;

  factory Review.fromJson(Map<String, dynamic> json) => Review(
        author: json['author'] as String,
        rating: json['rating'] as int,
        text: json['text'] as String,
        relativeTime: json['relative_time'] as String?,
      );

  Map<String, dynamic> toJson() => {
        'author': author,
        'rating': rating,
        'text': text,
        'relative_time': relativeTime,
      };
}

/// Response body of `GET /rooms/{code}/restaurants/{id}/details`.
class RestaurantDetails {
  const RestaurantDetails({
    required this.restaurant,
    this.website,
    this.phone,
    this.mapsUrl,
    required this.reviews,
  });

  final Restaurant restaurant;
  final String? website;
  final String? phone;
  final String? mapsUrl;
  final List<Review> reviews;

  factory RestaurantDetails.fromJson(Map<String, dynamic> json) =>
      RestaurantDetails(
        restaurant:
            Restaurant.fromJson(json['restaurant'] as Map<String, dynamic>),
        website: json['website'] as String?,
        phone: json['phone'] as String?,
        mapsUrl: json['maps_url'] as String?,
        reviews: (json['reviews'] as List)
            .map((e) => Review.fromJson(e as Map<String, dynamic>))
            .toList(),
      );

  Map<String, dynamic> toJson() => {
        'restaurant': restaurant.toJson(),
        'website': website,
        'phone': phone,
        'maps_url': mapsUrl,
        'reviews': reviews.map((r) => r.toJson()).toList(),
      };
}

/// RoomDto with params flattened, matching the server's serialization.
class Room {
  const Room({
    required this.id,
    required this.code,
    this.name,
    required this.locationLabel,
    required this.lat,
    required this.lng,
    required this.radiusM,
    required this.cuisines,
    required this.priceMin,
    required this.priceMax,
    required this.minRating,
    required this.createdAt,
  });

  final String id;
  final String code;
  final String? name;
  final String locationLabel;
  final double lat;
  final double lng;
  final int radiusM;
  final List<String> cuisines;
  final int priceMin;
  final int priceMax;
  final double minRating;
  final DateTime createdAt;

  factory Room.fromJson(Map<String, dynamic> json) => Room(
        id: json['id'] as String,
        code: json['code'] as String,
        name: json['name'] as String?,
        locationLabel: json['location_label'] as String,
        lat: _asDouble(json['lat']),
        lng: _asDouble(json['lng']),
        radiusM: json['radius_m'] as int,
        cuisines: (json['cuisines'] as List).cast<String>(),
        priceMin: json['price_min'] as int,
        priceMax: json['price_max'] as int,
        minRating: _asDouble(json['min_rating']),
        createdAt: DateTime.parse(json['created_at'] as String),
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'code': code,
        'name': name,
        'location_label': locationLabel,
        'lat': lat,
        'lng': lng,
        'radius_m': radiusM,
        'cuisines': cuisines,
        'price_min': priceMin,
        'price_max': priceMax,
        'min_rating': minRating,
        'created_at': createdAt.toUtc().toIso8601String(),
      };
}

class Participant {
  const Participant({
    required this.id,
    required this.roomId,
    required this.userId,
    required this.displayName,
  });

  final String id;
  final String roomId;
  final String userId;
  final String displayName;

  factory Participant.fromJson(Map<String, dynamic> json) => Participant(
        id: json['id'] as String,
        roomId: json['room_id'] as String,
        userId: json['user_id'] as String,
        displayName: json['display_name'] as String,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'room_id': roomId,
        'user_id': userId,
        'display_name': displayName,
      };
}

class MatchEntry {
  const MatchEntry({
    required this.restaurant,
    required this.likeCount,
    required this.lastLikedAt,
  });

  final Restaurant restaurant;
  final int likeCount;
  final DateTime lastLikedAt;

  factory MatchEntry.fromJson(Map<String, dynamic> json) => MatchEntry(
        restaurant:
            Restaurant.fromJson(json['restaurant'] as Map<String, dynamic>),
        likeCount: json['like_count'] as int,
        lastLikedAt: DateTime.parse(json['last_liked_at'] as String),
      );

  Map<String, dynamic> toJson() => {
        'restaurant': restaurant.toJson(),
        'like_count': likeCount,
        'last_liked_at': lastLikedAt.toUtc().toIso8601String(),
      };
}

/// "List" clashes with dart:core, hence DinnerList.
class DinnerList {
  const DinnerList({
    required this.id,
    required this.code,
    required this.name,
    required this.ownerUserId,
  });

  final String id;
  final String code;
  final String name;
  final String ownerUserId;

  factory DinnerList.fromJson(Map<String, dynamic> json) => DinnerList(
        id: json['id'] as String,
        code: json['code'] as String,
        name: json['name'] as String,
        ownerUserId: json['owner_user_id'] as String,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'code': code,
        'name': name,
        'owner_user_id': ownerUserId,
      };
}

/// One entry of `GET /lists`: the list's own fields flattened, plus is_owner.
class MyList {
  const MyList({required this.list, required this.isOwner});

  final DinnerList list;
  final bool isOwner;

  factory MyList.fromJson(Map<String, dynamic> json) => MyList(
        list: DinnerList.fromJson(json),
        isOwner: json['is_owner'] as bool,
      );

  Map<String, dynamic> toJson() => {
        ...list.toJson(),
        'is_owner': isOwner,
      };
}

class ListItem {
  const ListItem({
    required this.id,
    required this.listId,
    required this.name,
    this.cuisine,
    this.priceLevel,
    this.rating,
    this.address,
    this.photoUrl,
    required this.addedByUserId,
    this.sourceRestaurantId,
  });

  final String id;
  final String listId;
  final String name;
  final String? cuisine;
  final int? priceLevel;
  final double? rating;
  final String? address;
  final String? photoUrl;
  final String addedByUserId;
  final String? sourceRestaurantId;

  factory ListItem.fromJson(Map<String, dynamic> json) => ListItem(
        id: json['id'] as String,
        listId: json['list_id'] as String,
        name: json['name'] as String,
        cuisine: json['cuisine'] as String?,
        priceLevel: json['price_level'] as int?,
        rating: json['rating'] == null ? null : _asDouble(json['rating']),
        address: json['address'] as String?,
        photoUrl: json['photo_url'] as String?,
        addedByUserId: json['added_by_user_id'] as String,
        sourceRestaurantId: json['source_restaurant_id'] as String?,
      );

  Map<String, dynamic> toJson() => {
        'id': id,
        'list_id': listId,
        'name': name,
        'cuisine': cuisine,
        'price_level': priceLevel,
        'rating': rating,
        'address': address,
        'photo_url': photoUrl,
        'added_by_user_id': addedByUserId,
        'source_restaurant_id': sourceRestaurantId,
      };
}

class CreateRoomRequest {
  const CreateRoomRequest({
    this.name,
    required this.locationLabel,
    required this.lat,
    required this.lng,
    required this.radiusM,
    this.cuisines = const [],
    required this.priceMin,
    required this.priceMax,
    required this.minRating,
  });

  final String? name;
  final String locationLabel;
  final double lat;
  final double lng;
  final int radiusM;
  final List<String> cuisines;
  final int priceMin;
  final int priceMax;
  final double minRating;

  Map<String, dynamic> toJson() => {
        if (name != null) 'name': name,
        'location_label': locationLabel,
        'lat': lat,
        'lng': lng,
        'radius_m': radiusM,
        'cuisines': cuisines,
        'price_min': priceMin,
        'price_max': priceMax,
        'min_rating': minRating,
      };
}

class NewListItem {
  const NewListItem({
    required this.name,
    this.cuisine,
    this.priceLevel,
    this.rating,
    this.address,
    this.photoUrl,
    this.sourceRestaurantId,
  });

  final String name;
  final String? cuisine;
  final int? priceLevel;
  final double? rating;
  final String? address;
  final String? photoUrl;
  final String? sourceRestaurantId;

  Map<String, dynamic> toJson() => {
        'name': name,
        if (cuisine != null) 'cuisine': cuisine,
        if (priceLevel != null) 'price_level': priceLevel,
        if (rating != null) 'rating': rating,
        if (address != null) 'address': address,
        if (photoUrl != null) 'photo_url': photoUrl,
        if (sourceRestaurantId != null)
          'source_restaurant_id': sourceRestaurantId,
      };
}
