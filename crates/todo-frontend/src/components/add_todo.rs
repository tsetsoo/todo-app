use leptos::*;
use todo_shared::{CreateTodoRequest, Section};
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::speech;

#[component]
pub fn AddTodo(
    active_section: ReadSignal<Section>,
    on_added: WriteSignal<usize>,
    refresh: ReadSignal<usize>,
) -> impl IntoView {
    let (title, set_title) = create_signal(String::new());
    let (submitting, set_submitting) = create_signal(false);
    let (listening, set_listening) = create_signal(false);

    let has_speech = speech::is_supported();

    let submit = move |_| {
        let t = title.get_untracked().trim().to_string();
        if t.is_empty() || submitting.get_untracked() {
            return;
        }
        set_submitting.set(true);
        let section = active_section.get_untracked();
        spawn_local(async move {
            let req = CreateTodoRequest { section, title: t };
            let _ = api::create_todo(&req).await;
            set_title.set(String::new());
            set_submitting.set(false);
            on_added.set(refresh.get_untracked() + 1);
        });
    };

    let on_keydown = move |ev: web_sys::KeyboardEvent| {
        if ev.key() == "Enter" {
            let t = title.get_untracked().trim().to_string();
            if t.is_empty() || submitting.get_untracked() {
                return;
            }
            set_submitting.set(true);
            let section = active_section.get_untracked();
            spawn_local(async move {
                let req = CreateTodoRequest { section, title: t };
                let _ = api::create_todo(&req).await;
                set_title.set(String::new());
                set_submitting.set(false);
                on_added.set(refresh.get_untracked() + 1);
            });
        }
    };

    let on_mic = move |_| {
        if listening.get_untracked() {
            return;
        }
        set_listening.set(true);
        speech::start_recognition(move |transcript| {
            set_listening.set(false);
            if !transcript.is_empty() {
                let current = title.get_untracked();
                if current.is_empty() {
                    set_title.set(transcript);
                } else {
                    set_title.set(format!("{} {}", current, transcript));
                }
            }
        });
    };

    view! {
        <div class="add-todo">
            <input
                type="text"
                placeholder="Add a todo..."
                prop:value=title
                on:input=move |ev| set_title.set(event_target_value(&ev))
                on:keydown=on_keydown
                prop:disabled=submitting
            />
            {if has_speech {
                Some(view! {
                    <button
                        class="mic-btn"
                        class:listening=listening
                        on:click=on_mic
                        prop:disabled=listening
                        title="Voice input"
                    >
                        {move || if listening.get() { "\u{25CF}" } else { "\u{1F3A4}" }}
                    </button>
                })
            } else {
                None
            }}
            <button
                on:click=submit
                prop:disabled=move || submitting.get() || title.get().trim().is_empty()
            >
                "Add"
            </button>
        </div>
    }
}
