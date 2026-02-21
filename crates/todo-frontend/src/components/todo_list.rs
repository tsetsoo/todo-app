use leptos::*;
use todo_shared::Todo;

use super::todo_item::TodoItem;

#[component]
pub fn TodoList(
    todos: Signal<Vec<Todo>>,
    on_changed: WriteSignal<usize>,
    refresh: ReadSignal<usize>,
) -> impl IntoView {
    view! {
        <ul class="todo-list">
            {move || {
                let items = todos.get();
                if items.is_empty() {
                    view! { <p class="empty-state">"No todos yet. Add one above!"</p> }.into_view()
                } else {
                    items.into_iter().map(|todo| {
                        view! {
                            <TodoItem
                                todo=todo
                                on_changed=on_changed
                                refresh=refresh
                            />
                        }
                    }).collect_view()
                }
            }}
        </ul>
    }
}
