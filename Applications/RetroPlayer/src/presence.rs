use discord_presence::{Client, models::Activity};
use std::{
    sync::{Arc, Mutex},
    thread::{JoinHandle, spawn},
};

pub fn start_discord_rpc(
    discord_rpc: &Arc<Mutex<Option<Client>>>,
    discord_rpc_handles: &mut Vec<JoinHandle<()>>,
) {
    let discord_rpc_pointer = Arc::clone(&discord_rpc);
    let handle = spawn(move || {
        let mut guard = discord_rpc_pointer.lock().unwrap();
        if let Some(client) = &mut *guard {
            client.start();
        }
    });
    discord_rpc_handles.push(handle);
}

pub fn update_discord_rpc(
    discord_rpc: &Arc<Mutex<Option<Client>>>,
    discord_rpc_handles: &mut Vec<JoinHandle<()>>,
    activity: Activity,
) {
    let discord_rpc_pointer = Arc::clone(&discord_rpc);
    let handle = spawn(move || {
        let mut guard = discord_rpc_pointer.lock().unwrap();
        if let Some(client) = &mut *guard {
            client.set_activity(|_| activity).unwrap();
        }
    });
    discord_rpc_handles.push(handle);
}
