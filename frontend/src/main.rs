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
            class: "bg-slate-900 w-64 flex flex-col items-center justify-start p-4",
            div {
                class: "text-blue-500 font-bold text-2xl mb-8",
                "PYMZA"
            }
            ul {
                class: "flex flex-col",
                li {
                    class: "p-3 rounded-lg hover:bg-slate-800 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Dashboard"
                    }
                }
                li {
                    class: "p-3 rounded-lg bg-blue-900/50 text-blue-400 hover:bg-slate-800 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Evaluación"
                    }
                }
                li {
                    class: "p-3 rounded-lg hover:bg-slate-800 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Clientes"
                    }
                }
                li {
                    class: "p-3 rounded-lg hover:bg-red-900/30 text-red-400 hover:text-white transition-colors",
                    a {
                        href: "#",
                        class: "flex items-center",
                        svg { class: "w-5 h-5 mr-3" },
                        "Red PYME"
                    }
                }
            }
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
