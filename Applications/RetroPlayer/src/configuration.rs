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
# How many recommended tracks to fetch per refresh.
recommendations_per_fetch = 8
# ListenBrainz LB Radio mode: \"easy\", \"medium\", or \"hard\" (controls how loosely
# \"similar\" is interpreted).
listenbrainz_mode = \"easy\"
# Path to the yt-dlp executable, or just \"yt-dlp\" if it's on your PATH.
ytdlp_path = \"yt-dlp\"
# Where downloaded tracks are saved. Leave blank to use songs_directory.
download_directory = \"\"
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

#[derive(Serialize, Deserialize, Default)]
pub struct StreamingTemplate {
    #[serde(default = "default_streaming_enabled")]
    pub enabled: bool,
    #[serde(default = "default_recommendations_per_fetch")]
    pub recommendations_per_fetch: usize,
    #[serde(default = "default_listenbrainz_mode")]
    pub listenbrainz_mode: String,
    #[serde(default = "default_ytdlp_path")]
    pub ytdlp_path: String,
    #[serde(default)]
    pub download_directory: String,
}

fn default_streaming_enabled() -> bool {
    true
}
fn default_recommendations_per_fetch() -> usize {
    8
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

#[derive(Serialize, Deserialize)]
pub struct ConfigurationTemplate {
    pub songs_directory: String,
    #[serde(default)]
    pub song_directories: Vec<String>,
    #[serde(default = "default_restore_last_playlist")]
    pub restore_last_playlist: bool,
    pub discord_presence: bool,
    pub theme: ThemeTemplate,
    #[serde(default)]
    pub streaming: StreamingTemplate,
}

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
}

#[derive(Clone)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub recommendations_per_fetch: usize,
    pub listenbrainz_mode: String,
    pub ytdlp_path: String,
    pub download_directory: String,
}

impl From<StreamingTemplate> for StreamingConfig {
    fn from(value: StreamingTemplate) -> Self {
        StreamingConfig {
            enabled: value.enabled,
            recommendations_per_fetch: value.recommendations_per_fetch,
            listenbrainz_mode: value.listenbrainz_mode,
            ytdlp_path: value.ytdlp_path,
            download_directory: value.download_directory,
        }
    }
}

impl std::default::Default for StreamingConfig {
    fn default() -> Self {
        StreamingConfig {
            enabled: true,
            recommendations_per_fetch: 8,
            listenbrainz_mode: "easy".to_string(),
            ytdlp_path: "yt-dlp".to_string(),
            download_directory: String::new(),
        }
    }
}

pub struct Configuration {
    pub songs_directory: String,
    pub song_directories: Vec<String>,
    pub restore_last_playlist: bool,
    pub discord_presence: bool,
    pub theme: Theme,
    pub streaming: StreamingConfig,
}

impl From<ConfigurationTemplate> for Configuration {
    fn from(value: ConfigurationTemplate) -> Self {
        Configuration {
            songs_directory: value.songs_directory,
            song_directories: value.song_directories,
            restore_last_playlist: value.restore_last_playlist,
            discord_presence: value.discord_presence,
            theme: Theme {
                primary: color_from_string(value.theme.primary),
                secondary: color_from_string(value.theme.secondary),
                accent: color_from_string(value.theme.accent),
            },
            streaming: value.streaming.into(),
        }
    }
}

impl std::default::Default for Configuration {
    fn default() -> Self {
        Configuration {
            songs_directory: String::new(),
            song_directories: vec![],
            restore_last_playlist: true,
            discord_presence: false,
            theme: Theme {
                primary: Color::Reset,
                secondary: Color::DarkGrey,
                accent: Color::Blue,
            },
            streaming: StreamingConfig::default(),
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
