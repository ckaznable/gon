use anyhow::Result;
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::notification::Notification;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Method {
    Done,
    Ping,
    NewNotification,
    GetHost,
    HostChanged,
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
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ResponseStatus {
    Success,
    Faild,
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

    pub fn faild() -> Self {
        Self {
            status: ResponseStatus::Faild,
            result: None,
        }
    }
}

pub fn handle_message(msg: Message) -> Response {
    let res: Result<Response> = {
        match msg.method {
            Method::Ping => {
                Ok(Response::success(Payload::Text("Pong".to_string())))
            },
            Method::NewNotification => todo!(),
            Method::GetHost => todo!(),
            _ => Ok(Response::empty()),
        }
    };

    res.unwrap_or(Response::faild())
}

