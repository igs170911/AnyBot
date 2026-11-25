use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures_util::StreamExt;

use crate::http_wire::{HttpWire, WireBody, WireOpen, WireRequest, WireResponse};
use crate::transport::TransportFail;

pub(crate) struct ReqwestWire {
    client: reqwest::Client,
}

impl ReqwestWire {
    pub(crate) fn new(client: reqwest::Client) -> Arc<Self> {
        Arc::new(Self { client })
    }

    fn builder(&self, request: &WireRequest) -> reqwest::RequestBuilder {
        let mut builder = match request.method {
            crate::http_wire::WireMethod::Get => self.client.get(&request.url),
            crate::http_wire::WireMethod::Post => self.client.post(&request.url),
        };
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if request.body.is_empty() {
            return builder;
        }
        builder
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(request.body.clone())
    }
}

impl HttpWire for ReqwestWire {
    fn send<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireResponse, TransportFail>> + Send + 'a>> {
        Box::pin(async move {
            let response = self
                .builder(request)
                .send()
                .await
                .map_err(|_| TransportFail::Unreachable)?;
            let status = response.status().as_u16();
            let headers = header_pairs(response.headers());
            let body = response.bytes().await.map_err(|_| TransportFail::Dropped)?;
            Ok(WireResponse {
                status,
                headers,
                body,
            })
        })
    }

    fn open<'a>(
        &'a self,
        request: &'a WireRequest,
    ) -> Pin<Box<dyn Future<Output = Result<WireOpen, TransportFail>> + Send + 'a>> {
        Box::pin(async move {
            let response = self
                .builder(request)
                .send()
                .await
                .map_err(|_| TransportFail::Unreachable)?;
            let status = response.status().as_u16();
            let headers = header_pairs(response.headers());
            let stream = response
                .bytes_stream()
                .map(|item| item.map_err(|_| TransportFail::Dropped));
            Ok(WireOpen {
                status,
                headers,
                body: WireBody::Stream(Box::pin(stream)),
            })
        })
    }
}

fn header_pairs(headers: &reqwest::header::HeaderMap) -> Vec<(String, String)> {
    headers
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|text| (name.as_str().to_owned(), text.to_owned()))
        })
        .collect()
}
