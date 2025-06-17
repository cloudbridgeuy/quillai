use std::rc::Rc;

use dioxus::{html::geometry::euclid::Rect, prelude::*};

#[component]
pub fn Editor() -> Element {
    let mut container: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut dimensions = use_signal(Rect::zero);

    let height = dimensions.read().origin.y;

    use_effect(move || {
        // You can use the count value to update the DOM manually
    });

    let id: &str = "editor";
    rsx! {
        div {
            class: "p-4",

            div {
                id,
                class:"m-d color-blue bg-gray-800 text-blue-400 font-mono overflow-hidden border border-gray-700 rounded-md",
                style: format!("overflow: hidden; height: calc(100vh - {}px - 1rem); max-height: calc(100vh - {}px - 1rem); max-width: calc(100vw - 2rem); width: calc(100vw - 2rem)", height, height),
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
                        document::eval(
                            &format!(r#"
try {{
    const editor = document.getElementById('{id}');
    console.log(editor);
    window.quill = new Quill(editor, {{
        debug: 'info', modules: {{ toolbar: false }}, theme: 'snow'
    }});
}} catch(e) {{
    console.error('Failed to create the Quill Editor instance');
    console.error(e);
}}"#,
                        ));
                    }
                },
                p { "Hello World!" },
                p { "Some initial ", strong { "bold" }, " text" }
                p { br {} }
            }
        }
    }
}
