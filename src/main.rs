
extern crate chrono;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate phoenix;
extern crate reqwest;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

mod config;
mod socket;

use chrono::NaiveDateTime;
use std::{thread, time};
use std::fs::File;
use std::io::prelude::*;


#[derive(Debug)]
struct Entry {
  date_time: NaiveDateTime,
  input_filename: Option<String>,
  output_filename: Option<String>,
  preset: Option<String>,
}

fn to_u16(s8: &mut [u8]) -> &mut [u16] {
    unsafe {
      std::slice::from_raw_parts_mut(s8.as_mut_ptr() as *mut u16, s8.len() / 2)
    }
}

fn load_file(filename: &str) -> Result<String, String> {
  let mut f = File::open(filename).expect("Unable to open file {filename}");

  let mut contents = Vec::new();
  f.read_to_end(&mut contents).expect("something went wrong reading the file");

  String::from_utf16(to_u16(contents.as_mut_slice())).map_err(|e| e.to_string())
}

fn parse(content: &str) -> Vec<Entry> {
  let mut result = vec![];

  let lines : Vec<&str> = content.split("\r\n").collect();
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

        let line = lines[index-index_back];
        if *line == "".to_string() {
          result.push(Entry{
            date_time,
            input_filename,
            output_filename,
            preset,
          });
          break;
        }
        if line.starts_with(" - Output File: ") {
          output_filename =
            Some(line.replace(" - Output File: ", "")
            .replace("\\", "/"));
        }
        if line.starts_with(" - Input File: ") {
          output_filename =
            Some(line.replace(" - Input File: ", "")
            .replace("\\", "/"));
        }
        if line.starts_with(" - Preset Used: ") {
          preset =
            Some(line.replace(" - Preset Used: ", "")
            .replace("\\", "/"));
        }
        index_back += 1;
      }
    }
  }

  result
}

fn main() {
  env_logger::init();

  loop {
    let mut s = socket::Socket::new_from_config();
    if let Err(msg) = s.generate_token() {
      error!("{}", msg);
    } else {
      if let Err(msg) = s.open_websocket() {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_channel("watch:all") {
          error!("{}", msg);
        } else {
          loop {
            match s.next_message() {
              Ok(message) => {
                if message.topic.as_str() == "watch:all" {
                  println!("{:?}", message);
                }
              },
              Err(_err) => {
                break;
              }
            }
          }
        }
      }
    }

    thread::sleep(time::Duration::from_millis(1000));
    debug!("retry to connect ...");
  }
  
  /*
    //if let Ok(content) = load_file("tests/AMEEncodingLog.txt") {
    //  let entries = parse(&content);
    //  println!("{:?}", entries);
    //  println!("found {:?} transcode", entries.len());
    //}
  }*/
}

#[test]
fn parse_test() {
  if let Ok(content) = load_file("tests/AMEEncodingLog.txt") {
    let entries = parse(&content);
    assert!(entries.len() == 128);
  }
}