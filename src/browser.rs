
use config;
use phoenix::message::Message;
use phoenix::event::Event;
use serde_json::{Map, Value};
use std::fs;

#[derive(Debug)]
struct FileSystem {
  path: String
}

impl FileSystem {
  fn from(content: Value) -> Option<Self> {
    if let Value::Object(map) = content {
      if let Some(value) = map.get("body") {
        if let Value::Object(map) = value {
          if let Some(path) = map.get("path") {
            if let Value::String(string_path) = path {
              return Some(FileSystem {
                path: string_path.to_owned()
              })
            }
          }
        }
      }
    }

    None
  }
}

#[derive(Debug, Serialize)]
pub struct FileSystemResponse {
  pub entries: Vec<FileSystemEntry>,
}

impl From<FileSystemResponse> for Value {
  fn from(response: FileSystemResponse) -> Self {
    let mut m = Map::new();
    m.insert("entries".to_string(), response.entries.into());
    m.into()
  }
}

#[derive(Debug, Serialize)]
pub struct FileSystemEntry {
  pub filename: String,
  pub is_dir: bool,
  pub is_file: bool,
}

impl From<FileSystemEntry> for Value {
  fn from(response: FileSystemEntry) -> Self {
    let mut m = Map::new();
    m.insert("filename".to_string(), response.filename.into());
    m.insert("is_dir".to_string(), response.is_dir.into());
    m.insert("is_file".to_string(), response.is_file.into());
    m.into()
  }
}

pub fn process(message: Message) -> FileSystemResponse {
  let mut result = vec![];
  if let Event::Custom(event) = message.event {
    match event.as_str() {
      "file_system" => {
        if let Some(order) = FileSystem::from(message.payload) {

          let full_path = config::get_root_path_browsing() + &order.path;

          if let Ok(paths) = fs::read_dir(full_path) {
            for path in paths {
              if let Ok(entry) = path {
                if let Ok(metadata) = entry.metadata() {
                  result.push(FileSystemEntry{
                    filename: entry.file_name().to_str().unwrap().to_string(),
                    is_dir: metadata.is_dir(),
                    is_file: metadata.is_file(),
                  })
                }
              }
            }
          }
        }
      }
      _ => {
      }
    }
  }
  FileSystemResponse {
    entries: result
  }
}