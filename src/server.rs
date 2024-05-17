// server.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use anyhow::{anyhow, Result};
use clap::Parser;
use ct2rs::auto::Tokenizer as AutoTokenizer;
use ct2rs::TranslationOptions;
use directories::ProjectDirs;
use std::fmt::Debug;
use std::path::Path;
use tonic::{transport::Server, Request, Response, Status};
use translator::{
    translator_server, Request as TranslationRequest, Response as TranslationResponse,
};

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
                max_input_length: 0,
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

const APP_NAME: &str = "vsop";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the model to be used.
    #[arg(short, long, value_name = "NAME", default_value = "fugumt-en-ja")]
    model: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = crate::Args::parse();
    let dirs = ProjectDirs::from("", "", crate::APP_NAME)
        .ok_or_else(|| anyhow!("failed to find home directory"))?;

    let model_path = dirs.cache_dir().join(args.model);
    let addr = "[::1]:50051".parse()?;

    Server::builder()
        .add_service(translator_server::TranslatorServer::new(Translator::new(
            model_path,
        )?))
        .serve(addr)
        .await?;

    Ok(())
}
