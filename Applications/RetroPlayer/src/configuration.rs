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

# Change 'false' to 'true' to enable Discord rich presence feature.
discord_presence = false

# Colors used by the application. The following colors are supported for each of the fields below the theme section.
# default, white, grey, red, green, yellow, blue, magenta, cyan, black, dark-grey, dark-red, dark-green, dark-yellow, dark-blue, dark-magenta, dark-cyan.
[theme]
primary = \"default\"
secondary = \"dark-grey\"
accent = \"blue\"
";

pub fn get_configuration_path() -> Result<PathBuf> {
    let configuration_directory = PROJECT_DIRECTORY.config_dir();
    create_dir_all(configuration_directory)?;
    return Ok(configuration_directory.join("Configuration.toml"));
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
        CONFIGURATION.read().unwrap()
    };
}

#[derive(Serialize, Deserialize)]
pub struct ThemeTemplate {
    pub primary: String,
    pub secondary: String,
    pub accent: String,
}

#[derive(Serialize, Deserialize)]
pub struct ConfigurationTemplate {
    pub songs_directory: String,
    pub discord_presence: bool,
    pub theme: ThemeTemplate,
}

pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub accent: Color,
}

pub struct Configuration {
    pub songs_directory: String,
    pub discord_presence: bool,
    pub theme: Theme,
}

impl From<ConfigurationTemplate> for Configuration {
    fn from(value: ConfigurationTemplate) -> Self {
        Configuration {
            songs_directory: value.songs_directory,
            discord_presence: value.discord_presence,
            theme: Theme {
                primary: color_from_string(value.theme.primary),
                secondary: color_from_string(value.theme.secondary),
                accent: color_from_string(value.theme.accent),
            },
        }
    }
}

impl std::default::Default for Configuration {
    fn default() -> Self {
        Configuration {
            songs_directory: String::new(),
            discord_presence: false,
            theme: Theme {
                primary: Color::Reset,
                secondary: Color::DarkGrey,
                accent: Color::Blue,
            },
        }
    }
}
