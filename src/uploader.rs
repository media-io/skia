
use config::get_data_size;
use phoenix::event::Event;
use phoenix::message::Message;
use serde_json;
use serde_json::Value;
use std::cmp::min;
use std::fs;
use std::fs::File;
use std::io::prelude::*;

use websocket::futures::future::Future;
use websocket::futures::sink::Sink;
use websocket::futures::stream::Stream;
use websocket::futures::sync::mpsc;
use std::thread;
use tokio_core::reactor::Core;
use websocket::result::WebSocketError;
use websocket::{ClientBuilder, OwnedMessage};

#[derive(Debug)]
struct UploadOrder {
  job_id: u64,
  path: String,
  destination: String,
}

impl UploadOrder {
  fn from(content: Value) -> Option<Self> {
    let mut maybe_job_id = None;
    let mut maybe_path = None;
    let mut maybe_destination = None;

    if let Value::Object(map) = content {
      if let Some(value) = map.get("job_id") {
        maybe_job_id = Some(value.clone())
      }

      if let Some(parameters) = map.get("parameters") {
        if let &Value::Object(ref params) = parameters {
          if let Some(source) = params.get("source") {
            if let &Value::Object(ref s) = source {
              if let Some(path) = s.get("path") {
                if let &Value::String(ref string_path) = path {
                  maybe_path = Some(string_path.to_owned())
                }
              }
            }
          }
          if let Some(source) = params.get("destination") {
            if let &Value::Object(ref s) = source {
              if let Some(path) = s.get("path") {
                if let &Value::String(ref string_path) = path {
                  maybe_destination = Some(string_path.to_owned())
                }
              }
            }
          }
        }
      }
    }

    match (maybe_job_id, maybe_path, maybe_destination) {
      (Some(Value::Number(ref j)), Some(ref p), Some(ref dst)) => Some(UploadOrder{
        job_id: j.as_u64().unwrap(),
        path: p.to_owned(),
        destination: dst.to_owned(),
      }),
      (_, _, _) => None
    }
  }
}

pub fn process(upload_ws: &str, message: Message, root_path: &str) {
  if let Event::Custom(ref event) = message.event {
    match event.as_str() {
      "start" => {
        if let Some(order) = UploadOrder::from(message.payload) {
          let full_path = format!("{}/{}", root_path, order.path);
          let ws = upload_ws.to_string();
          let t = thread::spawn(move || {
            let _ = upload_file(ws.as_str(), &full_path, &order.destination);
          });

          let _ = t.join();
        }
      }
      _ => {}
    }
  }
}

#[derive(Debug, Serialize)]
struct StartMessage {
  filename: String,
  size: u64,
}

pub fn upload_file(upload_ws: &str, filename: &str, dst_filename: &str) -> Result<(), String> {
  info!("Start to upload {:?}", filename);

  let mut core = Core::new().unwrap();
  let (usr_msg, stdin_ch) = mpsc::channel(0);

  let metadata = fs::metadata(filename.clone()).unwrap();
  let file_size = metadata.len();

  let mut file = File::open(filename).unwrap();

  let mut sended_data = 0;

  let start_message = StartMessage {
    filename: dst_filename.to_string(),
    size: file_size
  };

  thread::spawn(move || {
    let mut stdin_sink = usr_msg.wait();

    let str_message = serde_json::to_string(&start_message).unwrap();
    let msg = OwnedMessage::Text(str_message);

    stdin_sink
      .send(msg)
      .expect("Sending message across stdin channel.");

    let packet_size_str = get_data_size(None);
    let packet_size = packet_size_str.parse::<u64>().unwrap();

    loop {
      let data_size = min(packet_size, file_size - sended_data);
      if data_size == 0 {
        break;
      }

      let mut contents = vec![0u8; data_size as usize];
      if let Ok(()) = file.read_exact(&mut contents) {
        sended_data += data_size;

        let msg = OwnedMessage::Binary(contents);

        stdin_sink
          .send(msg)
          .expect("Sending message across stdin channel.");
      }
    }


    let msg = OwnedMessage::Close(None);
    stdin_sink
      .send(msg)
      .expect("Sending message across stdin channel.");

    info!("Sended {}/{} bytes", sended_data, file_size);
  });

  let runner = ClientBuilder::new(upload_ws)
    .unwrap()
    .add_protocol("rust-websocket")
    .async_connect(None, &core.handle())
    .and_then(|(duplex, _)| {
      let (sink, stream) = duplex.split();
      stream
        .filter_map(|message| {
          debug!("Received Message: {:?}", message);
          match message {
            OwnedMessage::Close(e) => Some(OwnedMessage::Close(e)),
            OwnedMessage::Ping(d) => Some(OwnedMessage::Pong(d)),
            _ => None,
          }
        }).select(stdin_ch.map_err(|_| WebSocketError::NoDataAvailable))
        .forward(sink)
    });
  core.run(runner).map_err(|e| e.to_string())?;
  Ok(())
}
