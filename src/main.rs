
extern crate chrono;
extern crate reqwest;
extern crate env_logger;
extern crate phoenix;
extern crate serde;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use chrono::NaiveDateTime;
use phoenix::Phoenix;
use std::{thread, time};
use std::collections::HashMap;
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

#[derive(Debug, Serialize)]
struct SessionBody {
  session: Session
}

#[derive(Debug, Serialize)]
struct Session {
  email: String,
  password: String,
}

fn main() {
  env_logger::init();

  let body = SessionBody {
    session: Session {
      email: "admin@media-io.com".to_owned(),
      password: "admin123".to_owned(),
    }
  };

  #[derive(Debug, Deserialize)]
  struct SessionReponse {
    access_token: String,
  }


  let client = reqwest::Client::new();
  let mut response = client.post("http://localhost:4000/api/sessions")
    .json(&body)
    .send().unwrap();

  if !response.status().is_success() {
    if response.status().is_server_error() {
      println!("server error!");
    } else {
      println!("Something else happened. Status: {:?}", response.status());
    }
    return;
  }

  let content: SessionReponse = response.json().unwrap();

  let token = content.access_token;
  let url = "ws://localhost:4000/socket";

  
  thread::spawn(move || {
    let mut params = HashMap::new();
    params.insert("userToken", token.as_str());

    let mut phx = Phoenix::new_with_parameters(&url, &params);
    let mutex_chan = phx.channel("watch:all").clone();
    {
      let mut device_chan = mutex_chan.lock().unwrap();
      let payload = json!({
        "identifier": "marco-dev"
      });

      device_chan.join_with_message(payload);
    }

    loop {
      match phx.out.recv() {
        Ok(_msg) => {
          //println!("user1: {:?}", msg)
        },
        Err(_err) => ()//println!("{:?}", err)
      }
    }
  });

  println!("start watching file...");
  loop {
    thread::sleep(time::Duration::from_millis(1000));
    
    //if let Ok(content) = load_file("tests/AMEEncodingLog.txt") {
    //  let entries = parse(&content);
    //  println!("{:?}", entries);
    //  println!("found {:?} transcode", entries.len());
    //}
  }
}

#[test]
fn parse_test() {
  if let Ok(content) = load_file("tests/AMEEncodingLog.txt") {
    let entries = parse(&content);
    assert!(entries.len() == 128);
  }
}