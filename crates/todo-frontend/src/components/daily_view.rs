use leptos::*;
use todo_shared::{DailyStatus, Todo};
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::components::todo_item::TodoItem;

#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn DailyView(refresh: ReadSignal<usize>, set_refresh: WriteSignal<usize>) -> impl IntoView {
    let (status, set_status) = create_signal(None::<DailyStatus>);
    let (viewed_date, set_viewed_date) = create_signal(api::local_today());
    let (todos, set_todos) = create_signal(Vec::<Todo>::new());
    let (board_exists, set_board_exists) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);

    // Keep viewed_date on "today" when the calendar day rolls, if user was on today.
    create_effect(move |_| {
        let _ = refresh.get();
        let today = api::local_today();
        spawn_local(async move {
            match api::fetch_daily_status(&today).await {
                Ok(s) => set_status.set(Some(s)),
                Err(e) => set_error.set(Some(e)),
            }
        });
    });

    create_effect(move |_| {
        let _ = refresh.get();
        let date = viewed_date.get();
        spawn_local(async move {
            match api::fetch_daily(&date).await {
                Ok(list) => {
                    // Board "exists" if API returns ok; empty list may mean no board or empty board.
                    // Probe status/days via create status: if date == today use has_today; else any todos OR we allow browsing empty.
                    set_todos.set(list.clone());
                    set_board_exists.set(!list.is_empty());
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            // Refine board existence for today from status; for other dates, empty is still viewable.
            let today = api::local_today();
            if date == today {
                if let Ok(s) = api::fetch_daily_status(&today).await {
                    set_board_exists.set(s.has_today);
                    set_status.set(Some(s));
                }
            } else {
                // Past/future: treat as browsable; empty message differs.
                set_board_exists.set(true);
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
            let today = api::local_today();
            if for_day == "today" {
                set_viewed_date.set(today);
            } else if for_day == "tomorrow" {
                set_viewed_date.set(api::shift_date(&today, 1));
            }
            set_refresh.set(refresh.get_untracked() + 1);
        });
    };

    let go_prev = move |_| {
        set_viewed_date.update(|d| *d = api::shift_date(d, -1));
    };
    let go_next = move |_| {
        set_viewed_date.update(|d| *d = api::shift_date(d, 1));
    };
    let go_today = move |_| {
        set_viewed_date.set(api::local_today());
    };

    view! {
        <div class="daily-view">
            <div class="daily-header">
                <div>
                    <h2>"Daily"</h2>
                    <div class="daily-nav">
                        <button class="daily-nav-btn" on:click=go_prev title="Previous day">"←"</button>
                        <input
                            type="date"
                            class="daily-date-input"
                            prop:value=move || viewed_date.get()
                            on:change=move |ev| {
                                let v = event_target_value(&ev);
                                if !v.is_empty() {
                                    set_viewed_date.set(v);
                                }
                            }
                        />
                        <button class="daily-nav-btn" on:click=go_next title="Next day">"→"</button>
                        {move || {
                            let today = api::local_today();
                            (viewed_date.get() != today).then(|| view! {
                                <button class="daily-nav-btn today-btn" on:click=go_today>"Today"</button>
                            })
                        }}
                    </div>
                    {move || {
                        let today = api::local_today();
                        let date = viewed_date.get();
                        let label = if date == today {
                            if status.get().is_some_and(|s| s.has_today) {
                                format!("Today · {date}")
                            } else {
                                "No daily board for today yet".into()
                            }
                        } else if date > today {
                            format!("Upcoming · {date}")
                        } else {
                            format!("Past · {date}")
                        };
                        view! { <p class="daily-date-label">{label}</p> }
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
                let today = api::local_today();
                let date = viewed_date.get();
                let is_today = date == today;
                let has_board = if is_today {
                    status.get().is_some_and(|s| s.has_today)
                } else {
                    board_exists.get() || !list.is_empty()
                };

                if is_today && !has_board {
                    view! {
                        <p class="empty-state">
                            "Start your day with Create todos for today. Incomplete items from the previous daily will carry forward."
                        </p>
                    }.into_view()
                } else if list.is_empty() {
                    let msg = if is_today {
                        "No todos on today's list yet. Pull items from General.".to_string()
                    } else {
                        format!("No todos on {date}.")
                    };
                    view! {
                        <p class="empty-state">{msg}</p>
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
