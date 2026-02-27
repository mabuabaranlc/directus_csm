use leptos::prelude::*;

#[component]
pub fn Checkbox(
    #[prop(optional)] label: Option<String>,
    #[prop(optional, into)] checked: RwSignal<bool>,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <label class="checkbox-group" class:disabled=move || disabled.get()>
            <input
                type="checkbox"
                class="checkbox-input"
                prop:checked=move || checked.get()
                on:change=move |_| checked.update(|v| *v = !*v)
                disabled=move || disabled.get()
            />
            <span class="checkbox-mark"/>
            {label.map(|l| view! { <span class="checkbox-label">{l}</span> })}
        </label>
    }
}
