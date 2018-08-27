extern crate chrono;
extern crate clap;
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
use clap::{Arg, App};
use phoenix::Event::Custom;
use serde_json::Value;
use std::{thread, time};

fn main() {
  let matches =
    App::new("Watcher")
    .version("0.1.0")
    .author("Marc-Antoine Arnaud <maarnaud@media-io.com>")
    .about("Watcher agent to detect Adome Media Encoder updates and provide file browser")
    .arg(Arg::with_name("identifier")
      .short("i")
      .long("identifier")
      .help("Configure an indentifier for this agent.")
      .takes_value(true))
    .arg(Arg::with_name("hostname")
      .short("h")
      .long("hostname")
      .help("Configure the hostname.")
      .takes_value(true))
    .arg(Arg::with_name("port")
      .short("p")
      .long("port")
      .help("Configure the port.")
      .takes_value(true))
    .arg(Arg::with_name("secure")
      .short("s")
      .long("secure")
      .help("Set a secured connection (use HTTPS and WSS).")
      .takes_value(true))
    .arg(Arg::with_name("username")
      .short("u")
      .long("username")
      .help("Authorisation username.")
      .takes_value(true))
    .arg(Arg::with_name("password")
      .long("password")
      .help("Authorisation password.")
      .takes_value(true))
    .arg(Arg::with_name("ame_log_filename")
      .short("l")
      .long("ame-log-filename")
      .help("Configure Adobe Media Encoder log filename.")
      .takes_value(true))
    .arg(Arg::with_name("mounted_browsing_path")
      .short("m")
      .long("mounted-browsing-path")
      .help("Configure the mounted browsing path.")
      .takes_value(true))
    .arg(Arg::with_name("root_path_browsing")
      .short("r")
      .long("root-path-browsing")
      .help("Configure the root of the browsing path.")
      .takes_value(true))
    .arg(Arg::with_name("v")
      .short("v")
      .multiple(true)
      .help("Sets the level of verbosity (warn, info, debug, trace)"))
    .get_matches();

  let log_level =
    match matches.occurrences_of("v") {
      0 => "warn",
      1 => "info",
      2 => "debug",
      3 | _ => "trace",
    };

  let env = env_logger::Env::default()
    .filter_or(env_logger::DEFAULT_FILTER_ENV, log_level);
 
  env_logger::Builder::from_env(env).init();

  let identifier = config::get_identifier(matches.value_of("identifier"));
  let root_path_browsing = config::get_root_path_browsing(matches.value_of("root_path_browsing"));

  let hostname = config::get_backend_hostname(matches.value_of("hostname"));
  let port = config::get_backend_port(matches.value_of("port"));
  let username = config::get_backend_username(matches.value_of("username"));
  let password = config::get_backend_password(matches.value_of("password"));
  let secure = config::get_backend_secure(matches.value_of("secure"));

  thread::spawn(move || {
    let identifier = config::get_identifier(matches.value_of("identifier"));

    loop {
      let mut last_time = None;
      let mut s = socket::Socket::new_from_config();

      if let Err(msg) = s.generate_token() {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_websocket(&identifier) {
          error!("{}", msg);
        } else {
          if let Err(msg) = s.open_channel(&identifier, "browser:notification") {
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
              let filename = config::get_adobe_media_encoder_log_filename(matches.value_of("ame_log_filename"));
              //debug!("watching file {}", filename);

              if let Ok(logs_content) = AdobeMediaEncoderLog::open(&filename) {
                for mut entry in logs_content.entries {
                  if let Some(lt) = last_time {
                    if entry.date_time <= lt {
                      continue;
                    }
                  }

                  let new_time = Some(entry.date_time);
                  let mounted_path = config::get_mounted_name_path_browsing(matches.value_of("mounted_browsing_path"));
                  let root_path = config::get_root_path_browsing(matches.value_of("root_path_browsing"));
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
    let mut s =
      socket::Socket::new(
        &hostname,
        &port,
        &username,
        &password,
        &secure
      );

    if let Err(msg) = s.generate_token() {
      error!("{}", msg);
    } else {
      if let Err(msg) = s.open_websocket(&identifier) {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_channel(&identifier, "browser:all") {
          error!("{}", msg);
        } else {
          loop {
            match s.next_message() {
              Ok(message) => {
                match message.topic.as_ref() {
                  "browser:all" => {
                    let files = browser::process(message, &root_path_browsing);
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
