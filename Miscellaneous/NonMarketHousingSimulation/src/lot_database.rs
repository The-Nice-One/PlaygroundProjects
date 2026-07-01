//! Asynchronous lot-level property data loading from binary lookup files.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::sync::Arc;

use bevy::prelude::*;
use flate2::read::GzDecoder;

use crate::RUN_LOCAL;
use crate::components::{ConstructionSite, LotData, OwnershipTier};
use crate::loader::{TokioHandle, spawn_task};
use crate::plugin::AppState;
use crate::sim_budget::HousingStats;
use crate::tile_backend::http_get_bytes;

// Paths

pub const LOT_LOOKUP_PATH: &str = "./res/lot_lookup.bin.gz";
pub const LOT_DB_URL: &str = "https://content-spf.funguylabs.app/user-uploads/lot_lookup.bin.gz";

// Binary constants

const MAGIC: &[u8; 4] = b"LOTD";
const HEADER_SIZE: usize = 12;
const RECORD_SIZE: usize = 92;

// BinTiers

/// Lightweight `Arc`-wrapped `BIN to ownership_tier` map.
/// Cloned cheaply into each tile-load tokio task via `RendererSnapshot`
/// so vertex colors can be baked at mesh-build time.
#[derive(Resource, Default, Clone)]
pub struct BinTiers(pub Arc<HashMap<u32, u8>>);

// LotDatabase

#[derive(Resource, Default)]
pub struct LotDatabase {
    /// Keyed by BIN (Building Identification Number).
    pub records: HashMap<u32, LotData>,
    pub constructions: HashMap<u32, ConstructionSite>,
}

impl LotDatabase {
    pub fn get(&self, bin: u32) -> Option<&LotData> {
        self.records.get(&bin)
    }
}

// Async-load channel resource

#[derive(Resource)]
pub struct LotDbReceiver(pub tokio::sync::oneshot::Receiver<Result<LotDatabase, String>>);

// Startup system

/// Fires a tokio task to read, decompress, and parse the binary file.
/// Runs in `Startup`, and the channel receiver is polled each frame until ready.
pub fn start_lot_db_load(mut commands: Commands, handle: Res<TokioHandle>) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    commands.insert_resource(LotDbReceiver(rx));

    spawn_task(
        async move {
            let path = if crate::RUN_LOCAL {
                LOT_LOOKUP_PATH
            } else {
                LOT_DB_URL
            };
            let result = load_and_parse(path).await;
            let _ = tx.send(result);
        },
        Some(&handle),
    );

    let source = if crate::RUN_LOCAL {
        "local disk"
    } else {
        "URL"
    };
    info!("LotDatabase: loading from {source} …");
}

// Poll system

/// Polls the oneshot receiver each frame.  On success, inserts `LotDatabase`
/// and `BinTiers` as resources and transitions to `AppState::Playing`.
/// On failure or channel closed, logs a warning and transitions anyway so the
/// map still works without property data.
pub fn poll_lot_db_load(
    mut commands: Commands,
    mut receiver: Option<ResMut<LotDbReceiver>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut stats: ResMut<HousingStats>,
) {
    let Some(ref mut rx) = receiver else {
        return;
    };

    use tokio::sync::oneshot::error::TryRecvError;
    match rx.0.try_recv() {
        Ok(Ok(db)) => {
            info!(
                "LotDatabase: {} records loaded — starting simulation",
                db.records.len()
            );

            let mut baseline_m = 0;
            let mut baseline_nm = 0;
            let mut tiers_inner = HashMap::with_capacity(db.records.len());

            for (&bin, lot) in &db.records {
                if lot.ownership.is_non_market() {
                    baseline_nm += lot.units_res as u32;
                } else if lot.ownership == OwnershipTier::Market {
                    baseline_m += lot.units_res as u32;
                }
                tiers_inner.insert(bin, lot.ownership.as_u8());
            }

            stats.baseline_market = baseline_m;
            stats.baseline_nonmarket = baseline_nm;
            commands.insert_resource(BinTiers(Arc::new(tiers_inner)));
            commands.insert_resource(db);
            commands.remove_resource::<LotDbReceiver>();
            next_state.set(AppState::Playing);
        }
        Ok(Err(e)) => {
            warn!("LotDatabase: load error — {e}  (continuing without property data)");
            commands.insert_resource(LotDatabase::default());
            commands.insert_resource(BinTiers::default());
            commands.remove_resource::<LotDbReceiver>();
            next_state.set(AppState::Playing);
        }
        Err(TryRecvError::Empty) => { /* not ready yet, try again next frame */ }
        Err(e) => {
            warn!("LotDatabase: channel error — {e}");
            commands.insert_resource(LotDatabase::default());
            commands.insert_resource(BinTiers::default());
            commands.remove_resource::<LotDbReceiver>();
            next_state.set(AppState::Playing);
        }
    }
}

// Async helpers

async fn get_compressed_bytes(url: &str) -> Result<Vec<u8>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    return Ok(tokio::fs::read(url)
        .await
        .map_err(|e| format!("read '{url}': {e}"))?);

    #[cfg(target_arch = "wasm32")]
    Ok(vec![])
}

async fn load_and_parse(url: &str) -> Result<LotDatabase, String> {
    let compressed = if RUN_LOCAL {
        get_compressed_bytes(url).await?
    } else {
        http_get_bytes(url).await?
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::task::spawn_blocking(move || parse_bytes(&compressed))
            .await
            .map_err(|e| format!("spawn_blocking: {e}"))?
    }
    #[cfg(target_arch = "wasm32")]
    {
        parse_bytes(&compressed)
    }
}

fn parse_bytes(compressed: &[u8]) -> Result<LotDatabase, String> {
    // Decompress
    let mut gz = GzDecoder::new(Cursor::new(compressed));
    let mut buf = Vec::new();
    gz.read_to_end(&mut buf)
        .map_err(|e| format!("gzip decompress: {e}"))?;

    // Validate header
    if buf.len() < HEADER_SIZE {
        return Err(format!("file too small ({} bytes)", buf.len()));
    }
    if &buf[0..4] != MAGIC {
        return Err(format!("bad magic bytes: {:?}", &buf[0..4]));
    }
    let record_size = u16::from_le_bytes([buf[6], buf[7]]) as usize;
    let count = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]) as usize;

    if record_size != RECORD_SIZE {
        return Err(format!(
            "unexpected record size {record_size} bytes (expected {RECORD_SIZE})"
        ));
    }
    let needed = HEADER_SIZE + count * RECORD_SIZE;
    if buf.len() < needed {
        return Err(format!(
            "truncated: have {} bytes, need {needed}",
            buf.len()
        ));
    }

    // Parse records
    let mut records = HashMap::with_capacity(count);
    for i in 0..count {
        let s = HEADER_SIZE + i * RECORD_SIZE;
        let r = parse_record(&buf[s..s + RECORD_SIZE]);
        records.insert(r.bin, r);
    }

    Ok(LotDatabase {
        records,
        constructions: Default::default(),
    })
}

/// Parse one 92-byte record slice into a `LotData`.
fn parse_record(b: &[u8]) -> LotData {
    debug_assert_eq!(b.len(), RECORD_SIZE);

    let bin = u32::from_le_bytes(b[0..4].try_into().unwrap());
    let bbl = u64::from_le_bytes(b[4..12].try_into().unwrap());
    let height_m = f32::from_le_bytes(b[12..16].try_into().unwrap());
    let assessed = u32::from_le_bytes(b[16..20].try_into().unwrap());
    let exempt = u32::from_le_bytes(b[20..24].try_into().unwrap());
    let lot_area = u32::from_le_bytes(b[24..28].try_into().unwrap());
    let res_far = f32::from_le_bytes(b[28..32].try_into().unwrap());
    let units_res = u16::from_le_bytes(b[32..34].try_into().unwrap());
    let units_total = u16::from_le_bytes(b[34..36].try_into().unwrap());
    let num_floors = b[36];
    // let landuse     = b[37]; // not used
    let bldg_class = [b[38], b[39]];
    let ownership = OwnershipTier::from_u8(b[40]);
    // 3 padding bytes in between
    let owner = std::str::from_utf8(&b[44..92])
        .unwrap_or("")
        .trim_matches('\0')
        .to_owned();

    LotData {
        bin,
        bbl,
        height_m,
        assessed,
        exempt,
        lot_area,
        res_far,
        units_res,
        units_total,
        num_floors,
        bldg_class,
        ownership,
        owner,
    }
}
