use leptos::*;
use todo_shared::{Importance, Section};

use crate::api;
use super::todo_item::TodoItem;

#[component]
pub fn ArchiveView(
    refresh: ReadSignal<usize>,
    set_refresh: WriteSignal<usize>,
) -> impl IntoView {
    let (section_filter, set_section_filter) = create_signal(String::new());
    let (importance_filter, set_importance_filter) = create_signal(String::new());

    let todos_resource = create_resource(
        move || (refresh.get(), section_filter.get()),
        move |(_, sec)| async move {
            let section = if sec.is_empty() { None } else { Some(sec) };
            api::fetch_todos(section.as_deref(), Some("importance_date"), Some("archived")).await.unwrap_or_default()
        },
    );

    let todos = Signal::derive(move || todos_resource.get().unwrap_or_default());

    let filtered_todos = Signal::derive(move || {
        let imp = importance_filter.get();
        let items = todos.get();
        if imp.is_empty() {
            items
        } else {
            items.into_iter().filter(|t| t.importance.as_str() == imp).collect()
        }
    });

    let count = Signal::derive(move || filtered_todos.get().len());

    view! {
        <div class="archive-view">
            <h2>"Archive"</h2>
            <div class="archive-filters">
                <select on:change=move |ev| set_section_filter.set(event_target_value(&ev))>
                    <option value="">"All sections"</option>
                    {Section::all().iter().map(|s| {
                        let val = s.as_str().to_string();
                        let label = s.as_str().to_string();
                        view! { <option value=val>{label}</option> }
                    }).collect_view()}
                </select>
                <select on:change=move |ev| set_importance_filter.set(event_target_value(&ev))>
                    <option value="">"All importance"</option>
                    {Importance::all().iter().map(|imp| {
                        let val = imp.as_str().to_string();
                        let label = imp.label().to_string();
                        view! { <option value=val>{label}</option> }
                    }).collect_view()}
                </select>
                <span class="archive-count">{move || format!("{} archived", count.get())}</span>
            </div>
            <ul class="todo-list">
                {move || {
                    let items = filtered_todos.get();
                    if items.is_empty() {
                        view! { <p class="empty-state">"No archived todos"</p> }.into_view()
                    } else {
                        items.into_iter().map(|todo| {
                            view! {
                                <TodoItem
                                    todo=todo
                                    on_changed=set_refresh
                                    refresh=refresh
                                    show_section=true
                                />
                            }
                        }).collect_view()
                    }
                }}
            </ul>
        </div>
    }
}
