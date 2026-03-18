// src-tauri/src/ftp_engine.rs
// FTP Engine — banner probe, connect, download

#![allow(unused_imports)]
use suppaftp::FtpStream;

pub use crate::{
  DownloadReport,
  FtpConfig,
  download_ftp_file,
  ftp_banner_probe,
  ftp_connect_with_retry,
  ftp_nlst_root_with_fallback,
  get_ftp_folders,
  resolve_ftp_config,
  resolve_socket_addrs,
};

#[allow(dead_code)]
fn _typecheck(_: Option<FtpStream>) {}
