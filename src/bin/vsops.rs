// vsops.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::fmt::{Debug, Display};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{crate_name, Parser};
use tokio::signal;
use tokio::sync::oneshot;

use vsop::socket::SocketFile;
use vsop::Server;

mod translator {
    tonic::include_proto!("translator");
}

/// A translation server using CTranslate2.
///
/// This server creates a UNIX domain socket, listens for translation requests, and handles them
/// accordingly.
///
/// By default, a socket file is created in the user’s data directory.
/// If the `--socket-file` flag is used to specify an alternative path, the socket file will be
/// created at that location.
///
/// The model specified by the `--model` flag will be downloaded from Hugging Face and loaded.
/// If the `--model-dir` flag is used to specify a directory path, the model within that directory
/// will be loaded instead.
#[derive(Parser, Debug)]
#[command(name = "vsops", author, version, long_about)]
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
        None => find_model_dir(args.model)?,
    };

    let (tx, rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        if signal::ctrl_c().await.is_ok() {
            tx.send(()).ok();
        }
    });

    let socket_file = match args.socket_file {
        Some(path) => SocketFile::with_path(path)?,
        None => SocketFile::new(crate_name!())?,
    };
    let server = Server::new(model_dir)?;
    server
        .serve(socket_file, async move {
            if let Err(e) = rx.await {
                println!("failed to receive a signal: {e}");
            }
        })
        .await?;

    Ok(())
}

fn find_model_dir<T: Display>(model: T) -> Result<PathBuf> {
    let api = hf_hub::api::sync::Api::new()?;
    let repo = api.model(format!("jkawamoto/{}-ct2", model));

    let mut res = None;
    for f in repo.info()?.siblings {
        let path = repo.get(&f.rfilename)?;
        if res.is_none() {
            res = path.parent().map(PathBuf::from);
        }
    }

    res.ok_or_else(|| anyhow!("no model files are found"))
}
