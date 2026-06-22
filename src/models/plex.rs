//! MediaServer models — Plex (`/status/sessions`, `/identity`, `/library/sections/all`).
//!
//! Every Plex response is wrapped in a single `{ "MediaContainer": { … } }`
//! envelope ([`MediaContainer`]); the container hoists paging attributes and
//! carries one PascalCase child array per endpoint (`Metadata[]`, `Directory[]`,
//! `Hub[]`). Fields mirror the negotiated-JSON shapes consumed by
//! `crate::app::media_server::plex`.
//!
//! Casing is *mixed* on the wire: container/child element keys are PascalCase
//! (`Metadata`, `Directory`, `Media`, `Part`, `Player`, `Session`, `User`,
//! `Tag`, `Hub`, `Image`), while scalar attributes are camelCase/lowercase
//! (`title`, `ratingKey`, `viewOffset`, `machineIdentifier`). No single
//! `rename_all` covers both, so every field carries an explicit
//! `#[serde(rename = …)]`.
//!
//! Plex quirks modelled here: `type` is a reserved word and is renamed `kind`;
//! `ratingKey`/`parentRatingKey`/`grandparentRatingKey` and `User.id` are
//! string-encoded even though they look numeric (`Player.userID` and `Tag.id`
//! are integers — id typing is inconsistent across types); `Media.id`/`Part.id`
//! and the epoch timestamps (`addedAt`, `updatedAt`, `lastViewedAt`,
//! `createdAt`, …) are int64 Unix-epoch seconds; and several flags
//! (`skipChildren`, `hasVoiceActivity`, `allowSync`) arrive as either a bool or
//! a `"0"`/`"1"` string, modelled as [`BoolOrIntStr`].

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A flag Plex Media Server sometimes returns as a JSON bool and sometimes as a
/// `"0"`/`"1"` string (e.g. `skipChildren`, `hasVoiceActivity`, `allowSync`).
/// Untagged so either wire form deserialises.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum BoolOrIntStr {
    Bool(bool),
    Str(String),
}

/// Client network location for a playback session (`Session.location`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SessionLocation {
    Lan,
    Wan,
}

/// Purpose/presentation of an [`Image`] element (`Image.type`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ImageType {
    #[serde(rename = "background")]
    Background,
    #[serde(rename = "backgroundSquare")]
    BackgroundSquare,
    #[serde(rename = "banner")]
    Banner,
    #[serde(rename = "clearLogo")]
    ClearLogo,
    #[serde(rename = "coverPoster")]
    CoverPoster,
    #[serde(rename = "snapshot")]
    Snapshot,
}

/// Root element of nearly every Plex response. Generic container that hoists
/// common/paging attributes from its children; one PascalCase child array is
/// populated per endpoint (`Metadata[]` for `/status/sessions`, `Directory[]`
/// for `/library/sections/all`, `Hub[]` for search). Also carries the
/// `/identity` scalars (`machineIdentifier`, `version`, `claimed`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MediaContainer {
    /// Unique identifier for this container.
    pub identifier: Option<String>,
    /// Number of items in this response page.
    pub size: Option<i64>,
    /// Total objects available (paging); also in `X-Plex-Container-Total-Size`.
    #[serde(rename = "totalSize")]
    pub total_size: Option<i64>,
    /// Offset where this page starts; also in `X-Plex-Container-Start`.
    pub offset: Option<i64>,
    /// Child metadata items (`/status/sessions` items also carry
    /// `Player`/`Session`/`User`).
    #[serde(rename = "Metadata", default)]
    pub metadata: Vec<Metadata>,
    /// Child directories. For `/library/sections/all` the items are
    /// [`LibrarySection`]; other endpoints use the generic `Directory` schema.
    #[serde(rename = "Directory", default)]
    pub directory: Vec<LibrarySection>,
    /// Child hubs (search / hub endpoints).
    #[serde(rename = "Hub", default)]
    pub hub: Vec<SearchHubResult>,
    /// Library-sections envelope extra; typically `"Plex Library"`.
    pub title1: Option<String>,
    /// `allowSync` (bool-or-`"0"`/`"1"`) on the library-sections envelope.
    #[serde(rename = "allowSync")]
    pub allow_sync: Option<BoolOrIntStr>,
    /// On `MediaContainerWithStatus`: if present and non-zero indicates an error.
    pub status: Option<i64>,
    // /identity scalars (inline schema, no named component):
    /// A unique identifier of the computer/server (`/identity`).
    #[serde(rename = "machineIdentifier")]
    pub machine_identifier: Option<String>,
    /// The full version string of the PMS (`/identity`).
    pub version: Option<String>,
    /// Whether this server has been claimed by a user (`/identity`).
    pub claimed: Option<bool>,
}

/// A library metadata item (movie/show/season/episode/artist/album/track/photo/
/// clip). PMS returns many undocumented extras via `additionalProperties` —
/// unknown fields deserialise-and-ignore. Required on the wire: `addedAt`,
/// `key`, `title`, `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Metadata {
    /// The title of the item (e.g. `"300"`).
    pub title: String,
    /// Item type such as movie/episode/clip. Reserved word — renamed `kind`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Key at which the item's details can be fetched.
    pub key: String,
    /// Epoch seconds the item was added to the library.
    #[serde(rename = "addedAt")]
    pub added_at: i64,
    /// Opaque id for timeline/scrobble/rating endpoints. String-encoded though
    /// often numeric.
    #[serde(rename = "ratingKey")]
    pub rating_key: Option<String>,
    /// Globally unique id, e.g. `plex://movie/5d77…`.
    pub guid: Option<String>,
    /// External provider guids (`imdb://`, `tmdb://`, `tvdb://`).
    #[serde(rename = "Guid", default)]
    pub guids: Vec<MetadataGuid>,
    /// Release year.
    pub year: Option<i32>,
    /// Current playback offset in milliseconds when in progress.
    #[serde(rename = "viewOffset")]
    pub view_offset: Option<i32>,
    /// Duration of the item in milliseconds.
    pub duration: Option<i32>,
    /// URL for poster/thumbnail.
    pub thumb: Option<String>,
    /// URL for background artwork.
    pub art: Option<String>,
    /// URL for a banner graphic.
    pub banner: Option<String>,
    /// URL for a hero image.
    pub hero: Option<String>,
    /// URL for theme music (usually TV shows).
    pub theme: Option<String>,
    /// Composite image URL for descendant items (photo albums/playlists).
    pub composite: Option<String>,
    /// Subtype such as `"photo"` for a video item in a photo library.
    pub subtype: Option<String>,
    /// Episode/season/track number.
    pub index: Option<i32>,
    /// Disc number for a track on multi-disc albums.
    #[serde(rename = "absoluteIndex")]
    pub absolute_index: Option<i32>,
    /// Extended textual description (plot/biography/review).
    pub summary: Option<String>,
    /// One-liner about the item (movies).
    pub tagline: Option<String>,
    /// Studio or label which produced the item.
    pub studio: Option<String>,
    /// Content rating (e.g. MPAA).
    #[serde(rename = "contentRating")]
    pub content_rating: Option<String>,
    /// Source of chapters: media/agent/mixed.
    #[serde(rename = "chapterSource")]
    pub chapter_source: Option<String>,
    /// Rating 0-10; meaning depends on source.
    pub rating: Option<f32>,
    /// URI for image shown with the rating.
    #[serde(rename = "ratingImage")]
    pub rating_image: Option<String>,
    /// Number of ratings.
    #[serde(rename = "ratingCount")]
    pub rating_count: Option<i32>,
    /// Audience rating 0-10.
    #[serde(rename = "audienceRating")]
    pub audience_rating: Option<f32>,
    /// URI for the audience-rating image.
    #[serde(rename = "audienceRatingImage")]
    pub audience_rating_image: Option<String>,
    /// The user's own rating 0-10.
    #[serde(rename = "userRating")]
    pub user_rating: Option<f32>,
    /// Array of rating tags.
    #[serde(rename = "Rating", default)]
    pub rating_array: Vec<Tag>,
    /// Air/release date `YYYY-MM-DD [HH:MM:SS]` (date string, not epoch).
    #[serde(rename = "originallyAvailableAt")]
    pub originally_available_at: Option<String>,
    /// Original/foreign title.
    #[serde(rename = "originalTitle")]
    pub original_title: Option<String>,
    /// Sort string (articles removed).
    #[serde(rename = "titleSort")]
    pub title_sort: Option<String>,
    /// Epoch seconds of last consumption (PlexDateTime, epoch i64).
    #[serde(rename = "lastViewedAt")]
    pub last_viewed_at: Option<i64>,
    /// Epoch seconds the item was last changed.
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<i64>,
    /// Number of completed consumptions.
    #[serde(rename = "viewCount")]
    pub view_count: Option<i32>,
    /// Number of viewed episodes (shows/seasons).
    #[serde(rename = "viewedLeafCount")]
    pub viewed_leaf_count: Option<i32>,
    /// Total episode count (shows/seasons).
    #[serde(rename = "leafCount")]
    pub leaf_count: Option<i32>,
    /// Number of child items.
    #[serde(rename = "childCount")]
    pub child_count: Option<i32>,
    /// Key for primary extra (trailer / music video).
    #[serde(rename = "primaryExtraKey")]
    pub primary_extra_key: Option<String>,
    /// Title of the parent.
    #[serde(rename = "parentTitle")]
    pub parent_title: Option<String>,
    /// Key of the parent.
    #[serde(rename = "parentKey")]
    pub parent_key: Option<String>,
    /// RatingKey of the parent (string-encoded).
    #[serde(rename = "parentRatingKey")]
    pub parent_rating_key: Option<String>,
    /// GUID of the parent.
    #[serde(rename = "parentGuid")]
    pub parent_guid: Option<String>,
    /// Index of the parent.
    #[serde(rename = "parentIndex")]
    pub parent_index: Option<i32>,
    /// Thumb of the parent.
    #[serde(rename = "parentThumb")]
    pub parent_thumb: Option<String>,
    /// Hero of the parent.
    #[serde(rename = "parentHero")]
    pub parent_hero: Option<String>,
    /// Title of the grandparent (e.g. show for an episode).
    #[serde(rename = "grandparentTitle")]
    pub grandparent_title: Option<String>,
    /// Key of the grandparent.
    #[serde(rename = "grandparentKey")]
    pub grandparent_key: Option<String>,
    /// RatingKey of the grandparent (string-encoded).
    #[serde(rename = "grandparentRatingKey")]
    pub grandparent_rating_key: Option<String>,
    /// GUID of the grandparent.
    #[serde(rename = "grandparentGuid")]
    pub grandparent_guid: Option<String>,
    /// Thumb of the grandparent.
    #[serde(rename = "grandparentThumb")]
    pub grandparent_thumb: Option<String>,
    /// Art of the grandparent.
    #[serde(rename = "grandparentArt")]
    pub grandparent_art: Option<String>,
    /// Hero of the grandparent.
    #[serde(rename = "grandparentHero")]
    pub grandparent_hero: Option<String>,
    /// Theme of the grandparent.
    #[serde(rename = "grandparentTheme")]
    pub grandparent_theme: Option<String>,
    /// Shows: skip seasons in favor of episodes. Bool or `"0"`/`"1"`.
    #[serde(rename = "skipChildren")]
    pub skip_children: Option<BoolOrIntStr>,
    /// Episode/track: skip parent in favor of grandparent. Bool or `"0"`/`"1"`.
    #[serde(rename = "skipParent")]
    pub skip_parent: Option<BoolOrIntStr>,
    /// Indicates this is a search directory.
    pub search: Option<bool>,
    /// Old-client nested-menu flag.
    pub secondary: Option<bool>,
    /// Prompt for this directory (e.g. `"Search Movies"`).
    pub prompt: Option<String>,
    /// Media files/instances for this item.
    #[serde(rename = "Media", default)]
    pub media: Vec<Media>,
    /// Genre tags.
    #[serde(rename = "Genre", default)]
    pub genre: Vec<Tag>,
    /// Director tags.
    #[serde(rename = "Director", default)]
    pub director: Vec<Tag>,
    /// Writer tags.
    #[serde(rename = "Writer", default)]
    pub writer: Vec<Tag>,
    /// Cast/role tags.
    #[serde(rename = "Role", default)]
    pub role: Vec<Tag>,
    /// Country tags.
    #[serde(rename = "Country", default)]
    pub country: Vec<Tag>,
    /// Auto-generated tags.
    #[serde(rename = "Autotag", default)]
    pub autotag: Vec<Tag>,
    /// Image elements (posters/artwork).
    #[serde(rename = "Image", default)]
    pub image: Vec<Image>,
    /// Filters (library top-level only).
    #[serde(rename = "Filter", default)]
    pub filter: Vec<Directory>,
    /// Sort fields (library top-level only).
    #[serde(rename = "Sort", default)]
    pub sort: Vec<Directory>,
    /// Undocumented on base Metadata; arrives via `additionalProperties`. Title
    /// of the owning library section.
    #[serde(rename = "librarySectionTitle")]
    pub library_section_title: Option<String>,
    /// Only on `/status/sessions` items: the player handling playback.
    #[serde(rename = "Player")]
    pub player: Option<Player>,
    /// Only on `/status/sessions` items: the playback session.
    #[serde(rename = "Session")]
    pub session: Option<Session>,
    /// Only on `/status/sessions` items: the user playing the content.
    #[serde(rename = "User")]
    pub user: Option<User>,
    /// Search-only: reason for the result if not a direct term match (a section
    /// key, or a source-hub identifier). Arrives via `additionalProperties` on
    /// `/hubs/search` items.
    pub reason: Option<String>,
    /// Search-only: string associated with the reason code (section name or
    /// match string).
    #[serde(rename = "reasonTitle")]
    pub reason_title: Option<String>,
    /// Search-only: id of the item associated with the reason.
    #[serde(rename = "reasonID")]
    pub reason_id: Option<String>,
    /// Search-only: relevance score (returned by PMS via `additionalProperties`).
    pub score: Option<f32>,
}

/// External-provider guid entry inside `Metadata.Guid[]`. Required: `id`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MetadataGuid {
    /// Provider guid, pattern `^(imdb|tmdb|tvdb)://.+$` (e.g. `imdb://tt13015952`).
    pub id: String,
}

/// A library section (a.k.a. library) — a typed collection of media. This is
/// the `Directory` element returned by `/library/sections/all` (items are
/// `LibrarySection`, not the generic `Directory`). Required on the wire:
/// `uuid`, `language`, `type`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LibrarySection {
    /// Universally unique id for the library (e.g. `e69655a2-…`).
    pub uuid: String,
    /// Library language.
    pub language: String,
    /// `MediaTypeString` enum: movie/show/season/episode/artist/album/track/
    /// photoalbum/photo/collection. Reserved word — renamed `kind`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Title of the library (e.g. `"Movies"`).
    pub title: Option<String>,
    /// Metadata agent for the section.
    pub agent: Option<String>,
    /// Scanner used for the section.
    pub scanner: Option<String>,
    /// Section key.
    pub key: Option<String>,
    /// Background artwork URL.
    pub art: Option<String>,
    /// Composite image URL.
    pub composite: Option<String>,
    /// Thumbnail URL.
    pub thumb: Option<String>,
    /// Whether sync is allowed. Bool or `"0"`/`"1"`.
    #[serde(rename = "allowSync")]
    pub allow_sync: Option<BoolOrIntStr>,
    /// Whether the section has content.
    pub content: Option<bool>,
    /// Whether this is a directory.
    pub directory: Option<bool>,
    /// Whether the section has filtering capabilities.
    pub filters: Option<bool>,
    /// Whether the section is hidden.
    pub hidden: Option<bool>,
    /// Whether the section is currently scanning.
    pub refreshing: Option<bool>,
    /// PlexDateTime epoch seconds.
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    /// PlexDateTime epoch seconds.
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<i64>,
    /// PlexDateTime epoch seconds.
    #[serde(rename = "scannedAt")]
    pub scanned_at: Option<i64>,
    /// PlexDateTime epoch seconds.
    #[serde(rename = "contentChangedAt")]
    pub content_changed_at: Option<i64>,
    /// On-disk locations storing this section's media.
    #[serde(rename = "Location", default)]
    pub location: Vec<LibrarySectionLocation>,
}

/// A top-level on-disk location for a library section (`LibrarySection.Location[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LibrarySectionLocation {
    /// Location id.
    pub id: Option<i64>,
    /// Directory path on disk.
    pub path: Option<String>,
}

/// Generic directory element (distinct from [`LibrarySection`]); base type for
/// the `Filter` and `Sort` elements on a library. PMS returns extras via
/// `additionalProperties` — unknown fields deserialise-and-ignore.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Directory {
    /// Directory title.
    pub title: Option<String>,
    /// Directory type. Reserved word — renamed `kind`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Directory key.
    pub key: Option<String>,
    /// Artwork URL.
    pub art: Option<String>,
    /// Thumbnail URL.
    pub thumb: Option<String>,
    /// Title-bar string.
    #[serde(rename = "titleBar")]
    pub title_bar: Option<String>,
    /// Whether the directory has content.
    pub content: Option<bool>,
    /// Filter parameter for querying matching content.
    pub filter: Option<String>,
    /// Whether the directory exposes preferences.
    #[serde(rename = "hasPrefs")]
    pub has_prefs: Option<bool>,
    /// Whether the directory exposes store services.
    #[serde(rename = "hasStoreServices")]
    pub has_store_services: Option<bool>,
    /// Associated hub key.
    #[serde(rename = "hubKey")]
    pub hub_key: Option<String>,
    /// Directory identifier.
    pub identifier: Option<String>,
    /// Last-accessed epoch.
    #[serde(rename = "lastAccessedAt")]
    pub last_accessed_at: Option<i64>,
    /// Share flag.
    pub share: Option<i64>,
    /// Pivot navigation entries.
    #[serde(rename = "Pivot", default)]
    pub pivot: Vec<DirectoryPivot>,
}

/// Pivot navigation entry inside `Directory.Pivot[]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DirectoryPivot {
    /// Pivot title.
    pub title: Option<String>,
    /// Pivot type. Reserved word — renamed `kind`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Pivot context.
    pub context: Option<String>,
    /// Pivot id (string).
    pub id: Option<String>,
    /// Pivot key.
    pub key: Option<String>,
    /// Pivot display symbol.
    pub symbol: Option<String>,
}

/// Plex has no dedicated `SearchResult` schema. Hub/voice search returns
/// `MediaContainer.Hub[]`, each `Hub` grouping `Metadata[]` (the actual
/// results). Search-specific `reason`/`reasonTitle`/`reasonID`/`score`
/// attributes arrive on the metadata items via `additionalProperties` (see the
/// search-only fields on [`Metadata`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SearchHubResult {
    /// Title for this grouping of content.
    pub title: Option<String>,
    /// Type of items in this hub, or `"mixed"`. Reserved word — renamed `kind`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    /// Unique identifier for the hub (e.g. `home.onDeck`).
    #[serde(rename = "hubIdentifier")]
    pub hub_identifier: Option<String>,
    /// Key to re-fetch exact current content (important for random hubs).
    #[serde(rename = "hubKey")]
    pub hub_key: Option<String>,
    /// Key to retrieve all content for this hub.
    pub key: Option<String>,
    /// Hub context (e.g. `hub.home.onDeck`).
    pub context: Option<String>,
    /// Display hint: hero/list/spotlight/upsell.
    pub style: Option<String>,
    /// Subtype of contained items, or `"mixed"`.
    pub subtype: Option<String>,
    /// Whether the hub has more than is included.
    pub more: Option<bool>,
    /// Whether the hub should be promoted to the home screen.
    pub promoted: Option<bool>,
    /// Whether contents may change each request.
    pub random: Option<bool>,
    /// Number of items in this response.
    pub size: Option<i64>,
    /// Total items available.
    #[serde(rename = "totalSize")]
    pub total_size: Option<i64>,
    /// The matched metadata items for this hub.
    #[serde(rename = "Metadata", default)]
    pub metadata: Vec<Metadata>,
}

/// One or more media files (parts), a child of a metadata item
/// (`Metadata.Media[]`). PMS returns extras via `additionalProperties`. Required
/// on the wire: `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Media {
    /// Media id (int64).
    pub id: i64,
    /// Duration in ms.
    pub duration: Option<i32>,
    /// Bitrate.
    pub bitrate: Option<i32>,
    /// Pixel width.
    pub width: Option<i32>,
    /// Pixel height.
    pub height: Option<i32>,
    /// Aspect ratio (e.g. 2.35).
    #[serde(rename = "aspectRatio")]
    pub aspect_ratio: Option<f32>,
    /// Audio channel count.
    #[serde(rename = "audioChannels")]
    pub audio_channels: Option<i32>,
    /// Audio codec (e.g. aac).
    #[serde(rename = "audioCodec")]
    pub audio_codec: Option<String>,
    /// Audio profile (e.g. lc).
    #[serde(rename = "audioProfile")]
    pub audio_profile: Option<String>,
    /// Video codec (e.g. h264).
    #[serde(rename = "videoCodec")]
    pub video_codec: Option<String>,
    /// Video profile (e.g. main).
    #[serde(rename = "videoProfile")]
    pub video_profile: Option<String>,
    /// Frame rate (e.g. 24p).
    #[serde(rename = "videoFrameRate")]
    pub video_frame_rate: Option<String>,
    /// Resolution string (e.g. `"720"`).
    #[serde(rename = "videoResolution")]
    pub video_resolution: Option<String>,
    /// Container (e.g. mov).
    pub container: Option<String>,
    /// Whether the file uses 64-bit offsets.
    #[serde(rename = "has64bitOffsets")]
    pub has_64bit_offsets: Option<bool>,
    /// Voice-activity flag. Bool or `"0"`/`"1"`.
    #[serde(rename = "hasVoiceActivity")]
    pub has_voice_activity: Option<BoolOrIntStr>,
    /// `BoolInt` enum `0`|`1`.
    #[serde(rename = "optimizedForStreaming")]
    pub optimized_for_streaming: Option<i32>,
    /// Playable file parts.
    #[serde(rename = "Part", default)]
    pub part: Vec<Part>,
}

/// A particular file/`part` of a media item — the playable unit (`Media.Part[]`).
/// PMS returns extras via `additionalProperties`. Required on the wire: `id`,
/// `key`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Part {
    /// Part id (int64).
    pub id: i64,
    /// Key from which the media can be streamed.
    pub key: String,
    /// Duration in ms.
    pub duration: Option<i32>,
    /// Local file path on the server.
    pub file: Option<String>,
    /// Size in bytes (int64).
    pub size: Option<i64>,
    /// File container (e.g. mp4/mkv).
    pub container: Option<String>,
    /// Audio profile.
    #[serde(rename = "audioProfile")]
    pub audio_profile: Option<String>,
    /// Video profile.
    #[serde(rename = "videoProfile")]
    pub video_profile: Option<String>,
    /// Index availability (e.g. `"sd"`).
    pub indexes: Option<String>,
    /// Whether the file uses 64-bit offsets.
    #[serde(rename = "has64bitOffsets")]
    pub has_64bit_offsets: Option<bool>,
    /// Whether optimized for streaming (bool here, unlike `Media` which is int).
    #[serde(rename = "optimizedForStreaming")]
    pub optimized_for_streaming: Option<bool>,
    /// Whether the part is accessible.
    pub accessible: Option<bool>,
    /// Whether the part exists.
    pub exists: Option<bool>,
    /// Streams (video/audio/subtitle) within this part.
    #[serde(rename = "Stream", default)]
    pub stream: Vec<Stream>,
}

/// A stream (video/audio/subtitle) within a [`Part`] (`Part.Stream[]`). Slimmed
/// to the stable identity fields; PMS returns many extras via
/// `additionalProperties`. Required on the wire: `id`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Stream {
    /// Stream id (int64).
    pub id: i64,
    /// Stream type code (1 = video, 2 = audio, 3 = subtitle). Reserved word —
    /// renamed `kind`.
    #[serde(rename = "streamType")]
    pub kind: Option<i32>,
    /// Codec (e.g. h264, aac, srt).
    pub codec: Option<String>,
    /// Language name.
    pub language: Option<String>,
    /// Display title for the stream.
    #[serde(rename = "displayTitle")]
    pub display_title: Option<String>,
    /// Whether the stream is currently selected.
    pub selected: Option<bool>,
}

/// Information about the player/client handling playback (`Metadata.Player`).
/// All fields optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Player {
    /// Title of the client.
    pub title: Option<String>,
    /// Client's last reported state (e.g. playing/paused/buffering).
    pub state: Option<String>,
    /// Product name of the client.
    pub product: Option<String>,
    /// Remote address of the client.
    pub address: Option<String>,
    /// Identifier of the client.
    #[serde(rename = "machineIdentifier")]
    pub machine_identifier: Option<String>,
    /// Model of the client.
    pub model: Option<String>,
    /// Platform of the client.
    pub platform: Option<String>,
    /// Platform version of the client.
    #[serde(rename = "platformVersion")]
    pub platform_version: Option<String>,
    /// Vendor of the client.
    pub vendor: Option<String>,
    /// Version of the client.
    pub version: Option<String>,
    /// Whether playing from local LAN.
    pub local: Option<bool>,
    /// Whether playing over a relay connection.
    pub relayed: Option<bool>,
    /// Whether playing over HTTPS.
    pub secure: Option<bool>,
    /// The client's public address.
    #[serde(rename = "remotePublicAddress")]
    pub remote_public_address: Option<String>,
    /// Id of the user (integer here, unlike [`User::id`] which is a String).
    #[serde(rename = "userID")]
    pub user_id: Option<i64>,
}

/// Information about the playback session (`Metadata.Session`). All fields
/// optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Session {
    /// Id of the playback session (string).
    pub id: Option<String>,
    /// Bandwidth used by this client's playback in kbps.
    pub bandwidth: Option<i64>,
    /// Location of the client; `lan`|`wan`.
    pub location: Option<SessionLocation>,
}

/// The in-session user playing the content (`Metadata.User`). Distinct from the
/// larger `UserPlexAccount` (my.plex.tv account, not modelled here). All fields
/// optional.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct User {
    /// Id of the user. String-encoded (note: `Player.userID` for the same user
    /// is an integer).
    pub id: Option<String>,
    /// The username.
    pub title: Option<String>,
    /// Thumb image URL to display for the user.
    pub thumb: Option<String>,
}

/// Extra information about a metadata item, surfaced under element names like
/// `Genre`/`Director`/`Role`/`Rating`. Required on the wire: `tag`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Tag {
    /// The tag value/name (e.g. an actor or genre name).
    pub tag: String,
    /// Tag id — integer, unlike [`Tag::rating_key`] which is a String.
    pub id: Option<i32>,
    /// Plex tag identifier for fetching more info from plex.tv.
    #[serde(rename = "tagKey")]
    pub tag_key: Option<String>,
    /// Tag type.
    #[serde(rename = "tagType")]
    pub tag_type: Option<i32>,
    /// Filter parameter to query matching content (e.g. `actor=49`).
    pub filter: Option<String>,
    /// The role this actor played.
    pub role: Option<String>,
    /// Thumbnail URL for the tag (e.g. actor headshot).
    pub thumb: Option<String>,
    /// Confidence of an automatic tag.
    pub confidence: Option<f64>,
    /// Tag context.
    pub context: Option<String>,
    /// Rating key (Media ID) of this item; always integer but represented as a
    /// String.
    #[serde(rename = "ratingKey")]
    pub rating_key: Option<String>,
}

/// Image elements such as posters and background artwork (`Metadata.Image[]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Image {
    /// Purpose/presentation of the image. Reserved word — renamed `kind`.
    #[serde(rename = "type")]
    pub kind: Option<ImageType>,
    /// Accessibility title.
    pub alt: Option<String>,
    /// Relative path or absolute URL for the image.
    pub url: Option<String>,
}

#[cfg(test)]
#[path = "plex_tests.rs"]
mod tests;
