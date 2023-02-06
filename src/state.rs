use cargo_metadata::{DependencyKind, Metadata, PackageId};
use clap::Parser;
use dioxus::prelude::*;
use fermi::Atom;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

pub static ALLOW_DIRTY: Atom<bool> = |_| false;
pub static DRY_RUN: Atom<bool> = |_| true;

pub fn collect_workspace_meta() -> Metadata {
    use cargo_metadata::MetadataCommand;

    #[derive(clap::Parser)]
    struct Args {
        path: Option<PathBuf>,
    }

    let args = Args::try_parse().unwrap();

    let mut cmd = MetadataCommand::new();

    if let Some(path) = args.path {
        cmd.manifest_path(path.join("Cargo.toml"));
    };

    cmd.exec().unwrap()
}

fn build_crate_graph(graph: &CrateGraph) -> Vec<(PackageId, usize)> {
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

    // first, sort alphebetically. This is so that the crates with the same number of dependencies are sorted
    render_graph.sort_by_key(|(id, _)| id.to_string());

    // then sort by the number of dependencies, keeping the alphabetical order
    render_graph.sort_by_key(|(_, num_deps)| *num_deps);

    render_graph
}

pub struct CrateGraph {
    pub meta: Metadata,

    pub crates: HashSet<PackageId>,

    // Neighbors within the workspace
    pub ws_deps: HashMap<PackageId, HashSet<PackageId>>,

    pub sorted: Vec<(PackageId, usize)>,
}

impl CrateGraph {
    pub fn new(meta: &Metadata) -> Self {
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

        let mut graph = Self {
            meta: meta.clone(),
            crates,
            ws_deps: deps,
            sorted: vec![],
        };

        graph.sorted = build_crate_graph(&graph);

        graph
    }

    pub fn get_crate(&self, id: &PackageId) -> &cargo_metadata::Package {
        self.meta.packages.iter().find(|p| p.id == *id).unwrap()
    }

    pub fn bump_minor(&self, id: &PackageId) {
        use toml_edit::{Document, Item};

        let package = self.get_crate(id);

        let mut version = package.version.clone();
        version.minor += 1;

        println!("Bumping {} to {}", package.name, version);

        let contents = std::fs::read_to_string(&package.manifest_path).unwrap();

        let mut doc = contents.parse::<Document>().expect("invalid doc");

        doc["package"]["version"] = Item::Value(version.to_string().into());

        // write back to fs
        std::fs::write(&package.manifest_path, doc.to_string()).unwrap();

        for dep in self.ws_deps[id].iter() {
            let dep = self.get_crate(dep);
            doc["dependencies"][&dep.name]["version"] =
                Item::Value(format!("^{}", version.to_string()).into());
            std::fs::write(&dep.manifest_path, doc.to_string()).unwrap();
        }
    }
}

fn write_crate_graph(
    deps: &HashMap<PackageId, HashSet<PackageId>>,
    meta: &Metadata,
    render_graph: &Vec<(PackageId, usize)>,
) {
    // add the workspace members to the graph
    let mut dag = VisualGraph::new(Orientation::TopToBottom);

    // Define the node styles:
    let look0 = StyleAttr::simple();
    let sz = Point::new(100., 100.);

    let mut mapping = HashMap::new();

    for (package, _) in render_graph.iter().rev() {
        let package_ = &meta.packages.iter().find(|p| p.id == *package).unwrap();

        let package_name = package_.name.as_str();
        let package_version = package_.version.to_string();

        if deps.contains_key(&package) && package_.publish == Some(vec![]) {
            continue;
        }

        let id = dag.add_node(Element::create(
            ShapeKind::new_box(&format!("{package_name}\nv{package_version}")),
            look0.clone(),
            Orientation::TopToBottom,
            sz,
        ));
        mapping.insert(package, id);
    }

    for (package, _) in render_graph {
        let Some(&id) = mapping.get(package) else { continue;};

        let deps = deps.get(package).unwrap();

        // sort the dependencies by the render graph
        let mut deps = deps.iter().collect::<Vec<_>>();
        deps.sort_by(|a, b| {
            let a = render_graph.iter().position(|(p, _)| p == *a).unwrap();
            let b = render_graph.iter().position(|(p, _)| p == *b).unwrap();
            a.cmp(&b)
        });

        for dep in deps {
            let Some(&dep_id) = mapping.get(dep) else { continue ;};

            if id == dep_id {
                continue;
            }

            let package_version = meta
                .packages
                .iter()
                .find(|p| p.id == *dep)
                .unwrap()
                .version
                .to_string();

            // Add an edge between the nodes.
            let arrow = Arrow::simple(&package_version);
            dag.add_edge(arrow, dep_id, id);
        }
    }

    use layout::backends::svg::SVGWriter;
    use layout::core::base::Orientation;
    use layout::core::geometry::Point;
    use layout::core::style::*;
    use layout::core::utils::save_to_file;
    use layout::std_shapes::shapes::*;
    use layout::topo::layout::VisualGraph;

    // Render the nodes to some rendering backend.
    let mut svg = SVGWriter::new();
    dag.do_it(false, false, false, &mut svg);

    // Save the output.
    let _ = save_to_file("graph.svg", &svg.finalize());
}
