#![allow(non_snake_case)]
use cargo_metadata::{Metadata, PackageId};
use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use dioxus_signals::use_init_signal_rt;
use fermi::use_init_atom_root;
use state::CrateGraph;
use std::collections::HashSet;

mod cargo_util;
mod config_banner;
mod header;
mod rows;
mod state;
mod templates;

fn main() {
    dioxus_desktop::launch_with_props(
        app,
        state::collect_workspace_meta(),
        Config::default()
            .with_window(
                WindowBuilder::new()
                    .with_title("Cargo Easy Release")
                    .with_inner_size(LogicalSize::new(1600, 1200)),
            )
            .with_custom_head(r#"<script src="https://cdn.tailwindcss.com"></script>"#.into()),
    );
}

fn app(cx: Scope<Metadata>) -> Element {
    use_init_atom_root(cx);
    use_init_signal_rt(cx);

    let graph = use_state(cx, || CrateGraph::new(&cx.props));
    let ignored_crates = use_state(cx, || {
        cx.props
            .packages
            .iter()
            .filter(|package| graph.crates.contains(&package.id) && package.publish == Some(vec![]))
            .map(|package| package.id.clone())
            .collect::<HashSet<PackageId>>()
    });

    render! {
        section { class: "py-12 bg-white font-mono container px-4 mx-auto max-w-screen-xl",

            header::Header { meta: cx.props, graph: graph }
            config_banner::GlobalConfig {}

            div {
                for (id , _) in graph.sorted.iter().filter(|id| !ignored_crates.contains(&id.0)) {
                    rows::RowItem { graph: graph, id: id.clone() }
                }

                if !ignored_crates.is_empty() {
                    rsx! {
                        div {
                            class: "w-full border-t border-gray-200 mt-4 text-gray-500 text-center",
                            "ignored"
                        }
                    }
                }

                for id in ignored_crates.iter() {
                    rows::RowItem { graph: graph, id: id.clone() }
                }
            }
        }
    }
}
