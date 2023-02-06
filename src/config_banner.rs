use crate::state::*;
use dioxus::prelude::*;
use fermi::use_atom_state;

pub fn GlobalConfig(cx: Scope) -> Element {
    let allow_dirty = use_atom_state(cx, ALLOW_DIRTY);
    let dry_run = use_atom_state(cx, DRY_RUN);

    render! {
        div { class: "border-b border-gray-200 mb-4 pb-2 flex flex-row justify-between",
            h3 { class: "text-md leading-tight font-medium text-gray-900 font-bold", "Workspace Crates (12)" }
            div { class: "flex flex-row",
                form {
                    input {
                        name: "allow-dirty",
                        r#type: "checkbox",
                        checked: "{allow_dirty}",
                        onchange: move |_| allow_dirty.set(!allow_dirty.get())
                    }
                    label { r#for: "allow-dirty", "Allow dirty?" }
                }
                form { class: "ml-4",
                    input {
                        r#type: "checkbox",
                        checked: "{dry_run}",
                        onchange: move |_| dry_run.set(!dry_run.get())
                    }
                    label { r#for: "dry-run", "Dry run?" }
                }
            }
        }
    }
}
