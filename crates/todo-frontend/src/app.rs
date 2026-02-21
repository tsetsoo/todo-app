use leptos::*;
use todo_shared::Section;

use crate::components::all_view::AllView;
use crate::components::quadrant::Quadrant;

#[component]
pub fn App() -> impl IntoView {
    let (refresh, set_refresh) = create_signal(0_usize);
    let (show_all, set_show_all) = create_signal(false);

    view! {
        <div class="app">
            <div class="app-header">
                <h1>"TODO List"</h1>
                <div class="view-toggle">
                    <button
                        class:active=move || !show_all.get()
                        on:click=move |_| set_show_all.set(false)
                    >
                        "Quadrants"
                    </button>
                    <button
                        class:active=move || show_all.get()
                        on:click=move |_| set_show_all.set(true)
                    >
                        "All by Importance"
                    </button>
                </div>
            </div>
            {move || {
                if show_all.get() {
                    view! {
                        <AllView refresh=refresh set_refresh=set_refresh />
                    }.into_view()
                } else {
                    view! {
                        <div class="quadrant-grid">
                            {Section::all().iter().map(|&section| {
                                view! {
                                    <Quadrant
                                        section=section
                                        refresh=refresh
                                        set_refresh=set_refresh
                                    />
                                }
                            }).collect_view()}
                        </div>
                    }.into_view()
                }
            }}
        </div>
    }
}
