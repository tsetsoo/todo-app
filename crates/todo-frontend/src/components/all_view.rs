use leptos::*;
use todo_shared::Importance;

use crate::api;
use super::todo_item::TodoItem;

#[component]
pub fn AllView(
    refresh: ReadSignal<usize>,
    set_refresh: WriteSignal<usize>,
) -> impl IntoView {
    let todos_resource = create_resource(
        move || refresh.get(),
        |_| async move {
            api::fetch_todos(None, Some("importance_date")).await.unwrap_or_default()
        },
    );

    let todos = Signal::derive(move || todos_resource.get().unwrap_or_default());

    view! {
        <div class="all-view">
            <h2>"All Todos by Importance"</h2>
            {move || {
                let items = todos.get();
                if items.is_empty() {
                    view! { <p class="empty-state">"No todos yet"</p> }.into_view()
                } else {
                    let groups: Vec<_> = Importance::all().iter().rev().map(|&imp| {
                        let group: Vec<_> = items.iter()
                            .filter(|t| t.importance == imp)
                            .cloned()
                            .collect();
                        (imp, group)
                    }).filter(|(_, g)| !g.is_empty()).collect();

                    groups.into_iter().map(|(imp, group)| {
                        let label = imp.label();
                        let class = format!("importance-group {}", imp.as_str());
                        view! {
                            <div class=class>
                                <h3>{label}</h3>
                                <ul class="todo-list">
                                    {group.into_iter().map(|todo| {
                                        view! {
                                            <TodoItem
                                                todo=todo
                                                on_changed=set_refresh
                                                refresh=refresh
                                                show_section=true
                                            />
                                        }
                                    }).collect_view()}
                                </ul>
                            </div>
                        }
                    }).collect_view()
                }
            }}
        </div>
    }
}
