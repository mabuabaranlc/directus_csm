use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn SearchInput(
    value: RwSignal<String>,
    #[prop(optional, into, default = "Search...".into())] placeholder: String,
) -> impl IntoView {
    view! {
        <div class="search-input">
            <Icon name="search"/>
            <input
                type="search"
                placeholder=placeholder
                prop:value=move || value.get()
                on:input=move |ev| {
                    value.set(event_target_value(&ev));
                }
            />
            <Show when=move || !value.get().is_empty()>
                <button class="search-clear" on:click=move |_| value.set(String::new())>
                    <Icon name="close"/>
                </button>
            </Show>
        </div>
    }
}
