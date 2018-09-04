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
extern crate tokio_core;
extern crate websocket;

mod adobe_media_encoder_log;
mod browser;
mod config;
mod socket;
mod uploader;

use adobe_media_encoder_log::AdobeMediaEncoderLog;
use chrono::NaiveDateTime;
use clap::{Arg, App};
use phoenix::{Event, PhoenixEvent};
use serde_json::Value;
use std::{thread, time};
use websocket::futures::sync::mpsc;
use websocket::futures::Stream;
use tokio_core::reactor::Core;

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

  let root_path_browsing = config::get_root_path_browsing(matches.value_of("root_path_browsing"));

  let hostname = config::get_backend_hostname(matches.value_of("hostname"));
  let identifier = config::get_identifier(matches.value_of("identifier"));
  let password = config::get_backend_password(matches.value_of("password"));
  let port = config::get_backend_port(matches.value_of("port"));
  let secure = config::get_backend_secure(matches.value_of("secure"));
  let username = config::get_backend_username(matches.value_of("username"));

  let m = matches.clone();

  thread::spawn(move || {
    let hostname = config::get_backend_hostname(m.value_of("hostname"));
    let identifier = config::get_identifier(m.value_of("identifier"));
    let password = config::get_backend_password(m.value_of("password"));
    let port = config::get_backend_port(m.value_of("port"));
    let secure = config::get_backend_secure(m.value_of("secure"));
    let username = config::get_backend_username(m.value_of("username"));

    let b_secure = match secure.as_str() {
      "true" | "True" | "TRUE" | "1" => true,
      _ => false,
    };

    let mut upload_ws =
      if b_secure {
        "wss://".to_owned()
      } else {
        "ws://".to_owned()
      };
    upload_ws += &hostname;
    upload_ws += ":";
    if &hostname != "127.0.0.1" &&
      &hostname != "localhost" &&
      &hostname != "0.0.0.0"  {
      upload_ws += &port;
      upload_ws += "/upload";
    } else {
      upload_ws += "4010";
    }

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
        let (sender, emitter) = mpsc::channel(0);
        let (callback, mut messages) = mpsc::channel(0);
        if let Err(msg) = s.open_websocket(&sender, emitter, &callback, &identifier) {
          error!("{}", msg);
        } else {
          if let Err(msg) = s.open_channel(&identifier, "transfer:upload") {
            error!("{}", msg);
          } else {

            let runner =
              messages
              .for_each(|message| {
                debug!("{:?}", message);
                match message.topic.as_ref() {
                  "transfer:upload" => {
                    match uploader::process(&upload_ws, message) {
                      Ok(msg) => {
                        let _ = s.send("upload_completed", msg.into());
                      }
                      Err(msg) => {
                        let _ = s.send("upload_error", msg.into());
                      }
                    }
                  }
                  "phoenix" => {
                    if message.event == Event::Defined(PhoenixEvent::Close) {
                      return Err(());
                    }
                  }
                  _ => { debug!("{:?}", message); }
                }
                Ok(())
              });


            let mut core = Core::new().unwrap();
            if let Err(msg) = core.run(runner) {
              error!("{:?}", msg);
            }
          }
        }
      }

      thread::sleep(time::Duration::from_millis(1000));
      warn!("retry to connect ...");
    }
  });

  thread::spawn(move || {
    let hostname = config::get_backend_hostname(matches.value_of("hostname"));
    let identifier = config::get_identifier(matches.value_of("identifier"));
    let password = config::get_backend_password(matches.value_of("password"));
    let port = config::get_backend_port(matches.value_of("port"));
    let secure = config::get_backend_secure(matches.value_of("secure"));
    let username = config::get_backend_username(matches.value_of("username"));

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
        let (sender, emitter) = mpsc::channel(0);
        let (callback, mut messages) = mpsc::channel(0);
        if let Err(msg) = s.open_websocket(&sender, emitter, &callback, &identifier) {
          error!("{}", msg);
        } else {
          if let Err(msg) = s.open_channel(&identifier, "browser:notification") {
            error!("{}", msg);
          } else {
            let _ = s.send("get_info", json!({ "identifier": identifier }));

            let runner =
              messages
              .filter_map(|message| {
                debug!("{:?}", message);

                match message.topic.as_ref() {
                  "browser:notification" => {
                    debug!("browser:notification: {:?}", message);
                    if let Event::Custom(ref event_name) = message.event {
                      if event_name == "reply_info" {
                        if let Value::Object(ref map) = message.payload {
                          if let Some(value) = map.get("last_event") {
                            if let Value::String(datetime) = value {
                              if let Ok(date_time) =
                                NaiveDateTime::parse_from_str(&datetime, "%Y-%m-%dT%H:%M:%S%.fZ")
                              {
                                return Some(Some(date_time));
                              } else {
                                return Some(None);
                              }
                            } else {
                              return Some(None);
                            }
                          }
                        }
                      }
                    }
                  }
                  "phoenix" => {
                    if message.event == Event::Defined(PhoenixEvent::Close) {
                      return None;
                    }
                  }
                  _ => { debug!("{:?}", message);}
                }

                None
              })
              .take(1)
              .collect();

            let mut core = Core::new().unwrap();
            match core.run(runner) {
              Err(msg) => {
                error!("{:?}", msg);
              }
              Ok(database_recorded_time) => {
                let mut last_time =
                  if let Some(first) = database_recorded_time.first() {
                    first.clone()
                  } else {
                    None
                  };
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
      let (sender, emitter) = mpsc::channel(0);
      let (callback, messages) = mpsc::channel(0);
      if let Err(msg) = s.open_websocket(&sender, emitter, &callback, &identifier) {
        error!("{}", msg);
      } else {
        if let Err(msg) = s.open_channel(&identifier, "browser:all") {
          error!("{}", msg);
        } else {
          let runner =
            messages
            .for_each(|message| {
              debug!("{:?}", message);

              match message.topic.as_ref() {
                "browser:all" => {
                  debug!("browser:all: {:?}", message);
                  let files = browser::process(message, &root_path_browsing);
                  if let Err(msg) = s.send("response", files.into()) {
                    error!("{:?}", msg);
                  }
                }
                "phoenix" => {
                  if message.event == Event::Defined(PhoenixEvent::Close) {
                    return Err(());
                  }
                }
                _ => { debug!("{:?}", message); }
              }

              Ok(())
            });

          let mut core = Core::new().unwrap();
          if let Err(msg) = core.run(runner) {
            error!("{:?}", msg);
          }
        }
      }
    }

    thread::sleep(time::Duration::from_millis(1000));
    warn!("retry to connect ...");
  }
}
