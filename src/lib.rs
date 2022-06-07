mod http;
mod kv;
mod tcp;

pub use http::http_server;
pub use kv::{DynKV, MemKV};
pub use tcp::{command_parse, tcp_server, Server};

#[derive(Debug, PartialEq)]
pub enum Command {
    Get(Vec<String>),
    Set(String, String),
}

impl Command {
    #[cfg(test)]
    fn to_string_command(&self) -> String {
        match self {
            Command::Get(keys) => format!("get {}\r\n", keys.join(" ")),
            Command::Set(key, value) => format!("set {key} 1 2 {} {value}\r\n", value.len()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_command_to_string_command() {
        assert_eq!("get a\r\n", Command::Get(vec!["a".to_string()]).to_string_command());
        assert_eq!("set key 1 2 5 value\r\n", Command::Set("key".to_string(), "value".to_string()).to_string_command());
    }
}
