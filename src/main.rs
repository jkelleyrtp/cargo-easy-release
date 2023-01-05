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
        Config::default().with_window(
            WindowBuilder::new()
                .with_title("Cargo Easy Release")
                .with_inner_size(LogicalSize::new(500, 800)),
        ),
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
        script { src: "https://cdn.tailwindcss.com"  }
        section {
            class: "py-24 bg-white",
            style: "background-image: url('flex-ui-assets/elements/pattern-white.svg'); background-position: center;",
            div { class: "container px-4 mx-auto",
                div { class: "max-w-3xl mb-12",
                    h3 { class: "mb-4 text-3xl md:text-4xl leading-tight font-medium text-coolGray-900 font-bold tracking-tighter",
                        "Cargo Easy Release"
                    }
                    p { class: "text-lg md:text-xl text-coolGray-500 font-medium", "Easily release your workspace crates to crates.io" }
                }

                div { class: "flex flex-row",
                    div { class: "",
                        h3 { class: "mb-4 text-md leading-tight font-medium text-coolGray-900 font-bold tracking-tighter",
                            "Workspace Crates"
                        }
                        div {
                            input { r#type: "checkbox" }
                            label { "Allow dirty?" }
                            input { r#type: "checkbox" }
                            label { "Dry run?" }
                        }

                        for (id, _) in render_graph.iter().filter(|id| !ignored_crates.contains(&id.0)) {
                            row_item {
                                graph: graph,
                                meta: &cx.props,
                                id: id.clone(),
                                released_crates: released_crates.clone(),
                                unrelease_crates: unrelease_crates.clone(),
                                ignored_crates: ignored_crates.clone(),
                            }
                        }

                        if !ignored_crates.is_empty() {
                            rsx! {
                                div {
                                    class: "w-full border-t border-coolGray-200 mt-4 text-coolGray-500 text-center",
                                    "ignored"
                                }
                            }
                        }

                        for id in ignored_crates.iter() {
                            row_item {
                                graph: graph,
                                meta: &cx.props,
                                id: id.clone(),
                                released_crates: released_crates.clone(),
                                unrelease_crates: unrelease_crates.clone(),
                                ignored_crates: ignored_crates.clone(),
                            }
                        }
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
fn row_item<'a>(
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
        div { class: "w-full lg:w-1/3 px-4 lg:text-right",
            button {
                class: "inline-flex ml-auto items-center font-medium leading-6 text-green-500 group-hover:text-green-600 transition duration-200",
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
                    released_crates.make_mut().insert(id.clone());
                    unrelease_crates.make_mut().remove(&id);

                    // todo: write the new version out to that crate's cargo.toml
                },
                span { class: "mr-2", "Release" }
                templates::icons::icon_0 {}
            }
        }
    });

    let package = meta.packages.iter().find(|p| p.id == *id).unwrap();

    cx.render(rsx! {
        div { class: "w-full px-4 mb-8",
            a { class: "group", href: "#",
                div { class: "bg-coolGray-50 group-hover:bg-coolGray-100 rounded-md shadow-md transition duration-200",
                    div { class: "flex flex-wrap items-start justify-between p-2 -mx-4",
                        div { class: "w-full lg:w-2/3 px-4 mb-6 lg:mb-0",
                            h3 { class: "mb-3 text-md text-coolGray-800 group-hover:text-coolGray-900 font-semibold transition duration-200",
                                "{package.name}"
                            }

                            crate_description { package: package }

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
                        }
                        release_button
                    }
                }
            }
        }
    })
}

#[inline_props]
fn crate_description<'a>(cx: Scope<'a>, package: &'a Package) -> Element {
    // // todo: download the metadata from the crates index using reqwest/downloader
    // let url = format!(
    //     "https://raw.githubusercontent.com/rust-lang/crates.io-index/master/{}/{}/{}",
    //     package.name.chars().next().unwrap(),
    //     package.name.chars().nth(1).unwrap(),
    //     package.name
    // );

    // Required fields include name, version, edition, edition, keywords, description, license, authors, license, tags, description
    //
    // name = "dioxus-liveview"
    // version = "0.3.0"
    // edition = "2021"
    // keywords = ["dom", "ui", "gui", "react", "wasm"]
    // description = "Build server-side apps with Dioxus"
    // license = "MIT/Apache-2.0"
    //
    // homepage = "https://dioxuslabs.com"
    // documentation = "https://dioxuslabs.com"
    // repository = "https://github.com/DioxusLabs/dioxus/"
    cx.render(rsx! {
        div { class: "text-coolGray-500 font-sm flex flex-col",
            // todo: throw an error if the version here matches the same version on crates, since
            // crates will reject that version
            span { "Version: " package.version.to_string() }
            span { "Edition: " package.edition.to_string() }
            span { "Keywords: " package.keywords.iter().map(|k| rsx!( "{k}, " )) }
            span { "License: " package.license.as_deref().unwrap_or("❌ missing") }
            span { "Description: " package.description.as_deref().unwrap_or("❌ missing") }
        }
    })
}

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
