// vsops.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::fmt::Debug;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{crate_name, Parser};
use directories::ProjectDirs;
use tokio::signal;
use tokio::sync::oneshot;

use vsop::socket::SocketFile;
use vsop::Server;

const APP_NAME: &str = crate_name!();

mod translator {
    tonic::include_proto!("translator");
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
