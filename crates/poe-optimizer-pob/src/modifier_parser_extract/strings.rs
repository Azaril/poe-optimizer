//! Authentication of the one closed byte-string helper and its original primitives.
use super::*;

pub(super) struct StringLibrary {
    table: Table,
    gsub: Function,
    upper: Function,
    get_metatable: Function,
    metatable: Table,
}
impl StringLibrary {
    pub(super) fn capture(lua: &Lua) -> Result<Self> {
        let table: Table = lua.globals().raw_get("string")?;
        let get_metatable: Function = lua.globals().raw_get("getmetatable")?;
        let metatable: Table = get_metatable.call("")?;
        let this = Self {
            gsub: table.raw_get("gsub")?,
            upper: table.raw_get("upper")?,
            table,
            get_metatable,
            metatable,
        };
        this.verify(lua)?;
        Ok(this)
    }
    pub(super) fn verify(&self, lua: &Lua) -> Result<()> {
        let actual: Table = lua.globals().raw_get("string")?;
        if actual.to_pointer() != self.table.to_pointer() {
            return Err(error("firstToUpper global string library was rebound"));
        }
        for (name, expected) in [("gsub", &self.gsub), ("upper", &self.upper)] {
            let actual: Function = self.table.raw_get(name)?;
            if actual.to_pointer() != expected.to_pointer() || actual.info().what != "C" {
                return Err(error(format!(
                    "firstToUpper standard string.{name} was rebound"
                )));
            }
        }
        let actual: Table = self.get_metatable.call("")?;
        let index: Table = actual.raw_get("__index")?;
        if actual.to_pointer() != self.metatable.to_pointer()
            || index.to_pointer() != self.table.to_pointer()
        {
            return Err(error("firstToUpper string method lookup was rebound"));
        }
        Ok(())
    }
}
const HELPER: &str = r#"local function firstToUpper(str)
 return (str:gsub("__PATTERN__", string.upper))
end"#;

pub(super) fn source_policy(
    lua: &Lua,
    sources: &BTreeMap<String, String>,
    spans: &mut BTreeMap<String, ItemSourceSpan>,
) -> Result<String> {
    let text = source(sources, PARSER)?;
    let start = "local function firstToUpper(";
    if text.matches(start).count() != 1 {
        return Err(error("firstToUpper declaration is missing or ambiguous"));
    }
    let part = section(text, start, "\nend")?;
    let at = part.as_ptr() as usize - text.as_ptr() as usize;
    let part = &text[at..at + part.len() + 4];
    let actual = tokens(part)?;
    let expected = tokens(HELPER)?;
    if actual.len() != expected.len() {
        return Err(error("complete firstToUpper helper changed"));
    }
    let mut pattern = None;
    for (a, e) in actual.iter().zip(&expected) {
        if e.quoted && e.text == r#""__PATTERN__""# {
            if !a.quoted || pattern.is_some() {
                return Err(error("firstToUpper requires one source pattern literal"));
            }
            pattern = Some(lua.load(format!("return {}", a.text)).eval::<String>()?);
        } else if a.text != e.text || a.quoted != e.quoted {
            return Err(error("complete firstToUpper helper changed"));
        }
    }
    let pattern = pattern.ok_or_else(|| error("missing firstToUpper pattern"))?;
    spans.insert(
        "first_to_upper_primitive".into(),
        part_span(sources, PARSER, part)?,
    );
    Ok(pattern)
}

pub(super) fn verify_helper(
    lua: &Lua,
    function: &Function,
    id: ParserCallbackId,
    descriptor: &ParserCallback,
    observed: &BTreeMap<usize, ParserCallbackId>,
    span: &ItemSourceSpan,
) -> Result<()> {
    if observed.get(&(function.to_pointer() as usize)) != Some(&id)
        || descriptor.kind
            != (ParserCallbackKind::Lua {
                source: span.clone(),
            })
        || !descriptor.upvalues.is_empty()
        || function
            .environment()
            .is_none_or(|e| e.to_pointer() != lua.globals().to_pointer())
    {
        return Err(error(
            "firstToUpper is not the observed closed source helper",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sources() -> BTreeMap<String, String> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../vendor/path-of-building-poe2")
            .join(PARSER);
        BTreeMap::from([(
            PARSER.into(),
            std::fs::read_to_string(path).unwrap().replace("\r\n", "\n"),
        )])
    }
    #[test]
    fn complete_original_helper_exports_authored_pattern_and_span() {
        let sources = sources();
        let mut spans = BTreeMap::new();
        assert_eq!(
            source_policy(&Lua::new(), &sources, &mut spans).unwrap(),
            "^%l"
        );
        let span = &spans["first_to_upper_primitive"];
        assert_eq!(*span, line_span(&sources, PARSER, 13, 15).unwrap());
        for pattern in ["", "[", "()(.)", "(.)()"] {
            let changed = sources[PARSER].replacen(
                r#"str:gsub("^%l", string.upper)"#,
                &format!("str:gsub({pattern:?}, string.upper)"),
                1,
            );
            assert_eq!(
                source_policy(
                    &Lua::new(),
                    &BTreeMap::from([(PARSER.into(), changed)]),
                    &mut BTreeMap::new()
                )
                .unwrap(),
                pattern
            );
        }
    }
    #[test]
    fn changed_helper_method_return_arity_scope_and_primitive_reject() {
        let sources = sources();
        for (from, to) in [
            (
                r#"return (str:gsub("^%l", string.upper))"#,
                r#"return str:gsub("^%l", string.upper)"#,
            ),
            ("str:gsub", "string.gsub"),
            ("string.upper))", "string.lower))"),
            ("string.upper))", "string.upper, 1))"),
            ("firstToUpper(str)", "firstToUpper(str, extra)"),
            ("local function firstToUpper", "function firstToUpper"),
        ] {
            let changed = sources[PARSER].replacen(from, to, 1);
            assert!(
                source_policy(
                    &Lua::new(),
                    &BTreeMap::from([(PARSER.into(), changed)]),
                    &mut BTreeMap::new()
                )
                .is_err(),
                "{from}"
            );
        }
    }
    #[test]
    fn changed_global_raw_primitives_and_string_method_identity_reject() {
        for code in [
            "string = {}",
            "string.gsub = function() end",
            "string.upper = function() end",
            "string.gsub = string.upper",
            "getmetatable('').__index = {}",
            "getmetatable('').__index = setmetatable({}, {__index=string})",
        ] {
            let lua = Lua::new();
            let original = StringLibrary::capture(&lua).unwrap();
            lua.load(code).exec().unwrap();
            assert!(original.verify(&lua).is_err(), "{code}");
        }
    }
    #[test]
    fn helper_identity_descriptor_and_lexical_environment_are_bound() {
        let lua = Lua::new();
        let f: Function = lua
            .load("return function(str) return (str:gsub('^%l', string.upper)) end")
            .eval()
            .unwrap();
        let span = ItemSourceSpan {
            path: PARSER.into(),
            line: 13,
            end_line: 15,
            sha256: "0".repeat(64),
        };
        let descriptor = ParserCallback {
            kind: ParserCallbackKind::Lua {
                source: span.clone(),
            },
            upvalues: vec![],
            environment: ParserEnvironment::OriginalGlobals,
        };
        let id = ParserCallbackId(1);
        let observed = BTreeMap::from([(f.to_pointer() as usize, id)]);
        verify_helper(&lua, &f, id, &descriptor, &observed, &span).unwrap();
        assert!(
            verify_helper(&lua, &f, ParserCallbackId(2), &descriptor, &observed, &span).is_err()
        );
        let other: Function = lua.load("return function() end").eval().unwrap();
        assert!(verify_helper(&lua, &other, id, &descriptor, &observed, &span).is_err());
        let mut changed = descriptor.clone();
        changed.upvalues.push(ParserUpvalue {
            name: "string".into(),
            value: ParserValue::Nil,
        });
        assert!(verify_helper(&lua, &f, id, &changed, &observed, &span).is_err());
        let mut changed_span = span.clone();
        changed_span.end_line += 1;
        assert!(verify_helper(&lua, &f, id, &descriptor, &observed, &changed_span).is_err());
        f.set_environment(lua.create_table().unwrap()).unwrap();
        assert!(verify_helper(&lua, &f, id, &descriptor, &observed, &span).is_err());
    }
}
