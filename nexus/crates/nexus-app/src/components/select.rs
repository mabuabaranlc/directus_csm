use leptos::prelude::*;

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

#[component]
pub fn Select(
    #[prop(optional)] label: Option<String>,
    options: Vec<SelectOption>,
    #[prop(optional, into)] value: RwSignal<String>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    view! {
        <div class="select-group">
            {label.map(|l| view! { <label class="input-label">{l}</label> })}
            <select
                class="select-field"
                prop:value=move || value.get()
                on:change=move |ev| value.set(event_target_value(&ev))
                disabled=move || disabled.get()
            >
                {placeholder.map(|p| view! { <option value="" disabled=true selected=true>{p}</option> })}
                {options.into_iter().map(|opt| view! {
                    <option value=opt.value.clone()>{opt.label}</option>
                }).collect::<Vec<_>>()}
            </select>
        </div>
    }
}
