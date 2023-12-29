use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn js_log(s: &str);
    #[wasm_bindgen(js_namespace = console, js_name = warn)]
    pub fn js_warn(s: &str);
    
    pub fn setTimeout(f: Function, t: u32);
}

macro_rules! console_log {
    ($($t:tt)*) => (js_log(&format_args!($($t)*).to_string()))
}
pub(crate) use console_log;

macro_rules! console_warn {
    ($($t:tt)*) => (js_warn(&format_args!($($t)*).to_string()))
}
pub(crate) use console_warn;
