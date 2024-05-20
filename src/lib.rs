// lib.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::future::Future;
use std::path::{Path, PathBuf};

use anyhow::{Error, Result};
use ct2rs::auto::Tokenizer as AutoTokenizer;
use ct2rs::config::Config;
use ct2rs::{TranslationOptions, Translator};
use tokio::net::{UnixListener, UnixStream};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::transport::{Channel, Endpoint, Server as TonicServer, Uri};
use tonic::{Request, Response, Status};
use tower::service_fn;

use translator::translator_client::TranslatorClient;
use translator::translator_server::{Translator as TranslationService, TranslatorServer};
use translator::{Request as TranslationRequest, Response as TranslationResponse};

pub mod socket;

mod translator {
    tonic::include_proto!("translator");
}

pub struct Client {
    inner: TranslatorClient<Channel>,
}

impl Client {
    pub async fn new(socket_file: PathBuf) -> Result<Self> {
        let channel = Endpoint::try_from("http://[::]:50051")?
            .connect_with_connector(service_fn(move |_: Uri| {
                // Connect to a Uds socket
                UnixStream::connect(socket_file.clone())
            }))
            .await?;
        Ok(Self {
            inner: TranslatorClient::new(channel),
        })
    }

    pub async fn translate(&mut self, source: Vec<String>) -> Result<Vec<String>> {
        let res = self
            .inner
            .translate(Request::new(TranslationRequest { source }))
            .await?;
        Ok(res.into_inner().result)
    }
}

pub struct Server {
    inner: Translator<AutoTokenizer>,
    options: TranslationOptions<String>,
}

impl Server {
    pub fn new<P: AsRef<Path>>(model_path: P, config: Config) -> Result<Self> {
        Ok(Self {
            inner: Translator::new(model_path, &config)?,
            options: TranslationOptions {
                beam_size: 12,
                use_vmap: true,
                ..Default::default()
            },
        })
    }

    pub async fn serve<P, F>(self, socket_file: P, signal: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: Future<Output = ()>,
    {
        TonicServer::builder()
            .add_service(TranslatorServer::new(self))
            .serve_with_incoming_shutdown(
                // Use a reference to `socket_file` to ensure it remains valid until the function
                // ends.
                UnixListenerStream::new(UnixListener::bind(&socket_file)?),
                signal,
            )
            .await
            .map_err(Error::from)
    }
}

#[tonic::async_trait]
impl TranslationService for Server {
    async fn translate(
        &self,
        req: Request<TranslationRequest>,
    ) -> Result<Response<TranslationResponse>, Status> {
        Ok(Response::new(TranslationResponse {
            result: self
                .inner
                .translate_batch(&req.get_ref().source, &self.options, None)
                .map(|r| r.into_iter().map(|(v, _)| v).collect())
                .map_err(|e| Status::from_error(e.into()))?,
        }))
    }
}
