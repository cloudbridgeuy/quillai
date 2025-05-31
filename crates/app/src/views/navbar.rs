use crate::Route;
use dioxus::prelude::*;

/// The Navbar component that will be rendered on all pages of our app since every page is under the layout.
///
///
/// This layout component wraps the UI of [Route::Home] and [Route::Blog] in a common navbar. The contents of the Home and Blog
/// routes will be rendered under the outlet inside this component
#[component]
pub fn Navbar() -> Element {
    rsx! {
        div {
            class: "flex p-4 flex-row gap-2",
            id: "navbar",
            Link {
                class: "text-white mr-5 no-underline hover:text-indigo-300 transition-colors duration-200 ease-in-out",
                to: Route::Home {},
                "Home"
            }
            Link {
                class: "text-white mr-5 no-underline hover:text-indigo-300 transition-colors duration-200 ease-in-out",
                to: Route::Editor {},
                "Editor"
            }
            Link {
                class: "text-white mr-5 no-underline hover:text-indigo-300 transition-colors duration-200 ease-in-out",
                to: Route::Blog { id: 1 },
                "Blog"
            }
        }

        // The `Outlet` component is used to render the next component inside the layout. In this case, it will render either
        // the [`Home`] or [`Blog`] component depending on the current route.
        Outlet::<Route> {}
    }
}
