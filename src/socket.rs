
use config;
use phoenix::{Message, Phoenix};
use reqwest;
use std::collections::HashMap;

//#[derive(Debug)]
pub struct Socket {
  pub hostname: String,
  pub password: String,
  pub port: String,
  pub secure: bool,
  pub token: Option<String>,
  pub username: String,
  pub websocket: Option<Phoenix>
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

#[derive(Debug, Deserialize)]
struct SessionReponse {
  access_token: String,
}

impl Socket {
  pub fn new_from_config() -> Self {
    let hostname = config::get_backend_hostname();
    let port = config::get_backend_port();
    let username = config::get_backend_username();
    let password = config::get_backend_password();
    let secure =
      match config::get_backend_secure().as_str() {
        "true" |
        "True" |
        "TRUE" |
        "1" => true,
        _ => false,
      };

    Socket {
      hostname,
      password,
      port,
      secure,
      token: None,
      username,
      websocket: None,
    }
  }

  pub fn generate_token(&mut self) -> Result<(), String> {
    let mut url =
      if self.secure {
        "https://".to_owned()
      } else {
        "http://".to_owned()
      };

    url += &self.hostname;
    url += ":";
    url += &self.port;
    url += "/api/sessions";
    debug!("get token with URL: {}", url);

    let body = SessionBody {
      session: Session {
        email: self.username.clone(),
        password: self.password.clone(),
      }
    };

    let client = reqwest::Client::new();
    let mut response = client.post(&url)
      .json(&body)
      .send().map_err(|e| e.to_string())?;

    if !response.status().is_success() {
      if response.status().is_server_error() {
        return Err("serveur error".to_owned());
      } else {
        return Err(format!("Something else happened. Status: {:?}", response.status()));
      }
    }

    let content: SessionReponse = response.json().unwrap();
    self.token = Some(content.access_token);
    Ok(())
  }

  pub fn open_websocket(&mut self) -> Result<(), String> {
    let mut url =
      if self.secure {
        "wss://".to_owned()
      } else {
        "ws://".to_owned()
      };

    url += &self.hostname;
    url += ":";
    url += &self.port;
    url += "/socket";

    if self.token == None {
      return Err("missing authentification token".to_owned());
    }

    let mut params = HashMap::new();
    let token = self.token.as_ref().unwrap();
    params.insert("userToken", token.as_str());

    debug!("connect to websocket: {}", url);
    self.websocket = Some(Phoenix::new_with_parameters(&url, &params));

    Ok(())
  }

  pub fn open_channel(&mut self, channel_name: &str) -> Result<(), String> {
    if let Some(ref mut phoenix) = self.websocket {
      let mutex_chan = phoenix.channel(channel_name).clone();
      let mut device_chan = mutex_chan.lock().unwrap();
      let payload = json!({
        "identifier": "marco-dev"
      });

      device_chan.join_with_message(payload);
      return Ok(())
    }
    Err("missing websocket connection".to_owned())
  }

  pub fn next_message(&mut self) -> Result<Message, String> {
    if let Some(ref mut phoenix) = self.websocket {
      phoenix.out.recv().map_err(|e| e.to_string())
    } else {
      Err("missing websocket connection".to_owned())
    }
  }
}
