use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            class: "flex h-screen",
            Sidebar {},
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
