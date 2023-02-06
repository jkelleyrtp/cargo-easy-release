use cargo_metadata::Metadata;
use dioxus::prelude::*;

use crate::CrateGraph;

#[inline_props]
pub fn Header<'a>(cx: Scope<'a>, meta: &'a Metadata, graph: &'a UseState<CrateGraph>) -> Element {
    render! {
        div { class: "mb-12",
            div { class: "flex flex-row justify-between",
                h3 { class: "text-4xl md:text-4xl leading-tight font-medium text-gray-900 font-bold tracking-tighter mb-1",
                    "Cargo Easy Release"
                }
                div { class: "flex flex-col text-right",
                    a { href: "http://github.com/jkelleyrtp/cargo-easy-release", span { "View on Github" } }
                    h2 { class: "text-gray-500 font-medium text-md mb-1", "With ❤️ from Dioxus Labs" }
                }
            }

            h2 { class: "text-gray-500 font-medium text-md mb-2", "{meta.workspace_root}" }
        }
    }
}
