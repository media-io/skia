
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

mod browser;
mod config;
mod socket;

use std::{thread, time};

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
        if let Err(msg) = s.open_channel("browser:all") {
          error!("{}", msg);
        } else {
          loop {
            match s.next_message() {
              Ok(message) => {
                match message.topic.as_ref() {
                  "watch:all" => {
                  }
                  "browser:all" => {
                    let files = browser::process(message);
                    // println!("{:?}", files);
                    s.send(files.into());
                  }
                  _ => debug!("{:?}", message),
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
}

/*
#[test]
fn parse_test() {
  if let Ok(content) = load_file("tests/AMEEncodingLog.txt") {
    let entries = parse(&content);
    assert!(entries.len() == 128);
  }
}
*/
