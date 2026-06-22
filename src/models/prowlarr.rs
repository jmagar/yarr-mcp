//! Indexer models — Prowlarr (`/api/v1`).
//!
//! Prowlarr exposes the standard *arr provider surface (indexers, applications,
//! download clients) plus search, indexer stats, tags, health, and system
//! status. Every resource is a plain camelCase JSON object, so each struct
//! derives `rename_all = "camelCase"`; the handful of exceptions use explicit
//! `#[serde(rename = …)]`.
//!
//! Source: <https://github.com/Prowlarr/Prowlarr/blob/develop/src/Prowlarr.Api.V1/openapi.json>
//!
//! Quirks worth flagging:
//! - The reserved word `type` becomes `kind` on [`Field`], [`ProviderMessage`],
//!   and [`HealthResource`] via `#[serde(rename = "type")]`.
//! - [`ReleaseResource`] carries `size` as a raw int64 byte count (not
//!   string-encoded) and `ageHours` / `ageMinutes` as floats — which is why no
//!   struct derives `Eq`.
//! - [`Field::hidden`] is a *string* (e.g. `"hidden"`), not a bool, and
//!   [`Field::value`] is untyped freeform JSON ([`serde_json::Value`]).
//! - List endpoints return bare JSON arrays (`Vec<T>`); the lone envelope is
//!   `GET /api/v1/indexerstats`, a single [`IndexerStatsResource`] wrapping three
//!   stats arrays.
//! - `presets` / `subCategories` are recursive self-references, heap-indirected
//!   through `Vec`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A configured indexer (`GET /api/v1/indexer`). `ProviderResource` subtype:
/// carries the provider config fields, search capabilities, app profile binding,
/// and failure status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerResource {
    pub id: Option<i32>,
    pub name: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
    pub implementation_name: Option<String>,
    /// Implementation class name (e.g. `Cardigann`, `Newznab`).
    pub implementation: Option<String>,
    pub config_contract: Option<String>,
    pub info_link: Option<String>,
    pub message: Option<ProviderMessage>,
    /// Tag ids (a `uniqueItems` set on the wire).
    #[serde(default)]
    pub tags: Vec<i32>,
    /// Preset variants — recursive self-reference, heap-indirected via `Vec`.
    #[serde(default)]
    pub presets: Vec<IndexerResource>,
    #[serde(default)]
    pub indexer_urls: Vec<String>,
    #[serde(default)]
    pub legacy_urls: Vec<String>,
    pub definition_name: Option<String>,
    pub description: Option<String>,
    pub language: Option<String>,
    pub encoding: Option<String>,
    pub enable: Option<bool>,
    pub redirect: Option<bool>,
    pub supports_rss: Option<bool>,
    pub supports_search: Option<bool>,
    pub supports_redirect: Option<bool>,
    pub supports_pagination: Option<bool>,
    pub app_profile_id: Option<i32>,
    pub protocol: Option<DownloadProtocol>,
    pub privacy: Option<IndexerPrivacy>,
    pub capabilities: Option<IndexerCapabilityResource>,
    pub priority: Option<i32>,
    /// Bound download client id (`0` = none).
    pub download_client_id: Option<i32>,
    /// ISO-8601 timestamp the indexer was added.
    pub added: Option<String>,
    pub status: Option<IndexerStatusResource>,
    pub sort_name: Option<String>,
}

/// An indexer's search capabilities and supported categories
/// ([`IndexerResource::capabilities`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerCapabilityResource {
    pub id: Option<i32>,
    pub limits_max: Option<i32>,
    pub limits_default: Option<i32>,
    #[serde(default)]
    pub categories: Vec<IndexerCategory>,
    pub supports_raw_search: Option<bool>,
    /// Generic search params (enum `{ q }`).
    #[serde(default)]
    pub search_params: Vec<SearchParam>,
    /// TV search params (`q`, `season`, `ep`, `imdbId`, `tvdbId`, …); kept as
    /// strings since the upstream enum is open-ended.
    #[serde(default)]
    pub tv_search_params: Vec<String>,
    #[serde(default)]
    pub movie_search_params: Vec<String>,
    #[serde(default)]
    pub music_search_params: Vec<String>,
    #[serde(default)]
    pub book_search_params: Vec<String>,
}

/// A Newznab category node (`capabilities.categories[]`,
/// [`ReleaseResource::categories`]). Recursive via [`Self::sub_categories`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerCategory {
    /// Category id (e.g. `5000` = TV).
    pub id: Option<i32>,
    pub name: Option<String>,
    /// Nested sub-categories — recursive, heap-indirected via `Vec`.
    #[serde(default)]
    pub sub_categories: Vec<IndexerCategory>,
    pub description: Option<String>,
}

/// A provider settings field (`fields[]` on indexers, applications, download
/// clients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub order: Option<i32>,
    pub name: Option<String>,
    pub label: Option<String>,
    pub unit: Option<String>,
    pub help_text: Option<String>,
    pub help_text_warning: Option<String>,
    pub help_link: Option<String>,
    /// Untyped field value (string/number/bool/array depending on field type).
    pub value: Option<serde_json::Value>,
    /// Reserved word: input-type discriminator (`textbox` / `select` /
    /// `checkbox` / …). Renamed from `type`.
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub advanced: Option<bool>,
    #[serde(default)]
    pub select_options: Vec<SelectOption>,
    pub select_options_provider_action: Option<String>,
    pub section: Option<String>,
    /// Quirk: a *string* (e.g. `"hidden"` / `"hiddenIfNotSet"`), not a bool.
    pub hidden: Option<String>,
    pub privacy: Option<PrivacyLevel>,
    pub placeholder: Option<String>,
    pub is_float: Option<bool>,
}

/// An option for a select-type [`Field`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SelectOption {
    pub value: Option<i32>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub hint: Option<String>,
    /// Parent option value for grouping.
    pub parent_value: Option<i32>,
}

/// A status message attached to a provider ([`IndexerResource::message`], etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProviderMessage {
    pub message: Option<String>,
    /// Reserved word: severity (`info` / `warning` / `error`). Renamed from
    /// `type`.
    #[serde(rename = "type")]
    pub kind: Option<ProviderMessageType>,
}

/// An indexer's failure / backoff status ([`IndexerResource::status`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerStatusResource {
    pub id: Option<i32>,
    pub indexer_id: Option<i32>,
    /// ISO-8601 timestamp the indexer is disabled until; absent when active.
    pub disabled_till: Option<String>,
    pub most_recent_failure: Option<String>,
    pub initial_failure: Option<String>,
}

/// Envelope: the single object returned by `GET /api/v1/indexerstats`, wrapping
/// three stats arrays. The per-indexer *row* is [`IndexerStatistics`], not this
/// type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerStatsResource {
    pub id: Option<i32>,
    #[serde(default)]
    pub indexers: Vec<IndexerStatistics>,
    #[serde(default)]
    pub user_agents: Vec<UserAgentStatistics>,
    #[serde(default)]
    pub hosts: Vec<HostStatistics>,
}

/// A per-indexer stats row ([`IndexerStatsResource::indexers`]). All counters are
/// raw int32 (not string-encoded).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IndexerStatistics {
    pub indexer_id: Option<i32>,
    pub indexer_name: Option<String>,
    /// Average query response time in ms.
    pub average_response_time: Option<i32>,
    /// Average grab response time in ms.
    pub average_grab_response_time: Option<i32>,
    pub number_of_queries: Option<i32>,
    pub number_of_grabs: Option<i32>,
    pub number_of_rss_queries: Option<i32>,
    pub number_of_auth_queries: Option<i32>,
    pub number_of_failed_queries: Option<i32>,
    pub number_of_failed_grabs: Option<i32>,
    pub number_of_failed_rss_queries: Option<i32>,
    pub number_of_failed_auth_queries: Option<i32>,
}

/// A per-user-agent stats row ([`IndexerStatsResource::user_agents`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserAgentStatistics {
    pub user_agent: Option<String>,
    pub number_of_queries: Option<i32>,
    pub number_of_grabs: Option<i32>,
}

/// A per-host stats row ([`IndexerStatsResource::hosts`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HostStatistics {
    pub host: Option<String>,
    pub number_of_queries: Option<i32>,
    pub number_of_grabs: Option<i32>,
}

/// A search-result release (`GET /api/v1/search` returns `[ReleaseResource]`).
/// This is the spec's `ReleaseResource` — the searchable result type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResource {
    pub id: Option<i32>,
    pub guid: Option<String>,
    /// Age in days.
    pub age: Option<i32>,
    /// Age in hours (float).
    pub age_hours: Option<f64>,
    /// Age in minutes (float).
    pub age_minutes: Option<f64>,
    /// Quirk: size in bytes as int64, emitted as a raw JSON number (not
    /// string-encoded).
    pub size: Option<i64>,
    pub files: Option<i32>,
    pub grabs: Option<i32>,
    pub indexer_id: Option<i32>,
    pub indexer: Option<String>,
    pub sub_group: Option<String>,
    pub release_hash: Option<String>,
    pub title: Option<String>,
    pub sort_title: Option<String>,
    /// IMDb id as int32 (`0` if absent).
    pub imdb_id: Option<i32>,
    pub tmdb_id: Option<i32>,
    pub tvdb_id: Option<i32>,
    pub tv_maze_id: Option<i32>,
    /// ISO-8601 publish timestamp.
    pub publish_date: Option<String>,
    pub comment_url: Option<String>,
    /// Direct download URL (`.nzb` / `.torrent`).
    pub download_url: Option<String>,
    pub info_url: Option<String>,
    pub poster_url: Option<String>,
    /// Indexer-specific flags (freeleech, etc.).
    #[serde(default)]
    pub indexer_flags: Vec<String>,
    #[serde(default)]
    pub categories: Vec<IndexerCategory>,
    /// Magnet URI (torrent).
    pub magnet_url: Option<String>,
    pub info_hash: Option<String>,
    /// Seeder count (torrent only).
    pub seeders: Option<i32>,
    /// Leecher count (torrent only).
    pub leechers: Option<i32>,
    pub protocol: Option<DownloadProtocol>,
    pub file_name: Option<String>,
    pub download_client_id: Option<i32>,
}

/// A configured sync application, e.g. Sonarr/Radarr (`GET /api/v1/applications`).
/// `ProviderResource` subtype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationResource {
    pub id: Option<i32>,
    pub name: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
    pub implementation_name: Option<String>,
    /// Implementation class (`Sonarr` / `Radarr` / `Lidarr` / …).
    pub implementation: Option<String>,
    pub config_contract: Option<String>,
    pub info_link: Option<String>,
    pub message: Option<ProviderMessage>,
    /// Tag ids (a `uniqueItems` set on the wire).
    #[serde(default)]
    pub tags: Vec<i32>,
    /// Preset variants — recursive self-reference, heap-indirected via `Vec`.
    #[serde(default)]
    pub presets: Vec<ApplicationResource>,
    pub sync_level: Option<ApplicationSyncLevel>,
    pub test_command: Option<String>,
}

/// A configured download client (`GET /api/v1/downloadclient`). `ProviderResource`
/// subtype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DownloadClientResource {
    pub id: Option<i32>,
    pub name: Option<String>,
    #[serde(default)]
    pub fields: Vec<Field>,
    pub implementation_name: Option<String>,
    /// Implementation class (`Sabnzbd` / `QBittorrent` / …).
    pub implementation: Option<String>,
    pub config_contract: Option<String>,
    pub info_link: Option<String>,
    pub message: Option<ProviderMessage>,
    /// Tag ids (a `uniqueItems` set on the wire).
    #[serde(default)]
    pub tags: Vec<i32>,
    /// Preset variants — recursive self-reference, heap-indirected via `Vec`.
    #[serde(default)]
    pub presets: Vec<DownloadClientResource>,
    pub enable: Option<bool>,
    pub protocol: Option<DownloadProtocol>,
    pub priority: Option<i32>,
    /// Category mappings (`DownloadClientCategory`); kept as freeform JSON since
    /// the nested shape is not modelled here.
    #[serde(default)]
    pub categories: Vec<serde_json::Value>,
    pub supports_categories: Option<bool>,
}

/// A tag (`GET /api/v1/tag` returns `[TagResource]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TagResource {
    pub id: Option<i32>,
    pub label: Option<String>,
}

/// A health check entry (`GET /api/v1/health` returns `[HealthResource]`). An
/// empty array means healthy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResource {
    pub id: Option<i32>,
    pub source: Option<String>,
    /// Reserved word: severity (`ok` / `notice` / `warning` / `error`). Renamed
    /// from `type`.
    #[serde(rename = "type")]
    pub kind: Option<HealthCheckResult>,
    pub message: Option<String>,
    pub wiki_url: Option<String>,
}

/// System status (`GET /api/v1/system/status`). A single object, not an array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SystemResource {
    /// Application name (`"Prowlarr"`).
    pub app_name: Option<String>,
    pub instance_name: Option<String>,
    pub version: Option<String>,
    /// ISO-8601 build timestamp.
    pub build_time: Option<String>,
    pub is_debug: Option<bool>,
    pub is_production: Option<bool>,
    pub is_admin: Option<bool>,
    pub is_user_interactive: Option<bool>,
    pub startup_path: Option<String>,
    pub app_data: Option<String>,
    pub os_name: Option<String>,
    pub os_version: Option<String>,
    pub is_net_core: Option<bool>,
    pub is_linux: Option<bool>,
    pub is_osx: Option<bool>,
    pub is_windows: Option<bool>,
    pub is_docker: Option<bool>,
    pub mode: Option<RuntimeMode>,
    /// Update branch (e.g. `"master"`).
    pub branch: Option<String>,
    pub database_type: Option<DatabaseType>,
    pub database_version: Option<String>,
    pub authentication: Option<AuthenticationType>,
    pub migration_version: Option<i32>,
    pub url_base: Option<String>,
    pub runtime_version: Option<String>,
    /// Runtime name (`"netCore"`).
    pub runtime_name: Option<String>,
    /// ISO-8601 process start timestamp.
    pub start_time: Option<String>,
    pub package_version: Option<String>,
    pub package_author: Option<String>,
    pub package_update_mechanism: Option<UpdateMechanism>,
    pub package_update_mechanism_message: Option<String>,
}

/// Download protocol (`unknown` / `usenet` / `torrent`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DownloadProtocol {
    Unknown,
    Usenet,
    Torrent,
}

/// Indexer privacy (`public` / `semiPrivate` / `private`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum IndexerPrivacy {
    Public,
    SemiPrivate,
    Private,
}

/// [`ProviderMessage`] severity (`info` / `warning` / `error`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ProviderMessageType {
    Info,
    Warning,
    Error,
}

/// [`HealthResource`] severity (`ok` / `notice` / `warning` / `error`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HealthCheckResult {
    Ok,
    Notice,
    Warning,
    Error,
}

/// Application sync level (`disabled` / `addOnly` / `fullSync`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ApplicationSyncLevel {
    Disabled,
    AddOnly,
    FullSync,
}

/// [`Field`] privacy level (`normal` / `password` / `apiKey` / `userName`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PrivacyLevel {
    Normal,
    Password,
    ApiKey,
    UserName,
}

/// System runtime mode (`console` / `service` / `tray`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeMode {
    Console,
    Service,
    Tray,
}

/// Generic search param (only `q`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SearchParam {
    Q,
}

/// Database engine on [`SystemResource`] (`sqLite` / `postgreSQL`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DatabaseType {
    SqLite,
    PostgreSQL,
}

/// Authentication method on [`SystemResource`] (`none` / `basic` / `forms` /
/// `external`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AuthenticationType {
    None,
    Basic,
    Forms,
    External,
}

/// Update mechanism on [`SystemResource`] (`builtIn` / `script` / `external` /
/// `apt` / `docker`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum UpdateMechanism {
    BuiltIn,
    Script,
    External,
    Apt,
    Docker,
}

#[cfg(test)]
#[path = "prowlarr_tests.rs"]
mod tests;
