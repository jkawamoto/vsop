// client.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::fs::File;
use std::io::{BufRead, BufReader, Read};
use std::process::Command;

use anyhow::Result;
use clap::Parser;
use spinners::{Spinner, Spinners};
use tempfile::NamedTempFile;
use tokio::net::UnixStream;
use tonic::transport::{Endpoint, Uri};
use tonic::Request;
use tower::service_fn;

use translator::translator_client::TranslatorClient;
use translator::Request as TranslationRequest;

use crate::socket::socket_filename;

#[allow(dead_code)]
mod socket;

const APP_NAME: &str = "vsop";

pub mod translator {
    tonic::include_proto!("translator");
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to a source file.
    #[arg(short, long, value_name = "FILE")]
    source_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let file: Box<dyn Read> = if let Some(source_file) = args.source_file {
        Box::new(File::open(source_file)?)
    } else {
        let file = NamedTempFile::new()?;
        let _ = Command::new("nano").arg(file.path()).spawn()?.wait()?;
        Box::new(file)
    };

    let mut prompts: Vec<String> = vec![];
    let mut prompt = String::new();
    for line in BufReader::new(file).lines() {
        let line = line?;
        let line = line.trim();
        if line.len() == 0 {
            prompts.push(prompt);
            prompt = String::new();
        } else if line.ends_with(".") {
            prompt.push(' ');
            prompt.push_str(line);
            prompts.push(prompt);
            prompt = String::new();
        } else {
            prompt.push(' ');
            prompt.push_str(line);
        }
    }
    if !prompt.is_empty() {
        prompts.push(prompt)
    }

    let socket_file = socket_filename(APP_NAME)?;
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            // Connect to a Uds socket
            UnixStream::connect(socket_file.clone())
        }))
        .await?;

    let mut client = TranslatorClient::new(channel);
    let req = Request::new(TranslationRequest { source: prompts });
    let mut sp = Spinner::new(Spinners::Dots, "Translating...".to_string());
    let res = client.translate(req).await?;
    sp.stop_with_newline();
    for r in res.get_ref().result.iter() {
        println!("{}", r.trim());
    }

    Ok(())
}
