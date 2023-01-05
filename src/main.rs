use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use cargo_metadata::{DependencyKind, Metadata, PackageId};
use cargo_util::crate_root;
use clap::Parser;
use dioxus::prelude::*;
use dioxus_desktop::Config;

mod cargo_util;
mod templates;

fn main() {
    use cargo_metadata::MetadataCommand;
    let args = Args::try_parse().unwrap();

    let mut cmd = MetadataCommand::new();

    if let Some(path) = args.path {
        cmd.manifest_path(path.join("Cargo.toml"));
    };

    let res = cmd.exec().unwrap();

    dioxus_desktop::launch_with_props(
        app,
        AppProps(res),
        Config::new().with_custom_head(
            r#"
<script src="https://cdn.tailwindcss.com"></script>
"#
            .to_string(),
        ),
    );
}

struct AppProps(Metadata);

fn app(cx: Scope<AppProps>) -> Element {
    let graph = &*cx.use_hook(|| CrateGraph::new(&cx.props.0));

    let mut released_crates = use_state(cx, || HashSet::<PackageId>::new());
    let mut ignored_crates = use_state(cx, || HashSet::<PackageId>::new());
    let mut unrelease_crates = use_state(cx, || graph.crates.clone());

    let mut render_graph = use_render_graph(cx, graph);

    cx.render(rsx! {
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
                    div { class: "w-1/2",
                        h3 { class: "mb-4 text-md leading-tight font-medium text-coolGray-900 font-bold tracking-tighter",
                            "Unreleased Crates"
                        }
                        render_graph.iter().filter_map(|(id, _)| {
                            let package = cx.props.0.packages.iter().find(|p| p.id == *id).unwrap();

                            if !unrelease_crates.contains(&id) {
                                return None;
                            }


                            cx.render(rsx! {
                                row_item {
                                    name: package.name.clone(),
                                    graph: graph,
                                    meta: &cx.props.0,
                                    description: package.description.clone().unwrap_or_default(),
                                    package: package.id.clone(),
                                    released_crates: released_crates.clone(),
                                    unrelease_crates: unrelease_crates.clone(),
                                    ignored_crates: ignored_crates.clone(),
                                }
                            })
                        })
                    }

                    div { class: " w-1/2",
                        h3 { class: "mb-4 text-md leading-tight font-medium text-coolGray-900 font-bold tracking-tighter",
                            "Released Crates"
                        }
                        released_crates.iter().filter_map(|id| {
                            let package = cx.props.0.packages.iter().find(|p| p.id == *id).unwrap();

                            cx.render(rsx! {
                                row_item {
                                    name: package.name.clone(),
                                    graph: graph,
                                    meta: &cx.props.0,
                                    description: package.description.clone().unwrap_or_default(),
                                    package: package.id.clone(),
                                    released_crates: released_crates.clone(),
                                    unrelease_crates: unrelease_crates.clone(),
                                    ignored_crates: ignored_crates.clone(),
                                }
                            })
                        }),


                        (!ignored_crates.is_empty()).then(|| rsx! {
                            div {
                                class: "w-full border-t border-coolGray-200 mt-4 text-coolGray-500 text-center",
                                "ignored"
                            }
                        })

                        ignored_crates.iter().filter_map(|id| {
                            let package = cx.props.0.packages.iter().find(|p| p.id == *id).unwrap();

                            cx.render(rsx! {
                                row_item {
                                    name: package.name.clone(),
                                    graph: graph,
                                    meta: &cx.props.0,
                                    description: package.description.clone().unwrap_or_default(),
                                    package: package.id.clone(),
                                    released_crates: released_crates.clone(),
                                    unrelease_crates: unrelease_crates.clone(),
                                    ignored_crates: ignored_crates.clone(),
                                }
                            })
                        }),

                        // row_item { name: "asdasdasd".to_string(), description: "asdasdasd".to_string() }
                        // row_item { name: "asdasdasd".to_string(), description: "asdasdasd".to_string() }
                        // row_item { name: "asdasdasd".to_string(), description: "asdasdasd".to_string() }
                        div {}
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
                // let package = cx.props.0.packages.iter().find(|p| p.id == *id).unwrap();

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
    name: String,
    graph: &'a CrateGraph,
    meta: &'a Metadata,
    description: String,
    package: PackageId,
    released_crates: UseState<HashSet<PackageId>>,
    unrelease_crates: UseState<HashSet<PackageId>>,
    ignored_crates: UseState<HashSet<PackageId>>,
) -> Element {
    let release_button = unrelease_crates.contains(package).then(|| rsx! {
        div { class: "w-full lg:w-1/3 px-4 lg:text-right",
            button {
                class: "inline-flex ml-auto items-center font-medium leading-6 text-green-500 group-hover:text-green-600 transition duration-200",
                onclick: move |_| {
                    ignored_crates.make_mut().insert(package.clone());
                    unrelease_crates.make_mut().remove(&package);
                },
                span { class: "mr-2", "Ignore" }
                templates::icons::icon_0 {}
            }
            button {
                class: "inline-flex ml-auto items-center font-medium leading-6 text-green-500 group-hover:text-green-600 transition duration-200 ",
                onclick: move |_| {
                    released_crates.make_mut().insert(package.clone());
                    unrelease_crates.make_mut().remove(&package);
                },
                span { class: "mr-2", "Release" }
                templates::icons::icon_0 {}
            }
        }
    });

    cx.render(rsx! {
        div { class: "w-full px-4 mb-8",
            a { class: "group", href: "#",
                div { class: "bg-coolGray-50 group-hover:bg-coolGray-100 rounded-md shadow-md transition duration-200",
                    div { class: "flex flex-wrap items-start justify-between p-2 -mx-4",
                        div { class: "w-full lg:w-2/3 px-4 mb-6 lg:mb-0",
                            h3 { class: "mb-3 text-md text-coolGray-800 group-hover:text-coolGray-900 font-semibold transition duration-200",
                                "{name}"
                            }
                            p { class: "text-coolGray-500 font-sm", "{description}" }
                            ul {
                                graph.ws_deps[package].iter().map(|f| {
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

// cx.render(rsx! {
//     div {
//         padding_bottom: "1em",
//         span {
//             "{package.name}"
//             button {
//                 onclick: move |_| {
//                     released_crates.make_mut().insert(id.clone());
//                     unrelease_crates.make_mut().remove(id);
//                 },
//                 "Release",
//             }
//             button {
//                 onclick: move |_| {
//                     ignored_crates.make_mut().insert(id.clone());
//                     unrelease_crates.make_mut().remove(id);
//                 },
//                 "Ignore",
//             }
//         }

//         div {
//             margin_left: "1em",
//             graph.ws_deps[id].iter().map(|f| {
//                 let package = cx.props.0.packages.iter().find(|p| p.id == *f).unwrap();
//                 let emoij = if released_crates.contains(f) {
//                     "👍"
//                 } else {
//                     "👎"
//                 };

//                 cx.render(rsx! {
//                     div {
//                         "{emoij}" "{package.name}"
//                     }
//                 })
//             })
//         }
//     }
// })

#[inline_props]
fn member_child<'a>(cx: Scope<'a>, meta: &'a Metadata, package: PackageId) -> Element {
    let package = meta.packages.iter().find(|p| p.id == *package).unwrap();

    // todo: fix this
    let url = format!(
        "https://raw.githubusercontent.com/rust-lang/crates.io-index/master/{}/{}/{}",
        package.name.chars().next().unwrap(),
        package.name.chars().nth(1).unwrap(),
        package.name
    );

    cx.render(rsx! {
        div { a { href: "{url}", prevent_default: "click", onclick: move |_| webbrowser::open(&url).unwrap(), "{package.name}" } }
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
