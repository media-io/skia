use std::env;

macro_rules! get_env_value {
  ($key:expr, $arg:expr, $default:expr) => {{
    // let mut item = None; $default.to_string();
    for (key, value) in env::vars() {
      match key.as_ref() {
        $key => {
          // item = Some(value);
          return value
        }
        _ => {}
      }
    }
    if let Some(value) = $arg {
      return value.to_string()
    }
    $default.to_string()
  }};
}

pub fn get_identifier(arg: Option<&str>) -> String {
  get_env_value!("IDENTIFIER", arg, "identifier_not_set")
}

pub fn get_backend_hostname(arg: Option<&str>) -> String {
  get_env_value!("BACKEND_HOSTNAME", arg, "127.0.0.1")
}

pub fn get_backend_port(arg: Option<&str>) -> String {
  get_env_value!("BACKEND_PORT", arg, "4000")
}

pub fn get_backend_username(arg: Option<&str>) -> String {
  get_env_value!("BACKEND_USERNAME", arg, "admin@media-io.com")
}

pub fn get_backend_password(arg: Option<&str>) -> String {
  get_env_value!("BACKEND_PASSWORD", arg, "admin123")
}

pub fn get_backend_secure(arg: Option<&str>) -> String {
  get_env_value!("BACKEND_SECURE", arg, "false")
}

pub fn get_root_path_browsing(arg: Option<&str>) -> String {
  get_env_value!("ROOT_PATH_BROWSING", arg, "/tmp/")
}

pub fn get_mounted_name_path_browsing(arg: Option<&str>) -> String {
  get_env_value!("MOUNTED_NAME_PATH_BROWSING", arg, "H:/NTS2018 mp4s/")
}

pub fn get_adobe_media_encoder_log_filename(arg: Option<&str>) -> String {
  get_env_value!(
    "ADOBE_MEDIA_ENCODER_LOG_FILENAME",
    arg,
    "tests/AMEEncodingLog.txt"
  )
}
