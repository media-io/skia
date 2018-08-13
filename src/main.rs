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

mod adobe_media_encoder_log;
mod browser;
mod config;
mod socket;

use adobe_media_encoder_log::AdobeMediaEncoderLog;
use chrono::NaiveDateTime;
use phoenix::Event::Custom;
use serde_json::Value;
use std::{thread, time};

fn main() {
  env_logger::init();
  let identifier = config::get_identifier();

  thread::spawn(move || {
    let identifier = config::get_identifier();

    loop {
      let mut last_time = None;
      let mut s = socket::Socket::new_from_config();

      if let Err(msg) = s.generate_token() {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_websocket(&identifier) {
          error!("{}", msg);
        } else {
          if let Err(msg) = s.open_channel("browser:notification") {
            error!("{}", msg);
          } else {
            let _ = s.send("get_info", json!({ "identifier": identifier }));

            loop {
              match s.next_message() {
                Ok(message) => match message.topic.as_ref() {
                  "browser:notification" => {
                    if let Custom(ref event_name) = message.event {
                      if event_name == "reply_info" {
                        if let Value::Object(map) = message.payload {
                          if let Some(value) = map.get("last_event") {
                            if let Value::String(datetime) = value {
                              if let Ok(date_time) =
                                NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%dT%H:%M:%S%.fZ")
                              {
                                last_time = Some(date_time);
                              }
                            }
                            break;
                          }
                        }
                      }
                    }
                  }
                  "phoenix" => {}
                  _ => debug!("{:?}", message),
                },
                Err(_err) => {
                  break;
                }
              }
            }

            info!("start watching with last time: {:?}", last_time);

            loop {
              let filename = config::get_adobe_media_encoder_log_filename();
              //debug!("watching file {}", filename);

              if let Ok(logs_content) = AdobeMediaEncoderLog::open(&filename) {
                for mut entry in logs_content.entries {
                  if let Some(lt) = last_time {
                    if entry.date_time <= lt {
                      continue;
                    }
                  }

                  let new_time = Some(entry.date_time);
                  let mounted_path = config::get_mounted_name_path_browsing();
                  let root_path = config::get_root_path_browsing();
                  entry.output_filename = match entry.output_filename {
                    Some(ref output_filename) => {
                      Some(output_filename.replace(&mounted_path, &root_path))
                    }
                    None => None,
                  };
                  // entry.output_filename.map(|x| x.replace(&mounted_path, &root_path));

                  if let Err(_) = s.send("new_item", entry.into()) {
                    break;
                  } else {
                    last_time = new_time;
                  }
                }
              }
              thread::sleep(time::Duration::from_millis(10000));
            }
          }
        }
      }

      thread::sleep(time::Duration::from_millis(10000));
    }
  });

  loop {
    let mut s = socket::Socket::new_from_config();
    if let Err(msg) = s.generate_token() {
      error!("{}", msg);
    } else {
      if let Err(msg) = s.open_websocket(&identifier) {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_channel("browser:all") {
          error!("{}", msg);
        } else {
          loop {
            match s.next_message() {
              Ok(message) => {
                match message.topic.as_ref() {
                  "browser:all" => {
                    let files = browser::process(message);
                    // println!("{:?}", files);
                    if let Err(_) = s.send("response", files.into()) {
                      break;
                    }
                  }
                  "phoenix" => {}
                  _ => debug!("{:?}", message),
                }
              }
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
