//! Semantic validation (blueprint §13). Shared helpers live here; the
//! module-specific passes are in submodules.

pub(crate) mod flow;
pub(crate) mod mindmap;

use std::collections::BTreeMap;

use crate::ast::Spanned;
use crate::diagnostics::{codes, Diagnostic, Diagnostics, Origin};
use crate::source::Span;

/// Built-in class names that user classes may not reuse (decision D4).
pub(crate) const RESERVED_CLASSES: [&str; 11] = [
    "process",
    "decision",
    "event",
    "input",
    "output",
    "database",
    "subprocess",
    "start",
    "end",
    "yes",
    "no",
];

/// Records declared IDs and checks class names, in the order called
/// (callers walk the document in source order).
#[derive(Debug, Default)]
pub(crate) struct IdClassChecker {
    ids: BTreeMap<String, Span>,
}

impl IdClassChecker {
    pub(crate) fn new() -> IdClassChecker {
        IdClassChecker::default()
    }

    /// Checks one declaration's optional ID and its classes.
    pub(crate) fn check(
        &mut self,
        id: Option<&Spanned<String>>,
        classes: &[Spanned<String>],
        diags: &mut Diagnostics,
    ) {
        if let Some(id) = id {
            self.check_id(id, diags);
        }
        for class in classes {
            check_class(class, diags);
        }
    }

    fn check_id(&mut self, id: &Spanned<String>, diags: &mut Diagnostics) {
        match self.ids.get(&id.value) {
            Some(first) => diags.push(
                Diagnostic::error(
                    codes::SEM_DUPLICATE_ID,
                    Origin::Source,
                    id.span,
                    format!("Duplicate ID `{}`.", id.value),
                )
                .with_help("IDs must be unique within a document; rename one of them.")
                .with_related(*first, "first declared"),
            ),
            None => {
                self.ids.insert(id.value.clone(), id.span);
            }
        }
    }
}

fn check_class(class: &Spanned<String>, diags: &mut Diagnostics) {
    if RESERVED_CLASSES.contains(&class.value.as_str()) {
        diags.push(
            Diagnostic::error(
                codes::SEM_RESERVED_CLASS,
                Origin::Source,
                class.span,
                format!("Class `{}` uses a reserved built-in name.", class.value),
            )
            .with_help("Choose a different class name; built-in names are reserved."),
        );
    }
}
