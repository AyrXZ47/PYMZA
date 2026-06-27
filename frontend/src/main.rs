use dioxus::prelude::*;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }
        div {
            class: "flex h-screen text-white",
            Sidebar {}
            MainArea {}
        }
    }
}

#[component]
fn Sidebar() -> Element {
    rsx! {
        div {
            class: "bg-slate-900 w-64 flex flex-col items-center justify-center",
            "Sidebar Content"
        }
    }
}

#[component]
fn MainArea() -> Element {
    rsx! {
        div {
            class: "bg-slate-800 flex-1 p-4",
            "Main Area Content"
        }
    }
}
