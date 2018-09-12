use chrono::NaiveDateTime;
use serde_json::{Map, Value};
use std;
use std::fs::File;
use std::io::prelude::*;

#[derive(Debug)]
pub struct AdobeMediaEncoderLog {
  pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize)]
pub struct Entry {
  pub date_time: NaiveDateTime,
  pub input_filename: Option<String>,
  pub output_filename: Option<String>,
  pub preset: Option<String>,
}

impl From<Entry> for Value {
  fn from(entry: Entry) -> Self {
    let mut m = Map::new();
    let date_time = format!("{}", entry.date_time.format("%Y-%m-%dT%H:%M:%S"));
    // println!("{:?}", date_time);
    m.insert("date_time".to_owned(), date_time.into());
    if let Some(output_filename) = entry.output_filename {
      m.insert("output_filename".to_owned(), output_filename.into());
    }
    if let Some(preset) = entry.preset {
      m.insert("preset".to_owned(), preset.into());
    }
    m.into()
  }
}

fn to_u16(s8: &mut [u8]) -> &mut [u16] {
  unsafe { std::slice::from_raw_parts_mut(s8.as_mut_ptr() as *mut u16, s8.len() / 2) }
}

fn load_file(filename: &str) -> Result<String, String> {
  let mut f = File::open(filename).map_err(|e| e.to_string())?;

  let mut contents = Vec::new();
  f.read_to_end(&mut contents)
    .expect("something went wrong reading the file");

  String::from_utf16(to_u16(contents.as_mut_slice())).map_err(|e| e.to_string())
}

impl AdobeMediaEncoderLog {
  pub fn open(filename: &str) -> Result<Self, String> {
    let mut entries = vec![];
    let content = load_file(filename)?;

    let lines: Vec<&str> = content.split("\r\n").collect();
    for (index, line) in lines.iter().enumerate() {
      if *line == "".to_string() {
        continue;
      }

      if line.ends_with(" : File Successfully Encoded") {
        let dt = line.replace(" : File Successfully Encoded", "");
        let date_time = NaiveDateTime::parse_from_str(&dt, "%m/%d/%Y %I:%M:%S %p").unwrap();

        let mut index_back = 1;
        let mut input_filename = None;
        let mut output_filename = None;
        let mut preset = None;

        loop {
          if index_back > index {
            break;
          }

          let line = lines[index - index_back];
          if *line == "".to_string() {
            entries.push(Entry {
              date_time,
              input_filename,
              output_filename,
              preset,
            });
            break;
          }
          if line.starts_with(" - Output File: ") {
            output_filename = Some(line.replace(" - Output File: ", "").replace("\\", "/"));
          }
          if line.starts_with(" - Input File: ") {
            output_filename = Some(line.replace(" - Input File: ", "").replace("\\", "/"));
          }
          if line.starts_with(" - Preset Used: ") {
            preset = Some(line.replace(" - Preset Used: ", "").replace("\\", "/"));
          }
          index_back += 1;
        }
      }
    }

    Ok(AdobeMediaEncoderLog { entries: entries })
  }
}
