//! Strict saved source selection shared by independent ownership adapters.
use super::source_shape::{charge_frame, container_text, plain_row, value};
use super::*;

pub(super) fn positive(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('0')
        && text.bytes().all(|b| b.is_ascii_digit())
        && text.parse::<u64>().is_ok_and(|n| n < (1_u64 << 53))
}

/// Return the actual container and the selected direct Spec source. This keeps
/// the existing generated-provider selector strict: no absent-value default,
/// out-of-range clamping, or interpretation of a Spec position as an ID.
pub(super) fn tree(
    b: &mut Builder<'_, '_>,
) -> Result<Option<(SourceOccurrenceId, SourceOccurrenceId)>> {
    let evidence = b.evidence;
    let root = &evidence.rows()[0];
    b.charge(root.children().len())?;
    let trees: Vec<_> = root
        .children()
        .iter()
        .map(|id| &evidence.rows()[id.ordinal() as usize])
        .filter(|row| matches!(row.occurrence().name(), "Tree" | "Spec"))
        .collect();
    if let [tree] = trees.as_slice() {
        charge_frame(b, tree, &["activeSpec"])?;
        if tree.occurrence().name() == "Tree"
            && plain_row(tree, &["activeSpec"], false)
            && container_text(tree)
            && let Some(index) = value(tree, "activeSpec")
                .filter(|v| positive(v))
                .and_then(|v| v.parse::<usize>().ok())
        {
            let mut valid = true;
            for id in tree.children() {
                let row = &evidence.rows()[id.ordinal() as usize];
                b.charge(1)?;
                valid &= row.occurrence().name() == "Spec"
                    && !row.occurrence().has_namespace_context()
                    && row.occurrence().parent() == Some(tree.occurrence().id())
                    && matches!(
                        row.authored_instance(),
                        Some(AuthoredInstanceId::PassiveSpec(_))
                    );
            }
            if valid {
                return Ok(tree
                    .children()
                    .get(index - 1)
                    .copied()
                    .map(|spec| (tree.occurrence().id(), spec)));
            }
        }
    }
    Ok(None)
}
