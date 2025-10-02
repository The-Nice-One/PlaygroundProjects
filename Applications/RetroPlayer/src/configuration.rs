use retro_engine::Stylize;
use serde::{Deserialize, Serialize};
use std::env::current_exe;
use std::fs::{read, write};

#[derive(Serialize, Deserialize)]
pub struct Configuration {
    pub songs_directory: String,
    pub discord_presence: bool,
}

pub fn get_configuration_file() -> Option<(String, Vec<u8>)> {
    let exe_path = current_exe();
    if exe_path.is_err() {
        println!(
            "{} Could not find executable path. Read permission may be denied.",
            "[ Error: ".red()
        );
        return None;
    }

    if let Some(exe_dir) = exe_path.unwrap().parent() {
        let configuration_path = exe_dir
            .join("Configuration.toml")
            .to_str()
            .unwrap()
            .to_string();

        let file = read(&configuration_path);
        if file.is_err() {
            println!(
                "{} Configuration file not found in executable directory. Attempting to create one...",
                "[ Error: ".red()
            );
            let write_result = write(
                &configuration_path,
                "# Inside the \"\" (quotes) type the path to your song directory.
songs_directory = \"\"

# Change 'false' to 'true' to enable Discord rich presence feature.
discord_presence = false",
            );
            if write_result.is_err() {
                println!(
                    "{} Could not write configuration file. Write permission may be denied.",
                    "[ Error: ".red()
                );
            } else {
                println!(
                    "{} Configuration file created successfully. You can edit it at {}",
                    "[ Success: ".green(),
                    &configuration_path
                );
            }
            return None;
        }
        return Some((configuration_path, file.unwrap()));
    }
    None
}
