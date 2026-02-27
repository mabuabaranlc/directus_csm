use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn Pagination(
    page: RwSignal<usize>,
    total: Signal<usize>,
    per_page: Signal<usize>,
) -> impl IntoView {
    let total_pages = move || {
        let pp = per_page.get().max(1);
        (total.get() + pp - 1) / pp
    };

    let can_prev = move || page.get() > 1;
    let can_next = move || page.get() < total_pages();

    view! {
        <div class="pagination">
            <button class="pagination-btn" disabled=move || !can_prev()
                on:click=move |_| page.update(|p| *p = p.saturating_sub(1))>
                <Icon name="chevron-left"/>
            </button>
            <span class="pagination-info">
                {move || format!("Page {} of {}", page.get(), total_pages().max(1))}
            </span>
            <button class="pagination-btn" disabled=move || !can_next()
                on:click=move |_| page.update(|p| *p += 1)>
                <Icon name="chevron-right"/>
            </button>
        </div>
    }
}
