use leptos::*;
use todo_shared::Section;

use crate::components::all_view::AllView;
use crate::components::quadrant::Quadrant;
use crate::components::section_view::SectionView;

#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Home,
    Section(Section),
    AllByImportance,
}

#[component]
pub fn App() -> impl IntoView {
    let (refresh, set_refresh) = create_signal(0_usize);
    let (current_view, set_current_view) = create_signal(View::Home);

    // Signal used by SectionView to navigate back
    let (go_back, set_go_back) = create_signal(false);
    create_effect(move |_| {
        if go_back.get() {
            set_current_view.set(View::Home);
            set_go_back.set(false);
        }
    });

    let on_open_section = Callback::new(move |s: Section| {
        set_current_view.set(View::Section(s));
    });

    view! {
        <div class="app">
            <div class="app-header">
                <h1>"TODO List"</h1>
                <div class="view-toggle">
                    <button
                        class:active=move || current_view.get() == View::Home
                        on:click=move |_| set_current_view.set(View::Home)
                    >
                        "Home"
                    </button>
                    {Section::all().iter().map(|&section| {
                        let label = section.as_str();
                        view! {
                            <button
                                class:active=move || current_view.get() == View::Section(section)
                                on:click=move |_| set_current_view.set(View::Section(section))
                            >
                                {label}
                            </button>
                        }
                    }).collect_view()}
                    <button
                        class:active=move || current_view.get() == View::AllByImportance
                        on:click=move |_| set_current_view.set(View::AllByImportance)
                    >
                        "All"
                    </button>
                </div>
            </div>
            {move || {
                match current_view.get() {
                    View::Home => {
                        view! {
                            <div class="quadrant-grid">
                                {Section::all().iter().map(|&section| {
                                    view! {
                                        <Quadrant
                                            section=section
                                            refresh=refresh
                                            set_refresh=set_refresh
                                            on_open=on_open_section
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_view()
                    }
                    View::Section(section) => {
                        view! {
                            <SectionView
                                section=section
                                refresh=refresh
                                set_refresh=set_refresh
                                on_back=set_go_back
                            />
                        }.into_view()
                    }
                    View::AllByImportance => {
                        view! {
                            <AllView refresh=refresh set_refresh=set_refresh />
                        }.into_view()
                    }
                }
            }}
        </div>
    }
}
