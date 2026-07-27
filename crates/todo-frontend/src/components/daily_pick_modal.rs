use leptos::*;
use std::collections::HashSet;
use todo_shared::{Section, Todo};
use wasm_bindgen_futures::spawn_local;

use crate::api;

/// Modal to pick incomplete General todos to add to a daily board.
#[component]
#[allow(clippy::needless_pass_by_value)]
pub fn DailyPickModal(
    /// Target daily date YYYY-MM-DD
    target_date: String,
    on_close: Callback<()>,
    set_refresh: WriteSignal<usize>,
    refresh: ReadSignal<usize>,
) -> impl IntoView {
    let (candidates, set_candidates) = create_signal(Vec::<Todo>::new());
    let (selected, set_selected) = create_signal(HashSet::<String>::new());
    let (loading, set_loading) = create_signal(true);
    let (saving, set_saving) = create_signal(false);
    let (error, set_error) = create_signal(None::<String>);

    // Load incomplete todos not already on the target day
    {
        let target = target_date.clone();
        spawn_local(async move {
            match api::fetch_todos(None, Some("importance_date"), None).await {
                Ok(list) => {
                    let list: Vec<Todo> = list
                        .into_iter()
                        .filter(|t| !t.completed)
                        .filter(|t| t.daily_date.as_deref() != Some(target.as_str()))
                        .collect();
                    set_candidates.set(list);
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    }

    let toggle_id = move |id: String| {
        set_selected.update(|set| {
            if set.contains(&id) {
                set.remove(&id);
            } else {
                set.insert(id);
            }
        });
    };

    let on_confirm = {
        let target = target_date.clone();
        move |_| {
            if saving.get_untracked() {
                return;
            }
            let ids: Vec<String> = selected.get_untracked().into_iter().collect();
            if ids.is_empty() {
                on_close.call(());
                return;
            }
            set_saving.set(true);
            let target = target.clone();
            spawn_local(async move {
                for id in ids {
                    if let Err(e) = api::set_todo_daily(&id, Some(&target)).await {
                        set_error.set(Some(e));
                        set_saving.set(false);
                        return;
                    }
                }
                set_saving.set(false);
                set_refresh.set(refresh.get_untracked() + 1);
                on_close.call(());
            });
        }
    };

    let on_skip = move |_| on_close.call(());

    view! {
        <div class="modal-backdrop" on:click=on_skip>
            <div class="modal-panel" on:click=move |ev| ev.stop_propagation()>
                <div class="modal-header">
                    <h3>{format!("Add from General · {target_date}")}</h3>
                    <button class="modal-close" on:click=on_skip>"\u{00d7}"</button>
                </div>
                <p class="modal-hint">"Select incomplete todos to put on this day. Carried items are already included."</p>

                {move || error.get().map(|e| view! { <p class="error">{e}</p> })}

                {move || {
                    if loading.get() {
                        view! { <p class="empty-state">"Loading..."</p> }.into_view()
                    } else {
                        let list = candidates.get();
                        if list.is_empty() {
                            view! {
                                <p class="empty-state">"No other incomplete todos in General."</p>
                            }.into_view()
                        } else {
                            view! {
                                <div class="pick-list">
                                    {Section::all().iter().filter_map(|&section| {
                                        let section_items: Vec<Todo> = list.iter()
                                            .filter(|t| t.section == section)
                                            .cloned()
                                            .collect();
                                        if section_items.is_empty() {
                                            return None;
                                        }
                                        Some(view! {
                                            <div class="pick-section">
                                                <h4>{section.as_str()}</h4>
                                                {section_items.into_iter().map(|todo| {
                                                    let id = todo.id.clone();
                                                    let id_check = todo.id.clone();
                                                    let title = todo.title.clone();
                                                    let imp = todo.importance.label().to_string();
                                                    view! {
                                                        <label class="pick-row">
                                                            <input
                                                                type="checkbox"
                                                                prop:checked=move || selected.get().contains(&id_check)
                                                                on:change=move |_| toggle_id(id.clone())
                                                            />
                                                            <span class="pick-title">{title}</span>
                                                            <span class="pick-meta">{imp}</span>
                                                        </label>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        })
                                    }).collect_view()}
                                </div>
                            }.into_view()
                        }
                    }
                }}

                <div class="modal-actions">
                    <button class="modal-secondary" on:click=on_skip prop:disabled=saving>"Skip"</button>
                    <button class="modal-primary" on:click=on_confirm prop:disabled=saving>
                        {move || {
                            let n = selected.get().len();
                            if saving.get() {
                                "Adding...".into()
                            } else if n == 0 {
                                "Done".into()
                            } else {
                                format!("Add {n} to day")
                            }
                        }}
                    </button>
                </div>
            </div>
        </div>
    }
}
