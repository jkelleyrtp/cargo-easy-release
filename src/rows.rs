use crate::state::CrateGraph;
use crate::templates;
use cargo_metadata::{
    camino::Utf8PathBuf,
    semver::{Version, VersionReq},
    DependencyKind, Metadata, Package, PackageId,
};
use dioxus::prelude::*;
use dioxus_signals::{use_signal, Signal};
use fermi::use_read;
use tokio::process::Command;

#[derive(Props)]
pub struct RowProps<'a> {
    graph: &'a CrateGraph,
    id: PackageId,
}

pub fn RowItem<'a>(cx: Scope<'a, RowProps<'a>>) -> Element {
    let RowProps { graph, id } = cx.props;
    let meta = &graph.meta;

    let manifest_path = use_signal(cx, || get_manifest_path(&graph.meta, id));
    let allow_dirty = use_read(cx, crate::state::ALLOW_DIRTY).clone();
    let dry_run = use_read(cx, crate::state::DRY_RUN).clone();
    let release_crate = move || run_release(manifest_path, allow_dirty, dry_run);
    let package = graph.get_crate(id);

    // Running a minor patch implies moving every transitive dep to the next minor version of this crate.
    // So bumping a "core" crate would involve bumping all the crates that depend on it too.
    let minor_patch = |_| {
        //
    };

    cx.render(rsx! {
        div { class: "p-2 w-full mb-8 h-100 bg-gray-50 group-hover:bg-gray-100 rounded-md shadow-md transition duration-200 flex flex-col justify-between",
            div { class: "w-full",
                h3 { class: "text-md mb-2 text-gray-800 group-hover:text-gray-900 font-semibold transition duration-200 font-mono flex flex-row justify-between",
                    div { class: "flex flex-row",
                        span { "{package.name}" }
                        span { class: "text-gray-500 ml-2", "{package.version}" }
                    }
                    div {
                        button {
                            class: "inline-flex ml-auto items-center font-medium leading-6 group-hover:text-green-600 transition duration-200 ",
                            onclick: move |_| {},
                            span { class: "mr-2", "Patch" }
                        }
                        button {
                            class: "inline-flex ml-auto items-center font-medium leading-6 group-hover:text-green-600 transition duration-200 ",
                            onclick: minor_patch,
                            span { class: "mr-2", "Minor" }
                        }

                        button {
                            class: "inline-flex ml-auto items-center font-medium leading-6 text-green-500 group-hover:text-green-600 transition duration-200 ",
                            onclick: move |_| cx.spawn(release_crate()),
                            span { class: "mr-2", "Release" }
                        }
                    }
                }
                div { class: "flex flex-row justify-between",
                    PackageChecklist { package: package }
                    CrateDeps { graph: graph, meta: meta, id: id.clone() }
                }
            }
        }
    })
}

async fn run_release(manifest_path: Signal<Utf8PathBuf>, allow_dirty: bool, dry_run: bool) {
    let mut cmd = Command::new("cargo");

    cmd.arg("publish")
        .arg("--manifest-path")
        .arg(&*manifest_path.get());

    if allow_dirty {
        cmd.arg("--allow-dirty");
    }

    if dry_run {
        cmd.arg("--dry-run");
    }

    cmd.spawn().unwrap().wait().await.unwrap();
}

#[inline_props]
fn PackageChecklist<'a>(cx: Scope<'a>, package: &'a Package) -> Element {
    render! {
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

            div {
                if let Some(description) =  package.description.as_deref() {
                    render! { "✅ ", description }
                } else {
                    render! { "❌ Missing description" }
                }
            }
        }
    }
}

fn package_has_local_deps(package: &cargo_metadata::Package) -> bool {
    let package_has_local_deps = package.dependencies.iter().any(|dep| {
        // If the source is a git repo, then it's a local dep
        if dep
            .source
            .as_ref()
            .map(|s| s.starts_with("git+"))
            .unwrap_or(false)
        {
            return true;
        }

        // If the source is a path with no version, then it's a local dep
        dep.path.is_some() && dep.kind == DependencyKind::Normal && dep.req.comparators.is_empty()
    });

    package_has_local_deps
}

fn collect_api_diff(id: &PackageId, graph: &CrateGraph) -> String {
    use public_api::Options;
    let id = id.to_string();
    let crate_name = id.split_ascii_whitespace().next().unwrap();
    let path = graph
        .meta
        .target_directory
        .to_path_buf()
        .join("doc")
        .join(format!("{crate_name}.json"));
    let maybe = || {
        let new =
            public_api::PublicApi::from_rustdoc_json(path.clone(), Options::default()).ok()?;

        let old =
            public_api::PublicApi::from_rustdoc_json(path.clone(), Options::default()).ok()?;

        Some(format!(
            "{:#?}",
            public_api::diff::PublicApiDiff::between(old, new)
        ))
    };
    maybe().unwrap_or_default()
}

#[inline_props]
fn CrateDeps<'a>(
    cx: Scope<'a>,
    graph: &'a CrateGraph,
    meta: &'a Metadata,
    id: PackageId,
) -> Element {
    let deps = use_state(cx, || collect_package_versions_from_manifest(id, graph));

    render! {
        ul { class: "text-xs text-gray-500 text-right",

            deps.iter().map(|(name, left, right)| {
                let mut color = "text-green-500";

                // If the version is different, then it's not published and invalid
                if !right.matches(&left) {
                    color = "text-yellow-500";
                }

                // If it's a local dep, then it's not published and invalid
                if right.to_string() == "*" {
                    color = "text-red-500";
                }

                rsx! {
                    li { "{name} " span { class: "{color} pl-2", "({right})" } }
                }
            })
        }
    }
}

fn collect_package_versions_from_manifest(
    id: &PackageId,
    graph: &CrateGraph,
) -> Vec<(String, Version, VersionReq)> {
    let mut out = graph.ws_deps[id]
        .iter()
        .map(|dep_id| {
            let dep = graph.get_crate(dep_id);
            let name = dep.name.clone();

            let left = dep.version.clone();
            let right = graph
                .get_crate(id)
                .dependencies
                .iter()
                .find(|toml_dep| toml_dep.name == name)
                .map(|toml_dep| toml_dep.req.clone())
                .unwrap();

            (name, left, right)
        })
        .collect::<Vec<_>>();

    out.sort_by_key(|(_, _, right)| right.to_string());

    out
}

fn get_manifest_path(meta: &Metadata, id: &PackageId) -> Utf8PathBuf {
    meta.packages
        .iter()
        .find(|p| &p.id == id)
        .unwrap()
        .manifest_path
        .clone()
}
