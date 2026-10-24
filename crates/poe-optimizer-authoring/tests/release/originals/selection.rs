//! Test-only saved selection: source selectors resolve through exact typed origins.
use poe_optimizer_core::{owned_draft::EvaluationSelection, owned_project::VariantSelection};
use poe_optimizer_import::{
    build_instance::SourceOccurrenceId,
    owned_normalize::{FreshNormalizationSidecar, NormalizedImport, OwnedOriginTarget},
    owned_source::{SourceEvidenceRow, SourceProjectEvidence},
};
use std::collections::BTreeSet;

fn only<T>(mut values: impl Iterator<Item = T>, what: &str) -> T {
    let value = values.next().unwrap_or_else(|| panic!("missing {what}"));
    assert!(values.next().is_none(), "ambiguous {what}");
    value
}

fn attr<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    row.attribute(name).unwrap().decoded().unwrap()
}

fn selected<'a, 's>(
    evidence: &'a SourceProjectEvidence<'s>,
    section: &str,
    child: &str,
    selector: &str,
) -> &'a SourceEvidenceRow<'s> {
    let owner = only(
        evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == section),
        section,
    );
    let wanted = attr(owner, selector);
    only(
        evidence.rows().iter().filter(|row| {
            row.occurrence().parent() == Some(owner.occurrence().id())
                && row.occurrence().name() == child
                && attr(row, "id") == wanted
        }),
        selector,
    )
}

fn link<T>(
    sidecar: &FreshNormalizationSidecar,
    source: SourceOccurrenceId,
    select: impl Fn(&OwnedOriginTarget) -> Option<T>,
    kind: &str,
) -> T {
    let origin = only(
        sidecar
            .origins
            .iter()
            .filter(|origin| origin.source == source),
        "selected source origin",
    );
    only(origin.links.iter().filter_map(select), kind)
}

pub(crate) fn selected_spec<'a, 's>(
    evidence: &'a SourceProjectEvidence<'s>,
) -> &'a SourceEvidenceRow<'s> {
    let tree = only(
        evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "Tree"),
        "Tree",
    );
    // The source stores a one-based Spec position, unlike the explicit IDs above.
    let spec_index = attr(tree, "activeSpec")
        .parse::<usize>()
        .unwrap()
        .checked_sub(1)
        .expect("activeSpec must be one-based");
    evidence
        .rows()
        .iter()
        .filter(|row| {
            row.occurrence().parent() == Some(tree.occurrence().id())
                && row.occurrence().name() == "Spec"
        })
        .nth(spec_index)
        .expect("selected Spec must exist")
}

pub(crate) fn selected_skill_set<'a, 's>(
    evidence: &'a SourceProjectEvidence<'s>,
) -> &'a SourceEvidenceRow<'s> {
    selected(evidence, "Skills", "SkillSet", "activeSkillSet")
}

pub(crate) fn saved(
    evidence: &SourceProjectEvidence<'_>,
    normalized: &NormalizedImport,
) -> EvaluationSelection {
    let items = selected(evidence, "Items", "ItemSet", "activeItemSet");
    let skills = selected_skill_set(evidence);
    let config = selected(evidence, "Config", "ConfigSet", "activeConfigSet");
    let spec = selected_spec(evidence);
    let loadout = match attr(items, "useSecondWeaponSet") {
        "true" => "weapon-set-two",
        "false" | "nil" => "weapon-set-one",
        value => panic!("unreviewed saved weapon-set flag: {value}"),
    };
    let sidecar = normalized.sidecar();
    let loadouts: BTreeSet<_> = sidecar
        .origins
        .iter()
        .flat_map(|origin| &origin.links)
        .filter_map(|link| match link {
            OwnedOriginTarget::WeaponLoadout { key, id } if key.as_str() == loadout => Some(*id),
            _ => None,
        })
        .collect();
    let active_weapon_loadout = only(loadouts.into_iter(), "selected weapon loadout");
    macro_rules! preset {
        ($row:expr, $variant:ident) => {
            link(
                sidecar,
                $row.occurrence().id(),
                |target| match target {
                    OwnedOriginTarget::$variant(id) => Some(*id),
                    _ => None,
                },
                stringify!($variant),
            )
        };
    }
    let queries = only(
        normalized.draft().input().query_presets.members.iter(),
        "query preset",
    );
    EvaluationSelection {
        build: VariantSelection {
            character: preset!(spec, CharacterPreset),
            equipment: preset!(items, EquipmentPreset),
            allocations: preset!(spec, AllocationPreset),
            skills: preset!(skills, SkillPreset),
            choices: preset!(config, ChoicePreset),
            active_weapon_loadout,
        },
        scenario: preset!(config, ScenarioPreset),
        queries: queries.id,
    }
}
