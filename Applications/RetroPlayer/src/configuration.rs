use crate::PROJECT_DIRECTORY;
use crate::theme::color_from_string;
use crate::{logger, logger::*};
use anyhow::Result;
use retro_engine::Color;
use serde::{Deserialize, Serialize};
use std::{
    fs::{create_dir_all, read, write},
    path::PathBuf,
    sync::{LazyLock, RwLock},
};

const DEFAULT_CONFIGURATION_CONTENT: &str = "# Inside the \"\" (quotes) type the path to your song directory.
songs_directory = \"\"
# Optional additional playlist directories. The first item is treated as the
# primary list when no saved selection exists.
song_directories = []
# Automatically restore the last selected playlist directory on startup.
restore_last_playlist = true
# Automatically shuffle the playlist on startup and on playlist switch.
preshuffle = false

# Change 'false' to 'true' to enable Discord rich presence feature.
discord_presence = false

# Colors used by the application. The following colors are supported for each of the fields below the theme section.
# default, white, grey, red, green, yellow, blue, magenta, cyan, black, dark-grey, dark-red, dark-green, dark-yellow, dark-blue, dark-magenta, dark-cyan.
[theme]
primary = \"default\"
secondary = \"dark-grey\"
accent = \"blue\"

# Online recommendations + download feature (MusicBrainz/ListenBrainz/Last.fm -> yt-dlp).
[streaming]
enabled = true
# Enable streaming mode: continuously downloads & inserts recommended tracks into playback queue.
streaming_mode = false
# How many recommended tracks to fetch per refresh.
recommendations_per_fetch = 8
# ListenBrainz LB Radio mode: \"easy\", \"medium\", or \"hard\" (controls how loosely
# \"similar\" is interpreted).
listenbrainz_mode = \"easy\"
# Path to the yt-dlp executable, or just \"yt-dlp\" if it's on your PATH.
ytdlp_path = \"yt-dlp\"
# Where downloaded tracks are saved. Leave blank to use songs_directory.
download_directory = \"\"
# Maximum number of downloaded recommendation songs to keep per playlist (0 to disable pruning).
recommendations_cache_limit = 30

# Lyrics fetching & embedding configuration.
[lyrics]
enabled = true
prefer_synced = true
save_lrc_file = true
embed_in_audio = true
";

pub fn get_configuration_path() -> Result<PathBuf> {
    let configuration_directory = PROJECT_DIRECTORY.config_dir();
    create_dir_all(configuration_directory)?;
    return Ok(configuration_directory.join("Configuration.toml"));
}

pub fn get_runtime_state_path() -> Result<PathBuf> {
    let data_directory = PROJECT_DIRECTORY.data_dir();
    create_dir_all(data_directory)?;
    Ok(data_directory.join("runtime_state.json"))
}

pub static CONFIGURATION: LazyLock<RwLock<Configuration>> = LazyLock::new(|| {
    fn get_configuration() -> Result<Configuration> {
        logger!().add_entry(
            EntryId::Configuration,
            logger::Entry::new(String::from("Getting configuration")),
        );
        let configuration_path = get_configuration_path()?;

        let configuration_file_result = read(&configuration_path);
        if configuration_file_result.is_err() {
            write(&configuration_path, DEFAULT_CONFIGURATION_CONTENT)?;
        }

        let configuration_file = read(&configuration_path)?;
        let configuration: ConfigurationTemplate = toml::from_slice(&configuration_file)?;
        logger!()
            .get_entry(EntryId::Configuration)
            .unwrap()
            .complete("Loaded configuration");
        return Ok(configuration.into());
    }
    RwLock::new(get_configuration().unwrap_or(Configuration::default()))
});

#[macro_export]
macro_rules! configuration {
    () => {
        $crate::CONFIGURATION.read().unwrap()
    };
}

#[derive(Serialize, Deserialize)]
pub struct ThemeTemplate {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
}

#[derive(Serialize, Deserialize)]
pub struct StreamingTemplate {
    #[serde(default = "default_streaming_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub streaming_mode: bool,
    #[serde(default = "default_recommendations_per_fetch")]
    pub recommendations_per_fetch: usize,
    #[serde(default = "default_listenbrainz_mode")]
    pub listenbrainz_mode: String,
    #[serde(default = "default_ytdlp_path")]
    pub ytdlp_path: String,
    #[serde(default)]
    pub download_directory: String,
    #[serde(default = "default_recommendations_cache_limit")]
    pub recommendations_cache_limit: usize,
}

impl Default for StreamingTemplate {
    fn default() -> Self {
        Self {
            enabled: true,
            streaming_mode: false,
            recommendations_per_fetch: 8,
            listenbrainz_mode: "easy".to_string(),
            ytdlp_path: "yt-dlp".to_string(),
            download_directory: String::new(),
            recommendations_cache_limit: 30,
        }
    }
}

fn default_streaming_enabled() -> bool {
    true
}
fn default_recommendations_per_fetch() -> usize {
    8
}
fn default_recommendations_cache_limit() -> usize {
    30
}
fn default_listenbrainz_mode() -> String {
    "easy".to_string()
}
fn default_ytdlp_path() -> String {
    "yt-dlp".to_string()
}
fn default_restore_last_playlist() -> bool {
    true
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LyricsTemplate {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_true")]
    pub prefer_synced: bool,
    #[serde(default = "default_true")]
    pub save_lrc_file: bool,
    #[serde(default = "default_true")]
    pub embed_in_audio: bool,
    #[serde(default)]
    pub apple_developer_token: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for LyricsTemplate {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_synced: true,
            save_lrc_file: true,
            embed_in_audio: true,
            apple_developer_token: None,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConfigurationTemplate {
    pub songs_directory: String,
    #[serde(default)]
    pub song_directories: Vec<String>,
    #[serde(default = "default_restore_last_playlist")]
    pub restore_last_playlist: bool,
    #[serde(default)]
    pub preshuffle: bool,
    pub discord_presence: bool,
    pub theme: ThemeTemplate,
    #[serde(default)]
    pub streaming: StreamingTemplate,
    #[serde(default)]
    pub lyrics: LyricsTemplate,
}

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
}

#[derive(Clone)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub streaming_mode: bool,
    pub recommendations_per_fetch: usize,
    pub listenbrainz_mode: String,
    pub ytdlp_path: String,
    pub download_directory: String,
    pub recommendations_cache_limit: usize,
}

impl From<StreamingTemplate> for StreamingConfig {
    fn from(value: StreamingTemplate) -> Self {
        StreamingConfig {
            enabled: value.enabled,
            streaming_mode: value.streaming_mode,
            recommendations_per_fetch: value.recommendations_per_fetch,
            listenbrainz_mode: value.listenbrainz_mode,
            ytdlp_path: value.ytdlp_path,
            download_directory: value.download_directory,
            recommendations_cache_limit: value.recommendations_cache_limit,
        }
    }
}

impl std::default::Default for StreamingConfig {
    fn default() -> Self {
        StreamingConfig {
            enabled: true,
            streaming_mode: false,
            recommendations_per_fetch: 8,
            listenbrainz_mode: "easy".to_string(),
            ytdlp_path: "yt-dlp".to_string(),
            download_directory: String::new(),
            recommendations_cache_limit: 30,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LyricsConfig {
    pub enabled: bool,
    pub prefer_synced: bool,
    pub save_lrc_file: bool,
    pub embed_in_audio: bool,
    pub apple_developer_token: Option<String>,
}

impl From<LyricsTemplate> for LyricsConfig {
    fn from(val: LyricsTemplate) -> Self {
        Self {
            enabled: val.enabled,
            prefer_synced: val.prefer_synced,
            save_lrc_file: val.save_lrc_file,
            embed_in_audio: val.embed_in_audio,
            apple_developer_token: val.apple_developer_token,
        }
    }
}

impl Default for LyricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prefer_synced: true,
            save_lrc_file: true,
            embed_in_audio: true,
            apple_developer_token: None,
        }
    }
}

pub struct Configuration {
    pub songs_directory: String,
    pub song_directories: Vec<String>,
    pub restore_last_playlist: bool,
    pub preshuffle: bool,
    pub discord_presence: bool,
    pub theme: Theme,
    pub streaming: StreamingConfig,
    pub lyrics: LyricsConfig,
}

impl From<ConfigurationTemplate> for Configuration {
    fn from(value: ConfigurationTemplate) -> Self {
        Configuration {
            songs_directory: value.songs_directory,
            song_directories: value.song_directories,
            restore_last_playlist: value.restore_last_playlist,
            preshuffle: value.preshuffle,
            discord_presence: value.discord_presence,
            theme: Theme {
                primary: color_from_string(value.theme.primary),
                secondary: color_from_string(value.theme.secondary),
                accent: color_from_string(value.theme.accent),
            },
            streaming: value.streaming.into(),
            lyrics: value.lyrics.into(),
        }
    }
}

impl std::default::Default for Configuration {
    fn default() -> Self {
        Configuration {
            songs_directory: String::new(),
            song_directories: vec![],
            restore_last_playlist: true,
            preshuffle: false,
            discord_presence: false,
            theme: Theme {
                primary: Color::Reset,
                secondary: Color::DarkGrey,
                accent: Color::Blue,
            },
            streaming: StreamingConfig::default(),
            lyrics: LyricsConfig::default(),
        }
    }
}
#[derive(Serialize, Deserialize, Default)]
struct RuntimeState {
    #[serde(default)]
    last_playlist: Option<String>,
}

pub fn load_last_playlist() -> Option<String> {
    let state_path = get_runtime_state_path().ok()?;
    let content = std::fs::read_to_string(state_path).ok()?;
    let state: RuntimeState = serde_json::from_str(&content).ok()?;
    state.last_playlist
}

pub fn save_last_playlist(path: String) -> Result<()> {
    let state_path = get_runtime_state_path()?;
    let state = RuntimeState {
        last_playlist: Some(path),
    };
    let content = serde_json::to_string_pretty(&state)?;
    std::fs::write(state_path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configuration_template_defaults() {
        let content = r#"
            songs_directory = "C:/Music"
            discord_presence = false
            theme = { primary = "default", secondary = "dark-grey", accent = "blue" }
        "#;
        let config_tmpl: ConfigurationTemplate = toml::from_str(content).unwrap();
        assert_eq!(config_tmpl.preshuffle, false);
        assert_eq!(config_tmpl.streaming.streaming_mode, false);
        assert_eq!(config_tmpl.streaming.recommendations_cache_limit, 30);

        let config: Configuration = config_tmpl.into();
        assert_eq!(config.preshuffle, false);
        assert_eq!(config.streaming.streaming_mode, false);
        assert_eq!(config.streaming.recommendations_cache_limit, 30);
    }

    #[test]
    fn test_configuration_template_custom_values() {
        let content = r#"
            songs_directory = "C:/Music"
            preshuffle = true
            discord_presence = false
            theme = { primary = "default", secondary = "dark-grey", accent = "blue" }
            [streaming]
            streaming_mode = true
            recommendations_cache_limit = 50
        "#;
        let config_tmpl: ConfigurationTemplate = toml::from_str(content).unwrap();
        assert_eq!(config_tmpl.preshuffle, true);
        assert_eq!(config_tmpl.streaming.streaming_mode, true);
        assert_eq!(config_tmpl.streaming.recommendations_cache_limit, 50);

        let config: Configuration = config_tmpl.into();
        assert_eq!(config.preshuffle, true);
        assert_eq!(config.streaming.streaming_mode, true);
        assert_eq!(config.streaming.recommendations_cache_limit, 50);
    }
}
