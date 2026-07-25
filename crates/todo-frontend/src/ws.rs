use leptos::{ReadSignal, WriteSignal, SignalGetUntracked, SignalSet};
use wasm_bindgen::prelude::*;
use web_sys::WebSocket;

fn ws_url() -> String {
    let window = web_sys::window().expect("no window");
    let location = window.location();
    let protocol = location.protocol().unwrap_or_default();
    let host = location.host().unwrap_or_default();
    let ws_proto = if protocol == "https:" { "wss:" } else { "ws:" };
    format!("{ws_proto}//{host}/api/ws")
}

fn connect_inner(set_refresh: WriteSignal<usize>, refresh: ReadSignal<usize>) {
    let url = ws_url();
    let ws = match WebSocket::new(&url) {
        Ok(ws) => ws,
        Err(e) => {
            log::error!("WebSocket connect failed: {e:?}");
            schedule_reconnect(set_refresh, refresh);
            return;
        }
    };

    let on_message = Closure::<dyn Fn(web_sys::MessageEvent)>::new(move |_: web_sys::MessageEvent| {
        set_refresh.set(refresh.get_untracked() + 1);
    });
    ws.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
    on_message.forget();

    let sr = set_refresh;
    let r = refresh;
    let on_close = Closure::<dyn Fn(web_sys::CloseEvent)>::new(move |_: web_sys::CloseEvent| {
        log::warn!("WebSocket closed, reconnecting...");
        schedule_reconnect(sr, r);
    });
    ws.set_onclose(Some(on_close.as_ref().unchecked_ref()));
    on_close.forget();

    let sr2 = set_refresh;
    let r2 = refresh;
    let on_error = Closure::<dyn Fn(web_sys::ErrorEvent)>::new(move |_: web_sys::ErrorEvent| {
        log::error!("WebSocket error, will reconnect on close");
        // The close event fires after error, which triggers reconnect.
        // But if it doesn't, schedule one here as a safety net.
        let _ = (sr2, r2);
    });
    ws.set_onerror(Some(on_error.as_ref().unchecked_ref()));
    on_error.forget();
}

fn schedule_reconnect(set_refresh: WriteSignal<usize>, refresh: ReadSignal<usize>) {
    let cb = Closure::<dyn Fn()>::new(move || {
        connect_inner(set_refresh, refresh);
    });
    let window = web_sys::window().expect("no window");
    let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
        cb.as_ref().unchecked_ref(),
        2000,
    );
    cb.forget();
}

pub fn connect(set_refresh: WriteSignal<usize>, refresh: ReadSignal<usize>) {
    connect_inner(set_refresh, refresh);
}
