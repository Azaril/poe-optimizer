//! Map actual FNEW edges to complete lexical function expressions.
use super::*;
use capture::ObservedSourceClosureCreations;
use poe_optimizer_data::source_program::*;
#[derive(Debug, Clone)]
pub(crate) struct BoundFunction {
    pub node: capture::closures::Node,
    pub provenance: SourceProgramProvenance,
    /// Offset in this function -> child callback and exact FNEW PC.
    pub children: BTreeMap<u32, (SourceCallbackId, u32)>,
}
pub fn lower_observed_closures_from_sources(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
    observations: &ObservedSourceClosureCreations,
) -> Result<SourceProgramExtraction> {
    if !owner.is_same_owner(&observations.owner) {
        return Err(error("closure observations belong to another owner"));
    }
    validate_sources(sources, owner)?;
    let bindings = bind_functions(sources, observations)?;
    lower_inner(sources, owner, Some((&bindings, &observations.profile)))
}
pub fn lower_observed_closures_and_constructors_from_sources(
    sources: &BTreeMap<String, String>,
    owner: &SourceProgramOwner,
    closures: &ObservedSourceClosureCreations,
    constructors: &capture::ObservedSourceConstructors,
) -> Result<SourceProgramExtraction> {
    let lowered = lower_observed_closures_from_sources(sources, owner, closures)?;
    super::constructors::attach(sources, owner, constructors, lowered)
}
fn bind_functions(
    sources: &BTreeMap<String, String>,
    observations: &ObservedSourceClosureCreations,
) -> Result<BTreeMap<SourceCallbackId, BoundFunction>> {
    let children = observations
        .nodes
        .values()
        .flat_map(|node| {
            node.children
                .values()
                .map(|id| SourceCallbackId(*id as u32))
        })
        .collect::<std::collections::BTreeSet<_>>();
    let mut bound = BTreeMap::new();
    for (id, node) in &observations.nodes {
        if children.contains(id) {
            continue;
        }
        let text = &sources[&node.source.path];
        let body = span_body(text, &node.source);
        let tokens = lowering::lex(body, &mut Budget::default()).map_err(error)?;
        let mut candidates = Vec::new();
        let mut index = 0;
        while index < tokens.len() {
            if !tokens[index].quoted && tokens[index].text == "function" {
                let end = lowering::complete_function_token_end(&tokens, index).map_err(error)?;
                let first = line_at(body, tokens[index].start, node.source.line);
                let last = line_at(body, tokens[end - 1].end - 1, node.source.line);
                if first == node.source.line && last == node.source.end_line {
                    candidates.push((tokens[index].start, tokens[end - 1].end));
                }
                index = end;
            } else {
                index += 1;
            }
        }
        if candidates.len() != 1 {
            continue;
        } // Explicit per-body frontier below.
        let (start, end) = candidates[0];
        bind_one(*id, start, end, sources, observations, &mut bound, 0)?;
    }
    Ok(bound)
}
fn span_body<'a>(
    text: &'a str,
    span: &poe_optimizer_data::item_loading::ItemSourceSpan,
) -> &'a str {
    // Inventory/span validation and actual prototype line bounds run first.
    capture::closures::source_slice(text, span.line, span.end_line).expect("validated source span")
}
fn line_at(body: &str, offset: usize, base: u32) -> u32 {
    base + body.as_bytes()[..offset]
        .iter()
        .filter(|b| **b == b'\n')
        .count() as u32
}
#[allow(clippy::too_many_arguments)]
fn bind_one(
    id: SourceCallbackId,
    start: usize,
    end: usize,
    sources: &BTreeMap<String, String>,
    observations: &ObservedSourceClosureCreations,
    bound: &mut BTreeMap<SourceCallbackId, BoundFunction>,
    depth: usize,
) -> Result<()> {
    if depth > 32 {
        return Err(error("closure lexical graph depth bound"));
    }
    let node = &observations.nodes[&id];
    let body = span_body(&sources[&node.source.path], &node.source);
    let function = body
        .get(start..end)
        .ok_or_else(|| error("closure expression source range"))?;
    let tokens = lowering::lex(function, &mut Budget::default()).map_err(error)?;
    let mut expressions = Vec::new();
    let mut index = 1;
    while index < tokens.len() {
        if !tokens[index].quoted && tokens[index].text == "function" {
            let end = lowering::complete_function_token_end(&tokens, index).map_err(error)?;
            expressions.push((tokens[index].start, tokens[end - 1].end));
            index = end;
        } else {
            index += 1;
        }
    }
    let instructions = node
        .metadata
        .instructions
        .iter()
        .enumerate()
        .filter(|(_, word)| **word & 255 == 51)
        .collect::<Vec<_>>();
    if expressions.len() != instructions.len() {
        return Err(error("complete source/FNEW child inventory mismatch"));
    }
    let mut children = BTreeMap::new();
    for ((child_start, child_end), (pc, instruction)) in expressions.into_iter().zip(instructions) {
        let child = SourceCallbackId(node.children[&(-1 - ((*instruction >> 16) as i32))] as u32);
        let child_node = &observations.nodes[&child];
        if line_at(function, child_start, node.source.line) != child_node.source.line
            || line_at(function, child_end - 1, node.source.line) != child_node.source.end_line
        {
            return Err(error("FNEW child source occurrence mismatch"));
        }
        let parent_absolute = sources[&node.source.path]
            .split_inclusive('\n')
            .take(node.source.line as usize - 1)
            .map(str::len)
            .sum::<usize>();
        let child_absolute = sources[&node.source.path]
            .split_inclusive('\n')
            .take(child_node.source.line as usize - 1)
            .map(str::len)
            .sum::<usize>();
        bind_one(
            child,
            parent_absolute + start + child_start - child_absolute,
            parent_absolute + start + child_end - child_absolute,
            sources,
            observations,
            bound,
            depth + 1,
        )?;
        children.insert(child_start as u32, (child, pc as u32 + 1));
    }
    let provenance = SourceProgramProvenance {
        source: node.source.clone(),
        function_start: start as u32,
        function_end: end as u32,
        function_sha256: hash(function.as_bytes()),
    };
    if let Some(old) = bound.get(&id)
        && old.provenance != provenance
    {
        return Err(error(
            "one actual prototype has ambiguous lexical occurrences",
        ));
    }
    bound.insert(
        id,
        BoundFunction {
            node: node.clone(),
            provenance,
            children,
        },
    );
    Ok(())
}
