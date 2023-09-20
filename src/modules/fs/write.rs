use cowstr::CowStr;
use std::borrow::Cow;
use std::path::PathBuf;
use std::string::ToString;
use thiserror::Error;
use tokio::fs::{self, File, OpenOptions};
use tokio::io::{AsyncWriteExt, BufWriter};
use typed_builder::TypedBuilder;

#[derive(Error, Debug)]
pub enum FileWrite {
  #[error("Failed to create file: {0}")]
  CreateFile(CowStr),

  #[error("Failed to write to file: {0}")]
  WriteToFile(CowStr),

  #[error("Failed to get parent directory for file: {0}")]
  GetParentDirectory(CowStr),
  // Add more error variants as needed
}

#[derive(TypedBuilder)]
pub struct FileWriter<'a> {
  filename: Cow<'a, PathBuf>,
  #[builder(default = true)]
  pub create_parents: bool,
  #[builder(default = true)]
  pub truncate: bool,
}

impl<'a> FileWriter<'a> {
  pub async fn write<T>(&self, content: T) -> Result<File, FileWrite>
  where
    // Accept AsRef<str> or bytes
    T: AsRef<[u8]> + ToString,
  {
    let filename_str = self.filename.to_string_lossy();

    if self.create_parents {
      if let Some(parent) = self.filename.parent() {
        fs::create_dir_all(parent)
          .await
          .map_err(|_| FileWrite::CreateFile(parent.to_string_lossy().to_string().into()))?;
      } else {
        return Err(FileWrite::GetParentDirectory(filename_str.as_ref().into()));
      }
    }

    let file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(self.truncate)
      .open(&filename_str.as_ref())
      .await
      .map_err(|_| FileWrite::CreateFile(filename_str.as_ref().into()))?;

    let mut buf_writer = BufWriter::new(file);

    buf_writer
      .write_all(content.as_ref())
      .await
      .map_err(|_| FileWrite::WriteToFile(filename_str.as_ref().into()))?;

    // Flush the buffered data to the file
    buf_writer
      .flush()
      .await
      .map_err(|_| FileWrite::WriteToFile(filename_str.as_ref().into()))?;

    Ok(buf_writer.into_inner()) // Return the File after writing and flushing
  }
}
