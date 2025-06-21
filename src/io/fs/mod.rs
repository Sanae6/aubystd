use core::error::Error;
use std::{fs::Metadata, path::Path};

use super::{StreamReader, StreamWriter};

pub trait FileSystem {
  type File: File;
  type OpenOptions;
  type Permissions;
  type Metadata;

  async fn open_file(&self, path: &Path, open_args: Self::OpenOptions) -> Result<Self::File, impl Error>;
  async fn remove_file(&self, path: &Path) -> Result<(), impl Error>;
  async fn exists(&self, path: &Path) -> Result<bool, impl Error>;
  async fn metadata(&self, path: &Path) -> Result<(), impl Error>;
  async fn set_permissions(&self, path: &Path, permissions: Self::Permissions);
}

pub trait DirectoryFileSystem: FileSystem {
  type ReadDirectory;

  async fn create_directory(&self, path: &Path) -> Result<(), impl Error>;
  async fn create_directory_recursive(&self, path: &Path) -> Result<(), impl Error>;
  async fn read_directory(&self, path: &Path) -> Result<Self::ReadDirectory, impl Error>;
  async fn remove_directory(&self, path: &Path) -> Result<(), impl Error>;
}

pub trait File: StreamReader + StreamWriter {
  type Metadata;
  fn metadata(&self) -> Result<(), Metadata>;
}

pub enum SeekFrom {
  Start(u64),
  Current(i64),
  End(u64),
}
pub trait SeekableFile: File {
  fn seek(&mut self, seek_from: SeekFrom) -> Result<(), impl Error>;
  fn position(&self) -> Result<u64, impl Error>;
}
pub trait SizedFile: File {
  fn file_size(&mut self) -> Result<u64, impl Error>;
}
