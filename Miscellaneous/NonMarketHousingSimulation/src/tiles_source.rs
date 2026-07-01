//! PMTiles and MBTiles async readers for local and remote data.

use crate::loader::TileSource;
#[cfg(target_arch = "wasm32")]
use crate::tile_backend::BrowserHttpBackend;
#[cfg(not(target_arch = "wasm32"))]
use crate::tile_backend::ReqwestHttpBackend;
use async_lock::Mutex;
use pmtiles::{AsyncPmTilesReader, TileCoord as PmtCoord, TileId};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
type Backend = ReqwestHttpBackend;
#[cfg(target_arch = "wasm32")]
type Backend = BrowserHttpBackend;

pub struct PmTilesTileSource {
    reader: Arc<Mutex<AsyncPmTilesReader<Backend>>>,
}

impl PmTilesTileSource {
    pub async fn new(url: String) -> Result<Self, String> {
        #[cfg(not(target_arch = "wasm32"))]
        let backend = ReqwestHttpBackend {
            url,
            client: reqwest::Client::builder()
                .build()
                .map_err(|e| e.to_string())?,
        };
        #[cfg(target_arch = "wasm32")]
        let backend = BrowserHttpBackend { url };

        let reader = AsyncPmTilesReader::try_from_source(backend)
            .await
            .map_err(|e| format!("PMTiles reader init fail: {:?}", e))?;

        Ok(Self {
            reader: Arc::new(Mutex::new(reader)),
        })
    }
}

impl TileSource for PmTilesTileSource {
    fn get_tile(
        &self,
        z: u8,
        x: u32,
        y: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Vec<u8>>, String>> + Send>>
    {
        let reader = Arc::clone(&self.reader);
        Box::pin(async move {
            let tile_id = TileId::from(PmtCoord::new(z, x, y).unwrap());
            let r = reader.lock().await;
            Ok(r.get_tile(tile_id)
                .await
                .map_err(|e| format!("{:?}", e))?
                .map(|b| b.to_vec()))
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct MbtilesTileSource {
    pool: mbtiles::MbtilesPool,
}

#[cfg(not(target_arch = "wasm32"))]
impl MbtilesTileSource {
    pub async fn open(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            pool: mbtiles::MbtilesPool::open_readonly(path).await?,
        })
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl TileSource for MbtilesTileSource {
    fn get_tile(
        &self,
        z: u8,
        x: u32,
        y: u32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<Vec<u8>>, String>> + Send>>
    {
        let pool = self.pool.clone();
        Box::pin(async move { pool.get_tile(z, x, y).await.map_err(|e| e.to_string()) })
    }
}
