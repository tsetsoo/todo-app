use wasm_bindgen::prelude::*;

pub fn is_supported() -> bool {
    let window = web_sys::window().unwrap();
    js_sys::Reflect::get(&window, &"__speechSupported".into())
        .map(|v| v.as_bool().unwrap_or(false))
        .unwrap_or(false)
}

pub fn start_recognition<F>(on_done: F)
where
    F: FnOnce(String) + 'static,
{
    let on_done = std::cell::RefCell::new(Some(on_done));
    let cb = Closure::<dyn FnMut(JsValue)>::new(move |val: JsValue| {
        let text = val.as_string().unwrap_or_default();
        if let Some(f) = on_done.borrow_mut().take() {
            f(text);
        }
    });

    let window = web_sys::window().unwrap();
    let start_fn: js_sys::Function = js_sys::Reflect::get(&window, &"__startSpeechRecognition".into())
        .unwrap()
        .unchecked_into();

    start_fn.call1(&JsValue::NULL, cb.as_ref()).unwrap();

    cb.forget();
}
