use leptos::*;

/// Whether today's daily board exists (for Add to Daily buttons).
#[derive(Clone, Copy)]
pub struct DailyBoardExists(pub ReadSignal<bool>);
