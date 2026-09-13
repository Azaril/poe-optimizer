//! Post-ParseRaw radius writes preserve owned aliases and reached failure prefixes.
use super::value::{Arena, TableId};
use super::{AssembledItem, AssemblyAttempt, AssemblyError, AssemblyLimits, Value};
use crate::item_loading::AssemblyRequest;
use poe_optimizer_data::item_loading::{ItemLoadingCatalog, JewelRadiusPolicy};

fn reject(error: AssemblyError) -> AssemblyAttempt {
    AssemblyAttempt {
        result: Err(error),
        partial: None,
        stage: "jewel radius",
    }
}
fn member(arena: &mut Arena, value: Value, key: &str) -> Result<Value, AssemblyError> {
    match value {
        Value::Table(t) => arena.get_field(t, key),
        Value::Text(_) => Err(AssemblyError::unsupported(
            "jewel data string-method lookup is not represented",
        )),
        _ => Err(AssemblyError::source(
            "attempt to index non-table jewel data",
        )),
    }
}
fn apply(
    arena: &mut Arena,
    root: TableId,
    p: &JewelRadiusPolicy,
    defer: bool,
) -> Result<(), AssemblyError> {
    if defer {
        let data = arena.get_field(root, &p.item_data_field)?;
        let index = member(arena, data, &p.deferred_index_field)?;
        arena.set_field(root, &p.item_index_field, index)?;
    }
    let data = arena.get_field(root, &p.item_data_field)?;
    if data.truthy() {
        let index = member(arena, data, &p.override_field)?;
        if index.truthy() {
            arena.set_field(root, &p.item_index_field, index)?;
        }
    }
    Ok(())
}
pub(crate) fn finish_jewel_radius(
    item: &AssembledItem,
    p: &JewelRadiusPolicy,
    defer: bool,
) -> AssemblyAttempt {
    let Some(binding) = item.binding().filter(|_| item.is_complete()).cloned() else {
        return reject(AssemblyError::unsupported(
            "radius continuation requires a complete owned assembly",
        ));
    };
    let mut arena = match Arena::from_item(item, AssemblyLimits::default()) {
        Ok(a) => a,
        Err(e) => return reject(e),
    };
    let root = item.root();
    match apply(&mut arena, root, p, defer) {
        Ok(()) => AssemblyAttempt {
            result: arena.finish_complete(root, binding),
            partial: None,
            stage: "jewel radius",
        },
        Err(e) => AssemblyAttempt {
            result: Err(e),
            partial: arena.finish_partial(root, binding).ok(),
            stage: "jewel radius",
        },
    }
}

// BuildModList returns immediately without a base, but ParseRaw still executes
// its radius tail. Refresh ParseRaw's fields before that tail; do not run any
// assembly queries or promote an earlier failed assembly to a complete result.
pub(crate) fn finish_no_base_jewel_radius(
    catalog: &ItemLoadingCatalog,
    request: &AssemblyRequest,
    defer: bool,
) -> AssemblyAttempt {
    if request.state.base_present || !request.binding().matches_definitions(catalog) {
        return reject(AssemblyError::unsupported(
            "NoBase radius request has a base or a different definition owner",
        ));
    }
    let previous = request.previous.as_ref();
    if previous.is_some_and(|p| p.binding().is_none_or(|b| !b.same_item(request.binding()))) {
        return reject(AssemblyError::unsupported(
            "NoBase radius graph belongs to another item or definition owner",
        ));
    }
    let initialized = if let Some(item) = previous {
        Arena::from_item(item, AssemblyLimits::default()).map(|a| (a, item.root()))
    } else {
        let mut arena = Arena::new(AssemblyLimits::default());
        arena.new_table().map(|root| (arena, root))
    };
    let (mut arena, root) = match initialized {
        Ok(v) => v,
        Err(e) => return reject(e),
    };
    // With a retained graph the cluster field already has its exact identity.
    // Fresh caller cluster metadata needs the assembly policy, which this tail
    // does not own; hydration explicitly refuses that otherwise unknown field.
    if let Err(e) =
        super::hydrate::hydrate(&mut arena, root, request, catalog, None, previous.is_some())
    {
        return reject(e); // Incomplete input hydration is not a source prefix.
    }
    let binding = request.binding().clone();
    match apply(&mut arena, root, &catalog.policy().jewel_radius, defer) {
        Ok(()) => AssemblyAttempt {
            result: arena.finish_partial(root, binding),
            partial: None,
            stage: "jewel radius",
        },
        Err(e) => AssemblyAttempt {
            result: Err(e),
            partial: arena.finish_partial(root, binding).ok(),
            stage: "jewel radius",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_loading::{
        AssemblyExecution, DependencyResult, ItemLoadMachine, ItemLoadProvider, ParseOutcome,
        ParseRequest,
    };
    use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
    use std::sync::OnceLock;
    fn data() -> &'static GameDataSnapshot {
        static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
        DATA.get_or_init(|| bundled_snapshot().unwrap())
    }
    fn request() -> AssemblyRequest {
        #[derive(Default)]
        struct Capture(Option<AssemblyRequest>);
        impl ItemLoadProvider for Capture {
            fn parse_modifier(&mut self, _: &ParseRequest) -> DependencyResult<ParseOutcome> {
                DependencyResult::Available(ParseOutcome {
                    modifiers: Some(Vec::new()),
                    extra: None,
                })
            }
            fn assemble_with_trace(&mut self, r: &AssemblyRequest) -> AssemblyExecution {
                self.0 = Some(r.clone());
                AssemblyExecution {
                    outcome: DependencyResult::Unavailable("capture".into()),
                    prefix: None,
                }
            }
        }
        let mut m = ItemLoadMachine::new(data().item_loading());
        let mut p = Capture::default();
        m.apply_text("Rarity: Normal\nRuby\nImplicits: 0", &mut p)
            .unwrap();
        let mut r = p.0.unwrap();
        r.state.base_present = false;
        r.state.name = "unknown base".into();
        r.reparsed = true;
        r
    }
    #[test]
    fn fresh_no_base_variable_reports_known_missing_data_after_current_hydration() {
        let r = request();
        let attempt = finish_no_base_jewel_radius(data().item_loading(), &r, true);
        assert_eq!(
            attempt.result.unwrap_err().kind,
            super::super::AssemblyErrorKind::Source
        );
        let prefix = attempt.partial.unwrap();
        assert!(!prefix.is_complete());
        assert!(prefix.binding().unwrap().matches_attempt(r.binding()));
        assert!(prefix.field(prefix.root(), "base").is_none());
        assert_eq!(
            prefix.field(prefix.root(), "name").unwrap().as_str(),
            Some("unknown base")
        );
        assert!(
            prefix
                .field(prefix.root(), "requirements")
                .unwrap()
                .as_table()
                .is_some()
        );
    }
    #[test]
    fn no_base_uses_partial_prior_graph_without_certifying_it_or_changing_aliases() {
        let mut r = request();
        let mut arena = Arena::new(AssemblyLimits::default());
        let root = arena.new_table().unwrap();
        let jewel = arena.new_table().unwrap();
        let index = arena.new_table().unwrap();
        arena
            .set_field(index, "marker", Value::Number(9.0))
            .unwrap();
        arena
            .set_field(jewel, "radiusIndex", Value::Table(index))
            .unwrap();
        arena
            .set_field(root, "jewelData", Value::Table(jewel))
            .unwrap();
        arena.set_field(root, "base", Value::Table(index)).unwrap();
        r.previous = Some(arena.finish_partial(root, r.binding().clone()).unwrap());
        let out = finish_no_base_jewel_radius(data().item_loading(), &r, true)
            .result
            .unwrap();
        assert!(!out.is_complete());
        assert_eq!(
            out.field(root, "jewelRadiusIndex"),
            Some(&Value::Table(index))
        );
        assert_eq!(out.field(root, "jewelData"), Some(&Value::Table(jewel)));
        assert!(out.field(root, "base").is_none());
        assert_eq!(
            r.previous.as_ref().unwrap().field(root, "base"),
            Some(&Value::Table(index))
        );
        assert!(
            finish_jewel_radius(&out, &data().item_loading().policy().jewel_radius, true)
                .result
                .is_err()
        );
        let mut foreign = request();
        foreign.previous = Some(out);
        let rejected = finish_no_base_jewel_radius(data().item_loading(), &foreign, true);
        assert!(
            rejected
                .result
                .unwrap_err()
                .message
                .contains("another item")
        );
        assert!(rejected.partial.is_none());
    }
}
