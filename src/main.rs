#![allow(non_snake_case)]

use cargo_metadata::{DependencyKind, Metadata, Package, PackageId};
use clap::Parser;
use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

mod cargo_util;
mod templates;

fn main() {
    dioxus_desktop::launch_with_props(
        app,
        collect_workspace_meta(),
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
    let graph = use_state(cx, || CrateGraph::new(&cx.props));
    let released_crates = use_state(cx, || HashSet::<PackageId>::new());
    let ignored_crates = use_state(cx, || {
        cx.props
            .packages
            .iter()
            .filter(|package| graph.crates.contains(&package.id) && package.publish == Some(vec![]))
            .map(|package| package.id.clone())
            .collect::<HashSet<PackageId>>()
    });
    let unrelease_crates = use_state(cx, || {
        graph
            .crates
            .iter()
            .filter(|f| !ignored_crates.contains(&f))
            .cloned()
            .collect::<HashSet<PackageId>>()
    });

    let render_graph = use_render_graph(cx, graph);

    cx.render(rsx! {
        section {
            class: "py-12 bg-white  font-mono",
            style: "background-image: url('flex-ui-assets/elements/pattern-white.svg'); background-position: center;",

            div { class: "container px-4 mx-auto max-w-screen-2xl",
                div { class: "mb-12",
                    div { class: "mb-8",
                        div { class: "flex flex-row justify-between",
                            h3 { class: "text-4xl md:text-4xl leading-tight font-medium text-gray-900 font-bold tracking-tighter mb-1",
                                "Easy Release"

                                span { class: "text-gray-500",
                                    " v0.1.0"
                                }
                            }
                            div { class: "flex flex-col text-right",
                                span { "View on Github" }
                                h2 {  class: "text-gray-500 font-medium text-md mb-1",
                                    "With ❤️ from Dioxus Labs"
                                }
                            }
                        }

                        h2 {  class: "text-gray-500 font-medium text-md mb-2",
                            "/Users/alexander/Projects/dioxus/cargo-easy-release"
                        }
                    }
                    p { class: "text-gray-800",
                        ul {
                            li { "12 workspace crates found" }
                            li { "7 crates out of date" }
                            li { "14 breaking API changes" }
                            li { "3 crates need fixes to be released" }
                        }
                    }
                }

                div { class: "flex flex-row",
                    div { class: "",
                        div { class: "border-b border-gray-200 mb-4 pb-2 flex flex-row justify-between",
                            h3 { class: "text-md leading-tight font-medium text-gray-900 font-bold",
                                "Workspace Crates (12)"
                            }

                            div {
                                input { r#type: "checkbox" }
                                label { "Allow dirty?" }
                                input { r#type: "checkbox" }
                                label { "Dry run?" }
                            }
                        }

                        div { class: "grid grid-cols-3 gap-4 justify-between",
                            for (id, _) in render_graph.iter().filter(|id| !ignored_crates.contains(&id.0)) {
                                RowItem {
                                    graph: graph,
                                    meta: &cx.props,
                                    id: id.clone(),
                                    released_crates: released_crates.clone(),
                                    unrelease_crates: unrelease_crates.clone(),
                                    ignored_crates: ignored_crates.clone(),
                                }
                            }
                        }


                        // if !ignored_crates.is_empty() {
                        //     rsx! {
                        //         div {
                        //             class: "w-full border-t border-gray-200 mt-4 text-gray-500 text-center",
                        //             "ignored"
                        //         }
                        //     }
                        // }

                        // for id in ignored_crates.iter() {
                        //     row_item {
                        //         graph: graph,
                        //         meta: &cx.props,
                        //         id: id.clone(),
                        //         released_crates: released_crates.clone(),
                        //         unrelease_crates: unrelease_crates.clone(),
                        //         ignored_crates: ignored_crates.clone(),
                        //     }
                        // }
                    }
                }
            }
        }
    })
}

fn use_render_graph<'a>(
    cx: &'a ScopeState,
    graph: &'a CrateGraph,
) -> &'a UseState<Vec<(PackageId, usize)>> {
    use_state(cx, || {
        // sort the crates by the number of dependencies they have
        // we want to put the zero-dependency crates at the to release them in the right order
        let mut render_graph = graph
            .crates
            .iter()
            .map(|id| {
                // Count the number of direct dependencies
                let mut num_deps = graph.ws_deps[id].len();

                // Recruisively count the number of indirect dependencies that are within the workspace
                // Just do three levels because we're lazy AF
                for dep in &graph.ws_deps[id] {
                    num_deps += graph.ws_deps[dep].len();

                    for dep in &graph.ws_deps[dep] {
                        num_deps += graph.ws_deps[dep].len();

                        for dep in &graph.ws_deps[dep] {
                            num_deps += graph.ws_deps[dep].len();
                        }
                    }
                }

                (id.clone(), num_deps)
            })
            .collect::<Vec<_>>();

        render_graph.sort_by_key(|(_, num_deps)| *num_deps);

        render_graph
    })
}

#[inline_props]
fn RowItem<'a>(
    cx: Scope<'a>,
    // lol that's a lot of fields
    graph: &'a CrateGraph,
    meta: &'a Metadata,
    id: PackageId,
    released_crates: UseState<HashSet<PackageId>>,
    unrelease_crates: UseState<HashSet<PackageId>>,
    ignored_crates: UseState<HashSet<PackageId>>,
) -> Element {
    let release_button = unrelease_crates.contains(id).then(|| rsx! {
        div { class: "w-full pb-2 text-xs pl-2",
            button {
                class: "inline-flex ml-auto items-center font-medium leading-6 text-gray-500 group-hover:text-green-600 transition duration-200 pr-2",
                onclick: move |_| {
                    ignored_crates.make_mut().insert(id.clone());
                    unrelease_crates.make_mut().remove(&id);
                },
                span { class: "mr-2", "Ignore" }
                templates::icons::icon_0 {}
            }

            button {
                class: "inline-flex ml-auto items-center font-medium leading-6 text-green-500 group-hover:text-green-600 transition duration-200 ",
                onclick: move |_| {
                    // released_crates.make_mut().insert(id.clone());
                    // unrelease_crates.make_mut().remove(&id);
                    println!("attempting to release crate..");
                },
                span { class: "mr-2", "Release" }
                templates::icons::icon_0 {}
            }
        }
    });

    let package = meta.packages.iter().find(|p| p.id == *id).unwrap();

    cx.render(rsx! {
        div { class: "p-2 w-full mb-8 h-64 bg-gray-50 group-hover:bg-gray-100 rounded-md shadow-md transition duration-200 flex flex-col justify-between",
            div { class: "w-full",
                h3 { class: "mb-2 text-md text-gray-800 group-hover:text-gray-900 font-semibold transition duration-200 font-mono flex flex-row justify-between",
                    span { "{package.name}" }
                    span { class: "text-gray-500 ml-2", "{package.version}" }
                }

                div { class: "text-gray-500 text-xs flex flex-col",
                    div {
                        if package.keywords.is_empty() {
                            render! { "❌ Missing keywords" }
                        } else {
                            render! ( "✅ ", package.keywords.iter().map(|k| render!( "{k}, " )) )
                        }
                    }



                    // todo: throw an error if the version here matches the same version on crates, since
                    // crates will reject that version
                    div {
                        if package.authors.is_empty() {
                            render! { "❌ Missing authors" }
                        } else {
                            render! ( "✅ ", package.authors.iter().map(|k| render!( "{k}, " )) )
                        }
                    }
                    div { "✅ ", "Edition {package.edition}" }
                    div {
                        if let Some(license) = package.license.as_deref() {
                            render! { "✅ ", license }
                        } else {
                            render!{ "❌ Missing license" }
                        }
                    }

                    span { class: "text-xs py-4", package.description.as_deref().unwrap_or("❌ Missing Description") }

                }

                // CrateDeps { graph: graph, id: id.clone(), meta: meta, released_crates: released_crates.clone() }
            }
            release_button
        }
    })
}

#[inline_props]
fn CrateDeps<'a>(
    cx: Scope<'a>,
    graph: &'a CrateGraph,
    meta: &'a Metadata,
    id: PackageId,
    released_crates: UseState<HashSet<PackageId>>,
) -> Element {
    cx.render(rsx! {
        ul {
            graph.ws_deps[id].iter().map(|f| {
                let package = meta.packages.iter().find(|p| p.id == *f).unwrap();
                let emoij = if released_crates.contains(f) {
                    "👍"
                } else {
                    "👎"
                };
                cx.render(rsx! {
                    li { "{emoij}" "{package.name}" }
                })
            })
        }
    })
}

// #[inline_props]
// fn Description<'a>(cx: Scope<'a>, package: &'a Package) -> Element {
//     // // todo: download the metadata from the crates index using reqwest/downloader
//     // let url = format!(
//     //     "https://raw.githubusercontent.com/rust-lang/crates.io-index/master/{}/{}/{}",
//     //     package.name.chars().next().unwrap(),
//     //     package.name.chars().nth(1).unwrap(),
//     //     package.name
//     // );

//     // Required fields include name, version, edition, edition, keywords, description, license, authors, license, tags, description
//     //
//     // name = "dioxus-liveview"
//     // version = "0.3.0"
//     // edition = "2021"
//     // keywords = ["dom", "ui", "gui", "react", "wasm"]
//     // description = "Build server-side apps with Dioxus"
//     // license = "MIT/Apache-2.0"
//     //
//     // homepage = "https://dioxuslabs.com"
//     // documentation = "https://dioxuslabs.com"
//     // repository = "https://github.com/DioxusLabs/dioxus/"
//     cx.render(rsx! {
//         div { class: "text-gray-500 text-xs flex flex-col",
//             // todo: throw an error if the version here matches the same version on crates, since
//             // crates will reject that version
//             div {
//                 if package.authors.is_empty() {
//                     render! { "Missing authors" }
//                 } else {
//                     render! ( package.authors.iter().map(|k| render!( "{k}, " )) )
//                 }
//             }
//             div { "Version {package.version}", }
//             div { "Edition {package.edition}" }
//             div {
//                 if package.keywords.is_empty() {
//                     render! { "Missing keywords" }
//                 } else {
//                     render! ( package.keywords.iter().map(|k| render!( "{k}, " )) )
//                 }
//             }
//             span { "License: " package.license.as_deref().unwrap_or("❌ missing") }
//             span { class: "text-xs", package.description.as_deref().unwrap_or("❌ missing") }
//         }
//     })
// }

#[derive(clap::Parser)]
struct Args {
    path: Option<PathBuf>,
}

struct CrateGraph {
    crates: HashSet<PackageId>,

    // Neighbors within the workspace
    ws_deps: HashMap<PackageId, HashSet<PackageId>>,
}

impl CrateGraph {
    fn new(meta: &Metadata) -> Self {
        let crates: HashSet<PackageId> = meta.workspace_members.clone().into_iter().collect();
        let crate_names = crates
            .iter()
            .map(|id| {
                meta.packages
                    .iter()
                    .find(|p| p.id == *id)
                    .unwrap()
                    .name
                    .clone()
            })
            .collect::<HashSet<_>>();

        let mut deps = HashMap::new();

        for package in meta.workspace_packages() {
            let mut this_deps = HashSet::new();

            for dep in &package.dependencies {
                if dep.kind != DependencyKind::Normal {
                    continue;
                }

                if crate_names.contains(&dep.name) {
                    let dep = meta
                        .packages
                        .iter()
                        .find(|p| p.name == dep.name)
                        .unwrap()
                        .id
                        .clone();

                    this_deps.insert(dep);
                }
            }

            deps.insert(package.id.clone(), this_deps);
        }

        Self {
            crates,
            ws_deps: deps,
        }
    }
}

fn collect_workspace_meta() -> Metadata {
    use cargo_metadata::MetadataCommand;
    let args = Args::try_parse().unwrap();

    let mut cmd = MetadataCommand::new();

    if let Some(path) = args.path {
        cmd.manifest_path(path.join("Cargo.toml"));
    };

    cmd.exec().unwrap()
}
