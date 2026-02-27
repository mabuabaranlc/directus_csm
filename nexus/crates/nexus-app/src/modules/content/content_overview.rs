use leptos::prelude::*;
use leptos_router::components::A;
use crate::stores::collections::CollectionsStore;
use crate::components::icon::Icon;

#[component]
pub fn ContentOverview() -> impl IntoView {
    let collections_store = expect_context::<CollectionsStore>();

    let visible = move || collections_store.visible_collections();

    let collection_cards = move || {
        let colls = visible();
        if colls.is_empty() {
            return view! {
                <div class="empty-state">
                    <div class="empty-state-icon"><Icon name="database"/></div>
                    <h3 class="empty-state-title">"No Collections"</h3>
                    <p class="empty-state-desc">"There are no visible collections. Create one in Settings."</p>
                </div>
            }.into_any();
        }

        let cards = colls.into_iter().map(|collection| {
            let href = format!("/content/{}", collection.collection);
            let icon = collection.display_icon().to_string();
            let name = collection.collection.clone();
            let note = collection.note.clone();
            view! {
                <A href=href attr:class="collection-card">
                    <div class="collection-card-icon">
                        <Icon name=icon/>
                    </div>
                    <div class="collection-card-info">
                        <h3>{name}</h3>
                        {note.map(|n| view! { <p>{n}</p> })}
                    </div>
                </A>
            }
        }).collect::<Vec<_>>();

        view! {
            <div class="collection-grid">
                {cards}
            </div>
        }.into_any()
    };

    view! {
        <div class="content-overview module-page">
            <div class="page-header">
                <h1>"Content"</h1>
            </div>
            <div class="page-body">
                {collection_cards}
            </div>
        </div>
    }
}
