// server.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::fmt::Debug;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use clap::{crate_name, Parser};
use ct2rs::auto::Tokenizer as AutoTokenizer;
use ct2rs::TranslationOptions;
use directories::ProjectDirs;
use tokio::net::UnixListener;
use tokio::signal;
use tokio::sync::oneshot;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{transport::Server, Request, Response, Status};

use translator::{
    translator_server, Request as TranslationRequest, Response as TranslationResponse,
};

use crate::socket::SocketFile;

#[allow(dead_code)]
mod socket;

const APP_NAME: &str = crate_name!();

mod translator {
    tonic::include_proto!("translator");
}

struct Translator {
    inner: ct2rs::Translator<AutoTokenizer>,
    options: TranslationOptions<String>,
}

impl Translator {
    fn new<P: AsRef<Path>>(model_path: P) -> Result<Self> {
        Ok(Self {
            inner: ct2rs::Translator::new(model_path, &Default::default())?,
            options: TranslationOptions {
                beam_size: 12,
                use_vmap: true,
                ..Default::default()
            },
        })
    }
}

#[tonic::async_trait]
impl translator_server::Translator for Translator {
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Specifies the name of the model to be used.
    #[arg(short, long, value_name = "NAME", default_value = "fugumt-en-ja")]
    model: String,
    /// Loads the model from the specified directory.
    #[arg(long, value_name = "DIR")]
    model_dir: Option<PathBuf>,
    /// Specifies the path to the socket file.
    #[arg(long)]
    socket_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let model_dir = match args.model_dir {
        Some(model_dir) => model_dir,
        None => ProjectDirs::from("", "", APP_NAME)
            .ok_or_else(|| anyhow!("failed to find home directory"))?
            .cache_dir()
            .join(args.model),
    };

    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        if signal::ctrl_c().await.is_ok() {
            tx.send(()).ok();
        }
    });

    let socket_file = match args.socket_file {
        Some(path) => SocketFile::with_path(path)?,
        None => SocketFile::new(APP_NAME)?,
    };
    Server::builder()
        .add_service(translator_server::TranslatorServer::new(Translator::new(
            model_dir,
        )?))
        .serve_with_incoming_shutdown(
            UnixListenerStream::new(UnixListener::bind(&socket_file)?),
            async move {
                if let Err(e) = rx.await {
                    println!("failed to receive a signal: {e}");
                }
            },
        )
        .await?;

    Ok(())
}
