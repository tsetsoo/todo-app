use leptos::*;
use todo_shared::Section;
use wasm_bindgen_futures::spawn_local;

use crate::api;
use crate::components::all_view::AllView;
use crate::components::archive_view::ArchiveView;
use crate::components::daily_view::DailyView;
use crate::components::quadrant::Quadrant;
use crate::components::section_view::SectionView;
use crate::daily_ctx::DailyBoardExists;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Daily,
    General,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GeneralView {
    Home,
    Section(Section),
    AllByImportance,
    Archive,
}

#[component]
pub fn App() -> impl IntoView {
    let (refresh, set_refresh) = create_signal(0_usize);
    crate::ws::connect(set_refresh, refresh);
    let (mode, set_mode) = create_signal(Mode::Daily);
    let (general_view, set_general_view) = create_signal(GeneralView::Home);
    let (daily_board_exists, set_daily_board_exists) = create_signal(false);

    provide_context(DailyBoardExists(daily_board_exists));

    create_effect(move |_| {
        let _ = refresh.get();
        spawn_local(async move {
            let today = api::local_today();
            if let Ok(s) = api::fetch_daily_status(&today).await {
                set_daily_board_exists.set(s.has_today);
            }
        });
    });

    let (go_back, set_go_back) = create_signal(false);
    create_effect(move |_| {
        if go_back.get() {
            set_general_view.set(GeneralView::Home);
            set_go_back.set(false);
        }
    });

    let on_open_section = Callback::new(move |s: Section| {
        set_general_view.set(GeneralView::Section(s));
    });

    view! {
        <div class="app">
            <div class="app-header">
                <h1>"TODO List"</h1>
                <div class="view-toggle mode-toggle">
                    <button
                        class:active=move || mode.get() == Mode::Daily
                        on:click=move |_| set_mode.set(Mode::Daily)
                    >
                        "Daily"
                    </button>
                    <button
                        class:active=move || mode.get() == Mode::General
                        on:click=move |_| set_mode.set(Mode::General)
                    >
                        "General"
                    </button>
                </div>
            </div>

            {move || match mode.get() {
                Mode::Daily => view! {
                    <DailyView refresh=refresh set_refresh=set_refresh />
                }.into_view(),
                Mode::General => view! {
                    <div class="general-mode">
                        <div class="view-toggle general-toggle">
                            <button
                                class:active=move || general_view.get() == GeneralView::Home
                                on:click=move |_| set_general_view.set(GeneralView::Home)
                            >
                                "Home"
                            </button>
                            {Section::all().iter().map(|&section| {
                                let label = section.as_str();
                                view! {
                                    <button
                                        class:active=move || general_view.get() == GeneralView::Section(section)
                                        on:click=move |_| set_general_view.set(GeneralView::Section(section))
                                    >
                                        {label}
                                    </button>
                                }
                            }).collect_view()}
                            <button
                                class:active=move || general_view.get() == GeneralView::AllByImportance
                                on:click=move |_| set_general_view.set(GeneralView::AllByImportance)
                            >
                                "All"
                            </button>
                            <button
                                class:active=move || general_view.get() == GeneralView::Archive
                                on:click=move |_| set_general_view.set(GeneralView::Archive)
                            >
                                "Archive"
                            </button>
                        </div>
                        {move || match general_view.get() {
                            GeneralView::Home => view! {
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
                            }.into_view(),
                            GeneralView::Section(section) => view! {
                                <SectionView
                                    section=section
                                    refresh=refresh
                                    set_refresh=set_refresh
                                    on_back=set_go_back
                                />
                            }.into_view(),
                            GeneralView::AllByImportance => view! {
                                <AllView refresh=refresh set_refresh=set_refresh />
                            }.into_view(),
                            GeneralView::Archive => view! {
                                <ArchiveView refresh=refresh set_refresh=set_refresh />
                            }.into_view(),
                        }}
                    </div>
                }.into_view(),
            }}
        </div>
    }
}
