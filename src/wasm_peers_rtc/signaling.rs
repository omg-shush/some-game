use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use wasm_bindgen::JsValue;

use super::{callback_channel::SendRecvCallbackChannel, deque_channel::{JsSender, JsDequeChannel, JsReceiver}};

pub type ConnectionId = String;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "action")]
pub enum SignalingMessage {
    #[serde(rename = "list")]
    List {
        servers: HashMap<ConnectionId, ServerEntry>
    },
    #[serde(rename = "register")]
    Register {
        game: String,
        name: String
    },
    #[serde(rename = "relay")]
    Relay {
        #[serde(skip_serializing)]
        src: ConnectionId,
        dst: ConnectionId,
        data: RelayMessage
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerEntry {
    pub name: String,
    pub game: String
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum RelayMessage {
    Offer(String),
    Answer(String),
    IceCandidate {
        candidate: String,
        sdp_mid: Option<String>,
        sdp_m_line_index: Option<u16>
    }
}

pub struct SignalingDemux {
    signaling_server: SendRecvCallbackChannel,
    clients: HashMap<ConnectionId, JsSender<RelayMessage>>
}

pub enum SignalingDemuxRecv {
    System(SignalingMessage),
    Relay(ConnectionId, SignalingClientConnection)
}

impl SignalingDemux {
    pub fn new(signaling_server: SendRecvCallbackChannel) -> SignalingDemux {
        SignalingDemux { signaling_server, clients: HashMap::new() }
    }
    pub fn _send(&mut self, msg: SignalingMessage) -> Result<(), JsValue> {
        self.signaling_server.send(msg)
    }

    pub async fn recv(&mut self) -> Result<SignalingDemuxRecv, JsValue> {
        loop {
            let msg: SignalingMessage = self.signaling_server.recv().await?;
            match msg {
                SignalingMessage::Relay { src, dst: _dst, data } if self.clients.contains_key(&src) => {
                    // Existing connection
                    self.clients.get_mut(&src).unwrap().send(data)?;
                }
                SignalingMessage::Relay { src, dst: _dst, data } => {
                    // New connection
                    let (sender, receiver) = JsDequeChannel::<RelayMessage>::channel();
                    sender.send(data)?;
                    self.clients.insert(src.to_owned(), sender);
                    return Ok(SignalingDemuxRecv::Relay(src.clone(), SignalingClientConnection {
                        connection_id: src,
                        signaling_server: self.signaling_server.clone(),
                        receiver
                    }));
                }
                _ => {
                    // System message
                    return Ok(SignalingDemuxRecv::System(msg))
                }
            }
        }
        
    }
}

pub struct SignalingClientConnection {
    connection_id: String,
    signaling_server: SendRecvCallbackChannel, // send-only
    receiver: JsReceiver<RelayMessage>
}

impl SignalingClientConnection {
    pub fn send(&mut self, msg: RelayMessage) -> Result<(), JsValue> {
        self.signaling_server.send(SignalingMessage::Relay {
            src: "".to_owned(),
            dst: self.connection_id.to_owned(),
            data: msg
        })
    }

    pub async fn recv(&mut self) -> Result<RelayMessage, JsValue> {
        self.receiver.recv().await
    }

    pub fn clone_sender(&self) -> SignalingClientSender {
        SignalingClientSender {
            connection_id: self.connection_id.clone(),
            signaling_server: self.signaling_server.clone()
        }
    }
}

pub struct SignalingClientSender {
    connection_id: String,
    signaling_server: SendRecvCallbackChannel, // send-only
}

impl SignalingClientSender {
    pub fn send(&mut self, msg: RelayMessage) -> Result<(), JsValue> {
        self.signaling_server.send(SignalingMessage::Relay {
            src: "".to_owned(),
            dst: self.connection_id.to_owned(),
            data: msg
        })
    }
}
