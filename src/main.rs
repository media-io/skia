
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
use std::{thread, time};

fn main() {
  env_logger::init();

  thread::spawn(move || {
    let mut last_time = None;

    loop {
      let mut s = socket::Socket::new_from_config();
      if let Err(msg) = s.generate_token() {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_websocket() {
          error!("{}", msg);
        } else {
          if let Err(msg) = s.open_channel("browser:notification") {
            error!("{}", msg);
          } else {
            loop {
              match s.next_message() {
                Ok(message) => {
                  match message.topic.as_ref() {
                    "browser:notification" => {
                      // if Defined(Reply) = message.event {

                      // } else {}
                      println!("{:?}", message);
                      // break;
                    }
                    _ => debug!("{:?}", message),
                  }
                },
                Err(_err) => {
                  break;
                }
              }
            }
            
            loop {
              let filename = config::get_adobe_media_encoder_log_filename();
              //debug!("watching file {}", filename);

              if let Ok(logs_content) = AdobeMediaEncoderLog::open(&filename) {
                for entry in logs_content.entries {

                  if let Some(lt) = last_time {
                    if entry.date_time <= lt {
                      continue;
                    }
                  }

                  let new_time = Some(entry.date_time);
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
                  "browser:all" => {
                    let files = browser::process(message);
                    // println!("{:?}", files);
                    if let Err(_) = s.send("response", files.into()) {
                      break;
                    }
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
