#![allow(non_snake_case)]
// The dioxus prelude contains a ton of common items used in dioxus apps. It's a good idea to
// import wherever you need dioxus
use dioxus::prelude::*;
use dioxus_logger::{self, tracing::Level};

use syntect::{highlighting::ThemeSet, parsing::SyntaxSet};
use views::{Blog, Editor, Home, Navbar};

/// Define a components module that contains all shared components for our app.
mod components;
/// Define a views module that contains the UI for all Layouts and Routes for our app.
mod views;

/// The Route enum is used to define the structure of internal routes in our app. All route enums
/// need to derive the [`Routable`] trait, which provides the necessary methods for the router to
/// work.
///
/// Each variant represents a different URL pattern that can be matched by the router. If that
/// pattern is matched, the components for that route will be rendered.
#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    // The layout attribute defines a wrapper for all routes under the layout. Layouts are great
    // for wrapping many routes with a common UI like a navbar.
    #[layout(Navbar)]
        // The route attribute defines the URL pattern that a specific route matches. If that
        // pattern matches the URL, the component for that route will be rendered. The component name
        // that is rendered defaults to the variant name.
        #[route("/")]
        Home {},
        #[route("/editor")]
        Editor {},
        // The route attribute can include dynamic parameters that implement [`std::str::FromStr`]
        // and [`std::fmt::Display`] with the `:` syntax. In this case, id will match any integer like
        // `/blog/123` or `/blog/-456`.
        #[route("/blog/:id")]
        // Fields of the route variant will be passed to the component as props. In this case, the
        // blog component must accept an `id` prop of type `i32`.
        Blog { id: i32 },
}

// We can import assets in dioxus with the `asset!` macro. This macro takes a path to an asset
// relative to the crate root. The macro returns an `Asset` type that will display as the path to
// the asset in the browser or a local path in desktop bundles.
const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");
const QUILL_SNOW_CSS: Asset = asset!("/assets/quill.snow.css");
const QUILL_JS: Asset = asset!("/assets/quill.2.0.3.js");

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    // The `launch` function is the main entry point for a dioxus app. It takes a component and
    // renders it with the platform feature you have enabled
    dioxus::launch(App);
}

pub struct State {
    pub ss: SyntaxSet,
    pub ts: ThemeSet,
}

impl State {
    fn new() -> Self {
        Self {
            ss: SyntaxSet::load_defaults_newlines(),
            ts: ThemeSet::load_defaults(),
        }
    }
}

/// App is the main component of our app. Components are the building blocks of dioxus apps. Each
/// component is a function that takes some props and returns an Element. In this case, App takes
/// no props because it is the root of our app.
///
/// Components should be annotated with `#[component]` to support props, better error messages, and
/// autocomplete
#[component]
fn App() -> Element {
    // Load these once at the start of your program
    use_context_provider(|| Signal::new(State::new()));

    // The `rsx!` macro lets us define HTML inside of rust. It expands to an Element with all of
    // our HTML inside.
    rsx! {
        // In addition to element and text (which we will see later), rsx can contain other
        // components. In this case, we are using the `document::Link` component to add a link to
        // our favicon and main CSS file into the head of our app.
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        document::Link { rel: "stylesheet", href: QUILL_SNOW_CSS }

        script { src: QUILL_JS }

        // The router component renders the route enum we defined above. It will handle
        // synchronization of the URL and render the layouts and components for the active route.
        Router::<Route> {}
    }
}
