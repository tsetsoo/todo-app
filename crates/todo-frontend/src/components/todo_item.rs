use leptos::*;
use todo_shared::Todo;
use wasm_bindgen_futures::spawn_local;

use crate::api;

#[component]
pub fn TodoItem(
    todo: Todo,
    on_changed: WriteSignal<usize>,
    refresh: ReadSignal<usize>,
    #[prop(optional)] show_section: bool,
) -> impl IntoView {
    let id_toggle = todo.id.clone();
    let id_delete = todo.id.clone();

    let completed = todo.completed;
    let title = todo.title.clone();
    let importance = todo.importance;
    let due_date = todo.due_date.clone();
    let section = todo.section;

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

    let class = {
        let mut c = format!("todo-item priority-{}", importance.as_str());
        if completed { c.push_str(" completed"); }
        c
    };
    let priority_badge_class = format!("priority-badge {}", importance.as_str());

    view! {
        <li class=class>
            <input type="checkbox" prop:checked=completed on:change=toggle />
            <span class=priority_badge_class>{importance.label()}</span>
            {show_section.then(|| view! {
                <span class="todo-section-badge">{section.as_str()}</span>
            })}
            <span class="title">{title}</span>
            {due_date.map(|d| view! {
                <span class="due-date">{d}</span>
            })}
            <button class="delete-btn" on:click=delete title="Delete">
                "\u{00d7}"
            </button>
        </li>
    }
}
