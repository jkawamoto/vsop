// socket.rs
//
// Copyright (c) 2024 Junpei Kawamoto
//
// This software is released under the MIT License.
//
// http://opensource.org/licenses/mit-license.php

use std::fmt::{Debug, Display, Formatter};
use std::fs::{create_dir_all, remove_file};
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use anyhow::Result;
use directories::ProjectDirs;

#[derive(Debug)]
pub struct SocketFile {
    path: PathBuf,
}

impl SocketFile {
    pub fn new<T: AsRef<str>>(app_name: T) -> Result<Self> {
        SocketFile::with_path(socket_filename(app_name)?)
    }

    pub fn with_path(path: PathBuf) -> Result<Self> {
        if path.exists() {
            remove_file(&path)?;
        }
        Ok(Self { path })
    }
}

impl Display for SocketFile {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}

impl AsRef<Path> for SocketFile {
    fn as_ref(&self) -> &Path {
        self.path.as_path()
    }
}

impl Drop for SocketFile {
    fn drop(&mut self) {
        if !self.path.exists() {
            return;
        }
        if let Err(e) = remove_file(&self.path) {
            println!("failed to remove socket file {}: {}", &self, e);
        }
    }
}

pub fn socket_filename<T: AsRef<str>>(app_name: T) -> Result<PathBuf> {
    let dirs = ProjectDirs::from("", "", app_name.as_ref())
        .ok_or_else(|| anyhow!("failed to find home directory"))?;
    let data_dir = dirs.data_dir();
    if !data_dir.exists() {
        create_dir_all(data_dir)?;
    }

    Ok(data_dir.join(format!("{}.socket", app_name.as_ref())))
}
