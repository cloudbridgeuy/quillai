use std::rc::Rc;

use dioxus::{html::geometry::euclid::Rect, prelude::*};

#[component]
pub fn Editor() -> Element {
    let mut container: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut dimensions = use_signal(Rect::zero);

    let height = dimensions.read().origin.y;

    rsx! {
        div {
            class: "p-4",
            div {
                class:"flex p-4 m-d color-blue bg-gray-800 text-blue-400 font-mono resize-none overflow-hidden border border-gray-700 rounded-md",
                spellcheck:"true",
                contenteditable:"true",
                style: format!("font-family: monospace; padding: 1rem; overflow: hidden; height: calc(100vh - {}px - 1rem); max-height: calc(100vh - {}px - 1rem); max-width: calc(100vw - 2rem); width: calc(100vw - 2rem)", height, height),
                onresize: move |_| async move {
                    if let Some(data) = container() {
                        dimensions.set(data.get_client_rect().await.unwrap_or(Rect::zero()));
                    }
                },
                onmounted: move |element| {
                    let data = element.data();
                    container.set(Some(data.clone()));
                    async move {
                        dimensions.set(data.get_client_rect().await.unwrap_or(Rect::zero()));
                    }
                },
            }
        }
    }
}
