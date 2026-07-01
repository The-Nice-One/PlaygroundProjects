//! Native and WASM HTTP backends for fetching map tile ranges.

use pmtiles::{AsyncBackend, BackendResponse, PmtError};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct ReqwestHttpBackend {
    pub url: String,
    pub client: reqwest::Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl AsyncBackend for ReqwestHttpBackend {
    async fn read_exact(&self, offset: usize, length: usize) -> Result<BackendResponse, PmtError> {
        let range = format!("bytes={}-{}", offset, offset + length - 1);
        let resp = self
            .client
            .get(&self.url)
            .header("Range", range)
            .send()
            .await
            .map_err(|e| PmtError::DirectoryCacheError(e.to_string()))?;

        let data = resp
            .bytes()
            .await
            .map_err(|e| PmtError::DirectoryCacheError(e.to_string()))?;

        Ok(BackendResponse::new(data))
    }

    async fn read(&self, offset: usize, length: usize) -> Result<BackendResponse, PmtError> {
        self.read_exact(offset, length).await
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct BrowserHttpBackend {
    pub url: String,
}

#[cfg(target_arch = "wasm32")]
impl AsyncBackend for BrowserHttpBackend {
    async fn read_exact(&self, offset: usize, length: usize) -> Result<BackendResponse, PmtError> {
        use send_wrapper::SendWrapper;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::{Headers, RequestInit, Response};

        let window = web_sys::window().ok_or(PmtError::DirectoryCacheError("No window".into()))?;
        let headers =
            Headers::new().map_err(|_| PmtError::DirectoryCacheError("Headers fail".into()))?;

        let range_value = format!("bytes={}-{}", offset, offset + length - 1);
        headers
            .set("Range", &range_value)
            .map_err(|_| PmtError::DirectoryCacheError("Header set fail".into()))?;

        let opts = RequestInit::new();
        opts.set_method("GET");
        opts.set_headers(&headers);
        opts.set_mode(web_sys::RequestMode::Cors);

        let request_promise = window.fetch_with_str_and_init(&self.url, &opts);
        let wrapped_fetch = SendWrapper::new(JsFuture::from(request_promise));

        let resp_value = wrapped_fetch
            .await
            .map_err(|_| PmtError::DirectoryCacheError("Fetch fail".into()))?;
        let response: Response = resp_value
            .dyn_into()
            .map_err(|_| PmtError::DirectoryCacheError("Type fail".into()))?;

        if response.status() != 206 && response.status() != 200 {
            return Err(PmtError::DirectoryCacheError(format!(
                "Status {}",
                response.status()
            )));
        }

        let buffer_promise = response
            .array_buffer()
            .map_err(|_| PmtError::DirectoryCacheError("Buffer error".into()))?;
        let wrapped_buffer = SendWrapper::new(JsFuture::from(buffer_promise));
        let buffer_value = wrapped_buffer
            .await
            .map_err(|_| PmtError::DirectoryCacheError("Buffer res fail".into()))?;

        let bytes = js_sys::Uint8Array::new(&buffer_value).to_vec();
        Ok(BackendResponse::new(bytes::Bytes::from(bytes)))
    }

    async fn read(&self, offset: usize, length: usize) -> Result<BackendResponse, PmtError> {
        self.read_exact(offset, length).await
    }
}

pub async fn http_get_bytes(url: &str) -> Result<Vec<u8>, String> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let resp = reqwest::get(url).await.map_err(|e| e.to_string())?;
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        Ok(bytes.to_vec())
    }

    #[cfg(target_arch = "wasm32")]
    {
        use send_wrapper::SendWrapper;
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::JsFuture;
        use web_sys::Response;

        let window = web_sys::window().ok_or("No window")?;
        let request_promise = window.fetch_with_str(url);
        let wrapped_fetch = SendWrapper::new(JsFuture::from(request_promise));
        let resp_value = wrapped_fetch.await.map_err(|_| "Fetch fail")?;
        let response: Response = resp_value.dyn_into().map_err(|_| "Type fail")?;
        let buffer_promise = response.array_buffer().map_err(|_| "Buffer fail")?;
        let wrapped_buffer = SendWrapper::new(JsFuture::from(buffer_promise));
        let buffer_value = wrapped_buffer.await.map_err(|_| "Buffer res fail")?;
        Ok(js_sys::Uint8Array::new(&buffer_value).to_vec())
    }
}
