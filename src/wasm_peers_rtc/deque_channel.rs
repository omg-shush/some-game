use std::{collections::VecDeque, rc::Rc, cell::RefCell};

use js_sys::{Promise, Function};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;

#[derive(Clone)]
pub struct JsSender<T> {
    channel: Rc<RefCell<JsDequeChannel<T>>>
}

impl<T> JsSender<T> {
    pub fn send(&self, data: T) -> Result<(), JsValue> {
        let mut channel = self.channel.borrow_mut();
        channel.buffer.push_back(data);
        channel.resolve.call0(&JsValue::UNDEFINED)?;
        (channel.promise, channel.resolve) = JsDequeChannel::<T>::make_promise();
        Ok(())
    }
}

#[derive(Clone)]
pub struct JsReceiver<T> {
    channel: Rc<RefCell<JsDequeChannel<T>>>
}

impl<T> JsReceiver<T> {
    pub async fn recv(&self) -> Result<T, JsValue> {
        if self.channel.borrow().buffer.is_empty() {
            // Sleep on promise
            let promise = self.channel.borrow().promise.clone();
            JsFuture::from(promise).await?;
        }
        let mut channel = self.channel.borrow_mut();
        assert!(!channel.buffer.is_empty());
        Ok(channel.buffer.pop_front().unwrap())
    }

    pub fn drain(&self) -> Vec<T> {
        self.channel.borrow_mut().buffer.drain(..).collect::<Vec<_>>()
    }
}

pub struct JsDequeChannel<T> {
    buffer: VecDeque<T>,
    promise: Promise,
    resolve: Function
}

impl<T> JsDequeChannel<T> {
    pub fn channel() -> (JsSender<T>, JsReceiver<T>) {
        let (promise, resolve) = Self::make_promise();
        let channel = Rc::new(RefCell::new(JsDequeChannel {
            buffer: VecDeque::new(),
            promise,
            resolve
        }));
        (JsSender { channel: channel.clone() }, JsReceiver { channel })
    }

    fn make_promise() -> (Promise, Function) {
        let resolve = Rc::new(RefCell::new(Function::new_no_args("")));
        let resolve_clone = resolve.clone();
        let promise = Promise::new(&mut |resolve, _| {
            *resolve_clone.borrow_mut() = resolve;
        });
        let resolve = resolve.take();
        (promise, resolve)
    }
}
