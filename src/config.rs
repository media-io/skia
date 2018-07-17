
use std::env;

macro_rules! get_env_value {
  ($key:expr, $default:expr) => (
  {
    let mut item = $default.to_string();
    for (key, value) in env::vars() {
      match key.as_ref() {
        $key => {
          item = value;
        }
        _ => {},
      }
    }
    item
  })
}

pub fn get_backend_hostname() -> String {
  get_env_value!("BACKEND_HOSTNAME", "127.0.0.1")
}

pub fn get_backend_port() -> String {
  get_env_value!("BACKEND_PORT", "4000")
}

pub fn get_backend_username() -> String {
  get_env_value!("BACKEND_USERNAME", "admin@media-io.com")
}

pub fn get_backend_password() -> String {
  get_env_value!("BACKEND_PASSWORD", "admin123")
}

pub fn get_backend_secure() -> String {
  get_env_value!("BACKEND_SECURE", "false")
}
