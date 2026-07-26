use leptos::*;
use todo_shared::{DailyStatus, Todo};
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::components::todo_item::TodoItem;

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn DailyView(refresh: ReadSignal<usize>, set_refresh: WriteSignal<usize>) -> impl IntoView {
    let (status, set_status) = create_signal(None::<DailyStatus>);
    let (todos, set_todos) = create_signal(Vec::<Todo>::new());
    let (error, set_error) = create_signal(None::<String>);

    create_effect(move |_| {
        let _ = refresh.get();
        spawn_local(async move {
            let today = api::local_today();
            match api::fetch_daily_status(&today).await {
                Ok(s) => {
                    set_status.set(Some(s.clone()));
                    if s.has_today {
                        match api::fetch_daily(&today).await {
                            Ok(list) => set_todos.set(list),
                            Err(e) => set_error.set(Some(e)),
                        }
                    } else {
                        set_todos.set(Vec::new());
                    }
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    });

    let on_create = move |_| {
        let for_day = status
            .get_untracked()
            .map(|s| s.button)
            .unwrap_or_else(|| "today".into());
        spawn_local(async move {
            if let Err(e) = api::create_daily(&for_day).await {
                set_error.set(Some(e));
                return;
            }
            set_refresh.set(refresh.get_untracked() + 1);
        });
    };

    view! {
        <div class="daily-view">
            <div class="daily-header">
                <div>
                    <h2>"Daily"</h2>
                    {move || {
                        status.get().map(|s| {
                            let label = if s.has_today {
                                format!("Today · {}", s.local_today)
                            } else {
                                "No daily board for today yet".into()
                            };
                            view! { <p class="daily-date-label">{label}</p> }
                        })
                    }}
                </div>
                <button class="daily-create-btn" on:click=on_create>
                    {move || {
                        match status.get().map(|s| s.button) {
                            Some(b) if b == "tomorrow" => "Create todos for tomorrow".to_string(),
                            _ => "Create todos for today".to_string(),
                        }
                    }}
                </button>
            </div>

            {move || error.get().map(|e| view! { <p class="error">{e}</p> })}

            {move || {
                let list = todos.get();
                let has_today = status.get().is_some_and(|s| s.has_today);
                if !has_today {
                    view! {
                        <p class="empty-state">
                            "Start your day with Create todos for today. Incomplete items from the previous daily will carry forward."
                        </p>
                    }.into_view()
                } else if list.is_empty() {
                    view! {
                        <p class="empty-state">"No todos on today's list yet. Pull items from General."</p>
                    }.into_view()
                } else {
                    view! {
                        <ul class="todo-list">
                            {list.into_iter().map(|todo| {
                                let carried = todo.carried_from.clone();
                                view! {
                                    <div class="daily-todo-wrap">
                                        {carried.map(|d| view! {
                                            <span class="carried-from">{format!("Carried from {d}")}</span>
                                        })}
                                        <TodoItem
                                            todo=todo
                                            on_changed=set_refresh
                                            refresh=refresh
                                            show_section=true
                                            show_daily_actions=true
                                        />
                                    </div>
                                }
                            }).collect_view()}
                        </ul>
                    }.into_view()
                }
            }}
        </div>
    }
}
