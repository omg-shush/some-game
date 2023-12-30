use std::{rc::Rc, cell::RefCell, collections::VecDeque, error::Error};

use js_sys::{Function, Promise, JSON};
use serde::{Serialize, de::DeserializeOwned};
use wasm_bindgen_futures::{JsFuture, spawn_local, future_to_promise};
use web_sys::{WebSocket, RtcDataChannel, MessageEvent};
use wasm_bindgen::{prelude::wasm_bindgen, JsValue, closure::Closure, JsCast};

use super::deque_channel::{JsDequeChannel, JsSender, JsReceiver};

macro_rules! console_warn {
    ($($t:tt)*) => (warn(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

pub trait CallbackChannel {
    fn clone(&self) -> Box<dyn CallbackChannel>;
    fn set_onopen(&self, value: Option<&Function>);
    fn set_onmessage(&self, value: Option<&Function>);
    fn set_onclose(&self, value: Option<&Function>);
    fn set_onerror(&self, value: Option<&Function>);
    fn send_with_str(&self, data: &str) -> Result<(), JsValue>;
}

pub struct SendRecvCallbackChannel {
    channel: Box<dyn CallbackChannel>,
    queue_sender: JsSender<JsValue>, // Enqueue newly received values
    queue_receiver: JsReceiver<JsValue>
}

impl Clone for SendRecvCallbackChannel {
    fn clone(&self) -> Self {
        Self { channel: self.channel.clone(), queue_sender: self.queue_sender.clone(), queue_receiver: self.queue_receiver.clone() }
    }
}

impl SendRecvCallbackChannel {
    pub async fn new(channel: Box<dyn CallbackChannel>) -> Result<SendRecvCallbackChannel, JsValue> {
        let (queue_sender, queue_receiver) = JsDequeChannel::channel();
        let ws = SendRecvCallbackChannel {
            channel,
            queue_sender,
            queue_receiver
        };
        let init_promise = Promise::new(&mut |resolve, _reject| {
            let on_open = Closure::<dyn FnMut()>::new(move || {
                resolve.call0(&JsValue::UNDEFINED).unwrap();
            });
            ws.channel.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            on_open.forget();
        
            let sender = ws.queue_sender.clone();
            let on_error = Closure::<dyn FnMut()>::new(move || {
                console_warn!("Error in channel!");
                sender.close().unwrap();
            });
            ws.channel.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            on_error.forget();

            let sender = ws.queue_sender.clone();
            let on_close = Closure::<dyn FnMut()>::new(move || {
                console_warn!("Closing channel.");
                sender.close().unwrap();
            });
            ws.channel.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            on_close.forget();

            let sender = ws.queue_sender.clone();
            let on_message = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
                let value = JSON::parse(&e.data().as_string().unwrap()).unwrap();
                sender.send(value).unwrap();
            });
            ws.channel.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();
        });
        JsFuture::from(init_promise).await?;
        Ok(ws)
    }

    pub fn send(&mut self, msg: impl Serialize + Sized) -> Result<(), JsValue> {
        let value = serde_wasm_bindgen::to_value(&msg)?;
        self.channel.send_with_str(&JSON::stringify(&value)?.as_string().unwrap())?;
        Ok(())
    }

    pub async fn recv<T: DeserializeOwned>(&mut self) -> Result<T, JsValue> {
        self.queue_receiver
            .recv()
            .await
            .map(|value| {
                serde_wasm_bindgen::from_value(value.clone()).map_err(|e| {
                    let str = format!("Failed to deserialize value `{:?}`: {}", value, e);
                    JsValue::from_str(&str)
                })
            })?
    }

    pub fn drain<T: DeserializeOwned>(&mut self) -> Result<Vec<T>, Box<dyn Error>> {
        let mut queue = self.queue_receiver.drain();
        let len = queue.len();
        queue
            .into_iter()
            .map(|m| {
                serde_wasm_bindgen::from_value(m).map_err(|e| Box::new(e) as Box<dyn Error>)
            })
            .collect()
    }

    pub fn is_closed(&self) -> bool {
        self.queue_receiver.is_closed()
    }
}

impl CallbackChannel for WebSocket {
    fn clone(&self) -> Box<dyn CallbackChannel> {
        Box::new(Clone::clone(self))
    }

    fn set_onopen(&self, value: Option<&Function>) {
        self.set_onopen(value);
    }

    fn set_onmessage(&self, value: Option<&Function>) {
        self.set_onmessage(value);
    }

    fn set_onclose(&self, value: Option<&Function>) {
        self.set_onclose(value);
    }

    fn set_onerror(&self, value: Option<&Function>) {
        self.set_onerror(value);
    }

    fn send_with_str(&self, data: &str) -> Result<(), JsValue> {
        self.send_with_str(data)
    }
}

impl CallbackChannel for RtcDataChannel {
    fn clone(&self) -> Box<dyn CallbackChannel> {
        Box::new(Clone::clone(self))
    }

    fn set_onopen(&self, value: Option<&Function>) {
        self.set_onopen(value);
    }

    fn set_onmessage(&self, value: Option<&Function>) {
        self.set_onmessage(value);
    }

    fn set_onclose(&self, value: Option<&Function>) {
        self.set_onclose(value);
    }

    fn set_onerror(&self, value: Option<&Function>) {
        self.set_onerror(value);
    }

    fn send_with_str(&self, data: &str) -> Result<(), JsValue> {
        self.send_with_str(data)
    }
}
