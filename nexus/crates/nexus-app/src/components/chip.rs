use leptos::prelude::*;

#[component]
pub fn Chip(
    #[prop(into)] label: String,
    #[prop(optional)] close: Option<Callback<()>>,
    #[prop(optional, into)] color: Option<String>,
) -> impl IntoView {
    let style = color.map(|c| format!("background-color: {c}")).unwrap_or_default();
    view! {
        <span class="chip" style=style>
            <span class="chip-label">{label}</span>
            {close.map(|cb| view! {
                <button class="chip-close" on:click=move |_| cb.run(())>
                    "\u{00d7}"
                </button>
            })}
        </span>
    }
}
