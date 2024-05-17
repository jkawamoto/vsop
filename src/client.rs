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

use clap::Parser;
use spinners::{Spinner, Spinners};
use tempfile::NamedTempFile;
use tonic::Request;

use translator::translator_client::TranslatorClient;
use translator::Request as TranslationRequest;

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
        if line.len() == 0 {
            prompts.push(prompt);
            prompt = String::new();
        } else {
            prompt.push(' ');
            prompt.push_str(&line);
        }
    }
    if !prompt.is_empty() {
        prompts.push(prompt)
    }

    let mut client = TranslatorClient::connect("http://[::1]:50051").await?;
    let req = Request::new(TranslationRequest { source: prompts });
    let mut sp = Spinner::new(Spinners::Dots, "Translating...".to_string());
    let res = client.translate(req).await?;
    sp.stop_with_newline();
    for r in res.get_ref().result.iter() {
        println!("{}", r.trim());
    }

    Ok(())
}
