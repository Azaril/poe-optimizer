//! Full original import with one opt-in activation input observer.
use super::materialization::{Case, SourceObservation, source as bootstrap};
#[path = "item_set_activation_sort.rs"]
mod startup_sort;
use mlua::{Function, Lua, Table, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use std::{cell::RefCell, path::Path, rc::Rc};
pub const OBSERVER: &str = include_str!("item_set_lifecycle.lua");
pub struct Observed {
    pub observation: SourceObservation,
    pub parser: Function,
    library: Table,
}
impl Observed {
    pub fn verify_parser(&self) {
        assert_eq!(
            self.observation
                .lua
                .globals()
                .raw_get::<Table>("modLib")
                .unwrap(),
            self.library
        );
        assert_eq!(
            self.library.raw_get::<Function>("parseMod").unwrap(),
            self.parser
        );
    }
}
pub fn observe(repo: &Path, directory: &Path, case: &Case, observed: bool) -> Observed {
    std::fs::create_dir_all(directory).unwrap();
    let host = Rc::new(RefCell::new(None::<Lua>));
    let module = Rc::new(RefCell::new(None::<Table>));
    let parser = Rc::new(RefCell::new(None::<(Table, Function)>));
    let options = Rc::new(RefCell::new(None::<Function>));
    let capture = Rc::new(RefCell::new(None::<Table>));
    let startup = Rc::new(RefCell::new(None::<startup_sort::StartupSort>));
    let before_source = |lua: &Lua| -> Result<(), RuntimeError> {
        *host.borrow_mut() = Some(lua.clone());
        *module.borrow_mut() = Some(
            lua.load(OBSERVER)
                .set_name("@item_set_lifecycle.lua")
                .eval()?,
        );
        let validity: Table = lua
            .load(include_str!("item_slot_validity_source.lua"))
            .set_name("@item_slot_validity_source.lua")
            .eval()?;
        *options.borrow_mut() = Some(
            lua.load(include_str!("item_set_activation_source.lua"))
                .set_name("@item_set_activation_source.lua")
                .call(validity)?,
        );
        // Loaded last so the startup hook cannot observe our helper setup as
        // original initialization. Controls retain/check the C intrinsic only.
        *startup.borrow_mut() = Some(startup_sort::StartupSort::start(lua, observed)?);
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        startup.borrow_mut().as_mut().unwrap().finish()?;
        let library: Table = lua.globals().raw_get("modLib")?;
        *parser.borrow_mut() = Some((library.clone(), library.raw_get("parseMod")?));
        let options: Table = options.borrow().as_ref().unwrap().call(())?;
        if !observed {
            options.raw_set("activation_context", Value::Nil)?;
        }
        let active: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("start")?
            .call((observed, options))?;
        let finish = active.raw_get("finish")?;
        *capture.borrow_mut() = Some(active);
        Ok(finish)
    };
    let outcome = bootstrap::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        directory,
        &case.xml,
        None,
        case.structural,
        Some(&before_source),
        Some(&before_build),
        None,
    );
    // This also cleans up if initialization failed before before_build. A
    // failed/incomplete startup witness is a fixture failure, never parity.
    startup
        .borrow_mut()
        .as_mut()
        .unwrap()
        .finish()
        .expect("original startup sorter witness and cleanup");
    let startup_report = startup
        .borrow()
        .as_ref()
        .unwrap()
        .after_import()
        .expect("retained original sorter after import");
    let lua = host.borrow().as_ref().unwrap().clone();
    let report: Table = capture
        .borrow()
        .as_ref()
        .unwrap()
        .raw_get("report")
        .expect("observer cleanup and source receipt");
    let scope: Table = report.raw_get("scope").unwrap();
    scope.raw_set("rune_sort_startup", startup_report).unwrap();
    if observed {
        scope
            .raw_set(
                "rune_sort_permutation_checks",
                startup
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .permutations()
                    .expect("derived original-sort permutation checks"),
            )
            .unwrap();
    }
    assert!(matches!(
        lua.load("return debug.gethook()").eval::<Value>().unwrap(),
        Value::Nil
    ));
    let (library, parser) = parser.borrow().as_ref().unwrap().clone();
    let value = Observed {
        observation: SourceObservation {
            lua,
            report,
            outcome,
        },
        parser,
        library,
    };
    value.verify_parser();
    value
}
