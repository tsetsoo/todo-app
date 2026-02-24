use leptos::*;
use todo_shared::{Importance, Todo, UpdateTodoRequest};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::api;

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn TodoItem(
    todo: Todo,
    on_changed: WriteSignal<usize>,
    refresh: ReadSignal<usize>,
    #[prop(optional)] show_section: bool,
) -> impl IntoView {
    let id_toggle = todo.id.clone();
    let id_delete = todo.id.clone();
    let id_title = todo.id.clone();
    let id_importance = todo.id.clone();
    let id_date = todo.id.clone();

    let completed = todo.completed;
    let title = todo.title.clone();
    let importance = todo.importance;
    let due_date = todo.due_date.clone();
    let section = todo.section;

    let (editing_title, set_editing_title) = create_signal(false);
    let (editing_importance, set_editing_importance) = create_signal(false);
    let (editing_due_date, set_editing_due_date) = create_signal(false);

    // Node refs for auto-focus
    let title_input_ref = create_node_ref::<html::Input>();
    let importance_select_ref = create_node_ref::<html::Select>();
    let date_input_ref = create_node_ref::<html::Input>();

    // Auto-focus effects
    create_effect(move |_| {
        if editing_title.get() {
            request_animation_frame(move || {
                if let Some(el) = title_input_ref.get() {
                    let _ = el.focus();
                    let _ = el.select();
                }
            });
        }
    });

    create_effect(move |_| {
        if editing_importance.get() {
            request_animation_frame(move || {
                if let Some(el) = importance_select_ref.get() {
                    let _ = el.focus();
                }
            });
        }
    });

    create_effect(move |_| {
        if editing_due_date.get() {
            request_animation_frame(move || {
                if let Some(el) = date_input_ref.get() {
                    let _ = el.focus();
                }
            });
        }
    });

    let toggle = move |_| {
        let id = id_toggle.clone();
        spawn_local(async move {
            let _ = api::toggle_todo(&id).await;
            on_changed.set(refresh.get_untracked() + 1);
        });
    };

    let delete = move |_| {
        let id = id_delete.clone();
        spawn_local(async move {
            let _ = api::delete_todo(&id).await;
            on_changed.set(refresh.get_untracked() + 1);
        });
    };

    // -- Title editing --
    let original_title = title.clone();
    let title_display = title.clone();
    let start_editing_title = move |_| {
        set_editing_importance.set(false);
        set_editing_due_date.set(false);
        set_editing_title.set(true);
    };

    let on_title_keydown = {
        let id = id_title.clone();
        let orig = original_title.clone();
        move |ev: web_sys::KeyboardEvent| {
            let key = ev.key();
            if key == "Escape" {
                set_editing_title.set(false);
                return;
            }
            if key == "Enter" {
                ev.prevent_default();
                let target = ev.target().unwrap();
                let input = target.unchecked_ref::<web_sys::HtmlInputElement>();
                let new_val = input.value().trim().to_string();
                if new_val.is_empty() || new_val == orig {
                    set_editing_title.set(false);
                    return;
                }
                let id = id.clone();
                spawn_local(async move {
                    let req = UpdateTodoRequest {
                        title: Some(new_val),
                        ..Default::default()
                    };
                    let _ = api::update_todo(&id, &req).await;
                    on_changed.set(refresh.get_untracked() + 1);
                });
                set_editing_title.set(false);
            }
        }
    };

    let on_title_blur = {
        let id = id_title.clone();
        let orig = original_title.clone();
        move |ev: web_sys::FocusEvent| {
            let target = ev.target().unwrap();
            let input = target.unchecked_ref::<web_sys::HtmlInputElement>();
            let new_val = input.value().trim().to_string();
            if !new_val.is_empty() && new_val != orig {
                let id = id.clone();
                spawn_local(async move {
                    let req = UpdateTodoRequest {
                        title: Some(new_val),
                        ..Default::default()
                    };
                    let _ = api::update_todo(&id, &req).await;
                    on_changed.set(refresh.get_untracked() + 1);
                });
            }
            set_editing_title.set(false);
        }
    };

    // -- Importance editing --
    let start_editing_importance = move |_| {
        set_editing_title.set(false);
        set_editing_due_date.set(false);
        set_editing_importance.set(true);
    };

    let on_importance_change = {
        let id = id_importance.clone();
        move |ev: web_sys::Event| {
            let target = ev.target().unwrap();
            let select = target.unchecked_ref::<web_sys::HtmlSelectElement>();
            let val = select.value();
            if let Some(new_imp) = Importance::parse(&val) {
                if new_imp != importance {
                    let id = id.clone();
                    spawn_local(async move {
                        let req = UpdateTodoRequest {
                            importance: Some(new_imp),
                            ..Default::default()
                        };
                        let _ = api::update_todo(&id, &req).await;
                        on_changed.set(refresh.get_untracked() + 1);
                    });
                }
            }
            set_editing_importance.set(false);
        }
    };

    let on_importance_blur = move |_: web_sys::FocusEvent| {
        set_editing_importance.set(false);
    };

    // -- Due date editing --
    let original_due_date = due_date.clone();
    let due_date_display = due_date.clone();
    let start_editing_due_date = move |_| {
        set_editing_title.set(false);
        set_editing_importance.set(false);
        set_editing_due_date.set(true);
    };

    let on_date_change = {
        let id = id_date.clone();
        let orig = original_due_date.clone();
        move |ev: web_sys::Event| {
            let target = ev.target().unwrap();
            let input = target.unchecked_ref::<web_sys::HtmlInputElement>();
            let val = input.value();
            let new_date = if val.is_empty() { None } else { Some(val) };
            if new_date != orig {
                let id = id.clone();
                spawn_local(async move {
                    let req = UpdateTodoRequest {
                        due_date: Some(new_date),
                        ..Default::default()
                    };
                    let _ = api::update_todo(&id, &req).await;
                    on_changed.set(refresh.get_untracked() + 1);
                });
            }
            set_editing_due_date.set(false);
        }
    };

    let on_date_blur = move |_: web_sys::FocusEvent| {
        set_editing_due_date.set(false);
    };

    let class = {
        let mut c = format!("todo-item priority-{}", importance.as_str());
        if completed {
            c.push_str(" completed");
        }
        c
    };
    let priority_badge_class = format!("priority-badge {}", importance.as_str());
    let title_for_input = title.clone();

    view! {
        <li class=class>
            <input type="checkbox" prop:checked=completed on:change=toggle />

            // Priority badge: click to edit
            {move || {
                if editing_importance.get() {
                    let current = importance.as_str().to_string();
                    view! {
                        <select
                            class="inline-importance-select"
                            on:change=on_importance_change.clone()
                            on:blur=on_importance_blur
                            node_ref=importance_select_ref
                        >
                            {Importance::all().iter().map(|imp| {
                                let val = imp.as_str().to_string();
                                let label = imp.label().to_string();
                                let selected = val == current;
                                view! {
                                    <option value=val selected=selected>{label}</option>
                                }
                            }).collect::<Vec<_>>()}
                        </select>
                    }.into_view()
                } else {
                    let badge_class = priority_badge_class.clone();
                    let label = importance.label().to_string();
                    view! {
                        <span class=format!("{badge_class} editable") on:click=start_editing_importance>{label}</span>
                    }.into_view()
                }
            }}

            {show_section.then(|| view! {
                <span class="todo-section-badge">{section.as_str()}</span>
            })}

            // Title: click to edit
            {move || {
                if editing_title.get() {
                    let val = title_for_input.clone();
                    view! {
                        <input
                            type="text"
                            class="inline-title-input"
                            value=val
                            on:keydown=on_title_keydown.clone()
                            on:blur=on_title_blur.clone()
                            node_ref=title_input_ref
                        />
                    }.into_view()
                } else {
                    let t = title_display.clone();
                    view! {
                        <span class="title editable" on:click=start_editing_title>{t}</span>
                    }.into_view()
                }
            }}

            // Due date: click to edit
            {move || {
                if editing_due_date.get() {
                    let val = due_date.clone().unwrap_or_default();
                    view! {
                        <input
                            type="date"
                            class="inline-date-input"
                            value=val
                            on:change=on_date_change.clone()
                            on:blur=on_date_blur
                            node_ref=date_input_ref
                        />
                    }.into_view()
                } else if let Some(d) = due_date_display.clone() {
                    view! {
                        <span class="due-date editable" on:click=start_editing_due_date>{d}</span>
                    }.into_view()
                } else {
                    view! {
                        <span class="add-due-date editable" on:click=start_editing_due_date>"+ date"</span>
                    }.into_view()
                }
            }}

            <button class="delete-btn" on:click=delete title="Delete">
                "\u{00d7}"
            </button>
        </li>
    }
}
