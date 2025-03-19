use serde::{Deserialize, Serialize};

use super::node::Payload;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Method {
    Done,
    NewNotification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    method: Method,
    payload: Payload,
}


