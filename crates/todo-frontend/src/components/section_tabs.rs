use leptos::*;
use todo_shared::Section;

#[component]
pub fn SectionTabs(
    active: ReadSignal<Section>,
    set_active: WriteSignal<Section>,
    counts: Signal<Vec<(Section, usize)>>,
) -> impl IntoView {
    let sections = Section::all();

    view! {
        <div class="section-tabs">
            {sections.iter().map(|&s| {
                let count = move || {
                    counts.get().iter()
                        .find(|(sec, _)| *sec == s)
                        .map(|(_, c)| *c)
                        .unwrap_or(0)
                };
                let is_active = move || active.get() == s;
                let label = s.as_str();
                view! {
                    <button
                        class:active=is_active
                        on:click=move |_| set_active.set(s)
                    >
                        {label}
                        <span class="count">"(" {count} ")"</span>
                    </button>
                }
            }).collect_view()}
        </div>
    }
}
