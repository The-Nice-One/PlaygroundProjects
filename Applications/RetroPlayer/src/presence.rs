use crate::{LOGGER, logger::EntryId};
use discord_presence::{Client, models::Activity};
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
