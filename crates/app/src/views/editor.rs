use dioxus::prelude::*;

/// The Home page component that will be rendered when the current route is `[Route::Home]`
#[component]
pub fn Editor() -> Element {
    // Get the
    rsx! {
        h1 {
            class: "p-4 mt-8 text-2xl font-bold",
            "Editor"
        }
        crate::components::Editor {}
    }
}
