use std::{collections::HashMap, net::{IpAddr, SocketAddr}};

use serde::{Deserialize, Serialize};

use crate::notification::Notification;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Method {
    Done,
    Ping,
    NewNotification,
    GetHost,
    ImHost,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    pub method: Method,
    pub payload: Payload,
}

impl Message {
    pub fn is_done(&self) -> bool {
        self.method == Method::Done
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Payload {
    Text(String),
    Number(u64),
    List(Vec<String>),
    Dictionary(HashMap<String, String>),
    Raw(Vec<u8>),
    Notification(Notification),
    Address(u8, u8, u8, u8, u16),
    Empty,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatus {
    Success,
    Faild,
    HostChanged,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    pub status: ResponseStatus,
    pub result: Option<Payload>,
}

impl Response {
    pub fn success(payload: Payload) -> Self {
        Self {
            status: ResponseStatus::Success,
            result: Some(payload),
        }
    }

    pub fn empty() -> Self {
        Self {
            status: ResponseStatus::Success,
            result: None,
        }
    }

    pub fn failed() -> Self {
        Self {
            status: ResponseStatus::Faild,
            result: None,
        }
    }

    pub fn host_changed(socket: SocketAddr) -> Self {
        let IpAddr::V4(addr) = socket.ip() else {
            return Self::failed();
        };

        let ip = addr.octets();
        let port = socket.port();

        Self {
            status: ResponseStatus::HostChanged,
            result: Some(Payload::Address(ip[0], ip[1], ip[2], ip[3], port)),
        }
    }

    pub fn is_host_changed(&self) -> bool {
        self.status == ResponseStatus::HostChanged
    }

    pub fn is_failed(&self) -> bool {
        self.status == ResponseStatus::Faild
    }
}
