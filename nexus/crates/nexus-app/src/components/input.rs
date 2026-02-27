use leptos::prelude::*;

#[component]
pub fn Input(
    #[prop(optional)] label: Option<String>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional, into)] value: RwSignal<String>,
    #[prop(optional)] input_type: &'static str,
    #[prop(optional)] disabled: Signal<bool>,
    #[prop(optional)] required: bool,
    #[prop(optional)] autofocus: bool,
    #[prop(optional)] note: Option<String>,
    #[prop(optional)] error: Option<Signal<Option<String>>>,
) -> impl IntoView {
    let input_type = if input_type.is_empty() { "text" } else { input_type };

    let has_error = move || {
        error
            .map(|e| e.get().is_some())
            .unwrap_or(false)
    };

    view! {
        <div class="input-group" class:has-error=has_error>
            {label.map(|l| view! {
                <label class="input-label">
                    {l}
                    <Show when=move || required>
                        <span class="required-mark">"*"</span>
                    </Show>
                </label>
            })}
            <input
                class="input-field"
                type=input_type
                placeholder=placeholder.unwrap_or_default()
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
                disabled=move || disabled.get()
                required=required
                autofocus=autofocus
            />
            {note.map(|n| view! { <div class="input-note">{n}</div> })}
            {error.map(|e| view! {
                <Show when=move || e.get().is_some()>
                    <div class="input-error">{move || e.get().unwrap_or_default()}</div>
                </Show>
            })}
        </div>
    }
}

#[component]
pub fn Textarea(
    #[prop(optional)] label: Option<String>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional, into)] value: RwSignal<String>,
    #[prop(optional)] rows: u32,
    #[prop(optional)] disabled: Signal<bool>,
) -> impl IntoView {
    let rows = if rows == 0 { 4 } else { rows };

    view! {
        <div class="input-group">
            {label.map(|l| view! { <label class="input-label">{l}</label> })}
            <textarea
                class="input-field textarea"
                placeholder=placeholder.unwrap_or_default()
                rows=rows
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
                disabled=move || disabled.get()
            />
        </div>
    }
}
