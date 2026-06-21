use crate::{LOGGER, logger::EntryId, player::PlayerSession};
use discord_presence::{
    Client,
    models::{Activity, ActivityButton, ActivityType},
};
use lofty::{
    file::TaggedFileExt,
    probe::Probe,
    tag::{ItemKey, ItemValue},
};
use std::{
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};

pub fn start_discord_rpc(
    discord_rpc: &Arc<Mutex<Option<Client>>>,
    discord_rpc_handles: &mut Vec<JoinHandle<()>>,
) {
    let discord_rpc_pointer = Arc::clone(discord_rpc);
    let handle = spawn(move || {
        let mut guard = discord_rpc_pointer.lock().unwrap();
        if let Some(client) = &mut *guard {
            client.start();
            client
                .on_ready(move |_| {
                    LOGGER
                        .lock()
                        .unwrap()
                        .get_entry(EntryId::DiscordPresence)
                        .unwrap()
                        .complete("Connected to discord");
                })
                .persist();
        }
    });
    discord_rpc_handles.push(handle);
}

pub fn update_discord_rpc(
    discord_rpc: &Arc<Mutex<Option<Client>>>,
    discord_rpc_handles: &mut Vec<JoinHandle<()>>,
    activity: Activity,
) {
    let discord_rpc_pointer = Arc::clone(discord_rpc);
    let handle = spawn(move || {
        let mut guard = discord_rpc_pointer.lock().unwrap();
        if let Some(client) = &mut *guard {
            let result = client.set_activity(|_| activity);
            if result.is_err() {
                LOGGER
                    .lock()
                    .unwrap()
                    .get_entry(EntryId::DiscordPresence)
                    .unwrap()
                    .fail("Failed to set discord presence");
            } else {
                LOGGER
                    .lock()
                    .unwrap()
                    .get_entry(EntryId::DiscordPresence)
                    .unwrap()
                    .complete("Set discord presence");
            }
        }
    });
    discord_rpc_handles.push(handle);
}

pub fn build_activity(player: &PlayerSession) -> Activity {
    // Fetch video URLs from ID3 tags specifically mapping to AudioSourceUrl (WOAS)
    let mut video_ids = Vec::new();

    // Limit to max 50 items so we don't exceed YouTube URL string caps
    for (_song_title, file_path) in player.songs.iter().take(50) {
        if let Ok(tagged_file) = Probe::open(file_path).and_then(|p| p.read()) {
            if let Some(tag) = tagged_file
                .primary_tag()
                .or_else(|| tagged_file.first_tag())
            {
                if let Some(url_item) = tag.get(&ItemKey::AudioSourceUrl) {
                    let url_str = match url_item.value() {
                        ItemValue::Locator(s) => s.as_str(),
                        ItemValue::Text(s) => s.as_str(),
                        _ => "",
                    };

                    // Parse video ID from common YouTube links
                    if let Some(idx) = url_str.find("v=") {
                        let id = url_str[idx + 2..].split('&').next().unwrap_or("");
                        if !id.is_empty() {
                            video_ids.push(id.to_string());
                        }
                    } else if let Some(idx) = url_str.find("youtu.be/") {
                        let id = url_str[idx + 9..].split('?').next().unwrap_or("");
                        if !id.is_empty() {
                            video_ids.push(id.to_string());
                        }
                    }
                }
            }
        }
    }

    let mut activity = Activity::new()
        .activity_type(ActivityType::Listening)
        .state(
            player
                .current()
                .unwrap_or((&String::from("None"), &String::from("None")))
                .0
                .clone(),
        );

    if !video_ids.is_empty() {
        let playlist_url = format!(
            "https://www.youtube.com/watch_videos?video_ids={}",
            video_ids.join(",")
        );
        activity = activity.append_buttons(|_| ActivityButton {
            label: Some("Listen on YouTube".to_string()),
            url: Some(playlist_url),
        });
    }

    activity
}
