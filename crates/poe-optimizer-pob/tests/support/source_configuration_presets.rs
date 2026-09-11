//! Complete original boss/preset callbacks on actual state, including non-None paths.
//! Each case uses one reusable compiled owner with a fresh private native session.
use super::*;
use sha2::{Digest, Sha256};

struct Restore {
    tables: Vec<(Table, Vec<(Value, Value)>)>,
    fields: Vec<(Table, String, Value)>,
}
impl Restore {
    fn new(captured: &capture::Captured, player: &Table, enemy: &Table, build: &Table) -> Self {
        let config: Table = build.raw_get("configTab").unwrap();
        let controls: Table = config.raw_get("varControls").unwrap();
        let mut result = Self {
            tables: Vec::new(),
            fields: Vec::new(),
        };
        let mut seen = BTreeSet::new();
        for table in [
            player.clone(),
            enemy.clone(),
            controls.clone(),
            config.raw_get("input").unwrap(),
            config.raw_get("placeholder").unwrap(),
        ] {
            if seen.insert(table.to_pointer()) {
                result.tables.push((
                    table.clone(),
                    table.pairs::<Value, Value>().map(Result::unwrap).collect(),
                ));
            }
        }
        result.field(build, "buildFlag");
        result.field(&config, "enemyLevel");
        for name in captured.names.clone().sequence_values::<String>() {
            result.field(
                &controls.raw_get::<Table>(name.unwrap()).unwrap(),
                "placeholder",
            );
        }
        for name in captured.dropdown_names.clone().sequence_values::<String>() {
            let control: Table = controls.raw_get(name.unwrap()).unwrap();
            result.field(&control, "selIndex");
            result.field(&control, "enabled");
        }
        // Existing rows are replaced/appended by these source methods, never
        // mutated in place. Retaining their actual Values preserves all aliases.
        result
    }
    fn field(&mut self, table: &Table, name: &str) {
        self.fields
            .push((table.clone(), name.into(), table.raw_get(name).unwrap()));
    }
    fn restore(&self) {
        for (table, entries) in &self.tables {
            let keys = table
                .clone()
                .pairs::<Value, Value>()
                .map(|entry| entry.unwrap().0)
                .collect::<Vec<_>>();
            for key in keys {
                if !entries.iter().any(|(saved, _)| saved == &key) {
                    table.raw_set(key, Value::Nil).unwrap();
                }
            }
            for (key, value) in entries {
                if table.raw_get::<Value>(key.clone()).unwrap() != *value {
                    table.raw_set(key.clone(), value.clone()).unwrap();
                }
            }
        }
        for (table, name, value) in &self.fields {
            table.raw_set(name.as_str(), value.clone()).unwrap();
        }
    }
}
impl Drop for Restore {
    fn drop(&mut self) {
        self.restore();
    }
}
struct Pair<'a> {
    captured: &'a capture::Captured,
    session: ProgramSession,
    roots: Vec<SessionValue>,
    player: Table,
    enemy: Table,
    build: Table,
    rows: Vec<Json>,
}
impl<'a> Pair<'a> {
    fn new(captured: &'a capture::Captured, player: &Table, enemy: &Table, build: &Table) -> Self {
        let (session, roots) = captured
            .compiled
            .session_from_input(
                captured.observed.input(),
                ProgramLimits {
                    max_steps: 5_000_000,
                    max_values: 500_000,
                    max_bytes: 32 * 1024 * 1024,
                    ..ProgramLimits::default()
                },
            )
            .unwrap();
        Self {
            captured,
            session,
            roots,
            player: player.clone(),
            enemy: enemy.clone(),
            build: build.clone(),
            rows: Vec::new(),
        }
    }
    fn root(&self, name: &str) -> SessionValue {
        self.roots[self.captured.observed.root_index(name).unwrap()].clone()
    }
    fn actual(&self) -> Json {
        let values: MultiValue = self
            .captured
            .probe
            .call((
                self.player.clone(),
                self.enemy.clone(),
                self.build.clone(),
                self.captured.names.clone(),
                self.captured.dropdown_names.clone(),
            ))
            .unwrap();
        observation::canonical(&observation::capture(&values.into_vec()))
    }
    fn native(&mut self) -> Json {
        let roots = [
            self.root("player"),
            self.root("enemy"),
            self.root("build"),
            self.root("names"),
            self.root("dropdown_names"),
        ];
        let values = self
            .session
            .invoke_callable(&self.root("probe.state"), &roots)
            .unwrap();
        observation::canonical(self.session.snapshot(&values).unwrap().graph())
    }
    fn check(&mut self, label: &str) -> Json {
        let expected = self.actual();
        assert_eq!(self.native(), expected, "continuing preset state: {label}");
        expected
    }
    fn record(&mut self, label: &str, state: &Json) {
        self.rows.push(json!({"step":label,"state_sha256":format!("{:x}",Sha256::digest(serde_json::to_vec(state).unwrap()))}));
    }
    fn select(&mut self, lua: &Lua, name: &str, value: &str) {
        self.check("before selection");
        let mut args = vec![self.root("build")];
        args.extend(
            self.session
                .borrow(&observation::capture(&[
                    Value::String(lua.create_string(name).unwrap()),
                    Value::String(lua.create_string(value).unwrap()),
                ]))
                .unwrap(),
        );
        assert!(
            self.session
                .invoke_callable(&self.root("probe.select"), &args)
                .unwrap()
                .is_empty()
        );
        let original: MultiValue = self
            .captured
            .select
            .call((self.build.clone(), name, value))
            .unwrap();
        assert!(original.is_empty());
        let state = self.check("after selection");
        self.record(&format!("select {name}={value}"), &state);
    }
    fn apply(&mut self, lua: &Lua, name: &str, value: &str, expect_error: bool) -> (Json, Json) {
        let before = self.check("callback entry");
        let index = self.captured.apply_indices[name];
        let value = Value::String(lua.create_string(value).unwrap());
        let mut args = self
            .session
            .borrow(&observation::capture(std::slice::from_ref(&value)))
            .unwrap();
        args.extend([self.root("player"), self.root("enemy"), self.root("build")]);
        let native = self
            .session
            .invoke_callable(&self.root(&format!("apply.{index}")), &args);
        let original = self.captured.functions[&index].call::<MultiValue>((
            value,
            self.player.clone(),
            self.enemy.clone(),
            self.build.clone(),
        ));
        if expect_error {
            assert!(original.is_err(), "expected original failure: {name}");
            assert_eq!(native.unwrap_err().kind, ProgramRuntimeErrorKind::Source);
        } else {
            assert!(original.unwrap().is_empty());
            assert!(native.unwrap().is_empty());
        }
        let after = self.check("callback exit or failed prefix");
        self.record(name, &after);
        (before, after)
    }
}

pub fn run(
    lua: &Lua,
    primitives: &Primitives,
    player: &Table,
    enemy: &Table,
    build: &Table,
) -> Json {
    let captured = capture::capture(lua, primitives, player, enemy, build);
    let restore = Restore::new(&captured, player, enemy, build);
    let baseline = Pair::new(&captured, player, enemy, build).actual();
    let config: Table = build.raw_get("configTab").unwrap();
    assert_eq!(
        player.to_pointer(),
        config.raw_get::<Table>("modList").unwrap().to_pointer()
    );
    assert_eq!(
        enemy.to_pointer(),
        config
            .raw_get::<Table>("enemyModList")
            .unwrap()
            .to_pointer()
    );
    let controls: Table = config.raw_get("varControls").unwrap();
    let boss: Table = controls.raw_get("enemyIsBoss").unwrap();
    let preset: Table = controls.raw_get("presetBossSkills").unwrap();
    let damage: Table = controls.raw_get("enemyDamageType").unwrap();
    let list_values = |control: &Table| {
        control
            .raw_get::<Table>("list")
            .unwrap()
            .sequence_values::<Table>()
            .map(|entry| entry.unwrap().raw_get::<String>("val").unwrap())
            .collect::<Vec<_>>()
    };
    let bosses = list_values(&boss);
    let names = list_values(&preset)
        .into_iter()
        .filter(|name| name != "None")
        .collect::<Vec<_>>();
    assert_eq!(
        (bosses.len(), names.len()),
        (4, 10),
        "review the pinned source preset inventory"
    );
    let definitions: Table = lua
        .globals()
        .raw_get::<Table>("data")
        .unwrap()
        .raw_get("bossSkills")
        .unwrap();
    let defined = definitions
        .clone()
        .pairs::<String, Table>()
        .map(|entry| entry.unwrap().0)
        .collect::<BTreeSet<_>>();
    assert_eq!(names.iter().cloned().collect::<BTreeSet<_>>(), defined);
    let mut reports = Vec::new();
    for boss_value in &bosses {
        for name in &names {
            restore.restore();
            let mut pair = Pair::new(&captured, player, enemy, build);
            assert_eq!(pair.check("fresh private session"), baseline);
            pair.select(lua, "enemyIsBoss", boss_value);
            pair.apply(lua, "enemyIsBoss", boss_value, false);
            pair.select(lua, "presetBossSkills", name);
            pair.apply(lua, "presetBossSkills", name, false);
            assert!(!damage.raw_get::<bool>("enabled").unwrap());
            pair.select(lua, "presetBossSkills", "None");
            pair.apply(lua, "presetBossSkills", "None", false);
            assert!(damage.raw_get::<bool>("enabled").unwrap());
            assert_eq!(
                config
                    .raw_get::<Table>("input")
                    .unwrap()
                    .raw_get::<String>("enemyDamageType")
                    .unwrap(),
                "Average"
            );
            let data: Table = definitions.raw_get(name.as_str()).unwrap();
            let multipliers: Table = data.raw_get("DamageMultipliers").unwrap();
            let damage_types = multipliers.pairs::<String, Value>().count();
            reports.push(json!({"boss":boss_value,"preset":name,"damage_types":damage_types,"earlier_uber":data.raw_get::<bool>("earlierUber").unwrap_or(false),"has_additional_stats":matches!(data.raw_get::<Value>("additionalStats").unwrap(),Value::Table(_)),"zero_crit":matches!(data.raw_get::<Value>("critChance").unwrap(),Value::Integer(0)|Value::Number(0.0)),"steps":pair.rows}));
        }
    }
    restore.restore();
    assert_eq!(
        Pair::new(&captured, player, enemy, build).actual(),
        baseline,
        "positive probes restore original state and aliases"
    );
    let mut failures = Vec::new();
    let mut missing = "__missing_boss_preset__".to_string();
    while defined.contains(&missing) {
        missing.push('_');
    }
    let mut pair = Pair::new(&captured, player, enemy, build);
    let (before, after) = pair.apply(lua, "presetBossSkills", &missing, true);
    assert_eq!(before, after, "unknown preset errors before writes");
    failures.push(json!({"case":"unknown definition","entry_unchanged":true,"steps":pair.rows}));
    restore.restore();
    controls.raw_set("enemyChaosDamage", Value::Nil).unwrap();
    let missing_control = capture::capture(lua, primitives, player, enemy, build);
    let mut pair = Pair::new(&missing_control, player, enemy, build);
    let (before, after) = pair.apply(lua, "presetBossSkills", &names[0], true);
    assert_ne!(
        before, after,
        "missing fifth damage control preserves earlier notifying writes"
    );
    failures.push(json!({"case":"missing fifth damage control","earlier_writes_preserved":true,"steps":pair.rows}));
    restore.restore();
    assert_eq!(
        Pair::new(&captured, player, enemy, build).actual(),
        baseline,
        "all fault probes restore original state and aliases"
    );
    json!({"cases":reports,"failure_prefixes":failures,"case_count":40,"positive_cases_share_compiled_owner":true,"private_session_per_case":true,"projected_source_state_and_aliases_restored":true,"scope":"Actual complete source boss/preset callbacks and dropdown method on controlled original state. Cases enumerate every injected preset and boss selection, then reset None. No UI selection callback or whole activation admission."})
}
