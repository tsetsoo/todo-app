use leptos::*;
use todo_shared::{DailyStatus, Section, Todo};
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::components::daily_pick_modal::DailyPickModal;
use crate::components::daily_quadrant::DailyQuadrant;

#[component]
#[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
pub fn DailyView(refresh: ReadSignal<usize>, set_refresh: WriteSignal<usize>) -> impl IntoView {
    let (status, set_status) = create_signal(None::<DailyStatus>);
    let (viewed_date, set_viewed_date) = create_signal(api::local_today());
    let (todos, set_todos) = create_signal(Vec::<Todo>::new());
    let (error, set_error) = create_signal(None::<String>);
    let (picker_date, set_picker_date) = create_signal(None::<String>);

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
                    set_todos.set(list);
                    set_error.set(None);
                }
                Err(e) => set_error.set(Some(e)),
            }
            let today = api::local_today();
            if date == today {
                if let Ok(s) = api::fetch_daily_status(&today).await {
                    set_status.set(Some(s));
                }
            }
        });
    });

    let daily_todos = Signal::derive(move || todos.get());
    let board_ready = Signal::derive(move || {
        let today = api::local_today();
        let date = viewed_date.get();
        if date == today {
            status.get().is_some_and(|s| s.has_today)
        } else if date == api::shift_date(&today, 1) {
            status.get().is_some_and(|s| s.has_tomorrow)
        } else {
            true
        }
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
            let target = if for_day == "tomorrow" {
                api::shift_date(&today, 1)
            } else {
                today
            };
            set_viewed_date.set(target.clone());
            set_picker_date.set(Some(target));
            set_refresh.set(refresh.get_untracked() + 1);
        });
    };

    let close_picker = Callback::new(move |()| {
        set_picker_date.set(None);
    });

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
                let today = api::local_today();
                let date = viewed_date.get();
                let is_today = date == today;
                let has_board = if is_today {
                    status.get().is_some_and(|s| s.has_today)
                } else {
                    true
                };

                if is_today && !has_board {
                    view! {
                        <p class="empty-state">
                            "Start your day with Create todos for today. You'll pick items from General, and incomplete items from the previous daily will carry forward."
                        </p>
                    }.into_view()
                } else {
                    view! {
                        <div class="quadrant-grid">
                            {Section::all().iter().map(|&section| {
                                view! {
                                    <DailyQuadrant
                                        section=section
                                        date=viewed_date
                                        refresh=refresh
                                        set_refresh=set_refresh
                                        daily_todos=daily_todos
                                        board_ready=board_ready
                                    />
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}

            {move || {
                picker_date.get().map(|date| {
                    view! {
                        <DailyPickModal
                            target_date=date
                            on_close=close_picker
                            set_refresh=set_refresh
                            refresh=refresh
                        />
                    }
                })
            }}
        </div>
    }
}
