use leptos::prelude::*;

#[component]
pub fn Toggle(
    #[prop(optional)] label: Option<String>,
    #[prop(optional, into)] value: RwSignal<bool>,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <label class="toggle-group" class:disabled=move || disabled.get()>
            <div class="toggle-track" class:active=move || value.get()
                on:click=move |_| { if !disabled.get() { value.update(|v| *v = !*v); } }>
                <div class="toggle-thumb"/>
            </div>
            {label.map(|l| view! { <span class="toggle-label">{l}</span> })}
        </label>
    }
}
