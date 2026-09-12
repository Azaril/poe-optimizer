//! Developer-only requested-allocation accounting; no production instrumentation.
//! Run `profile_allocations --self-check` in a separate process before measurement.
//! Run `profile_allocations` with no arguments for one cold/later load lifecycle.
use poe_optimizer_data::{
    self as data,
    game_data::{
        ConfigDefinitionCatalog, DataTrust, GameDataSnapshot, ItemAssemblyCatalog,
        ItemLoadingCatalog, ItemScalabilityCatalog, ModifierParserCatalog,
        ParserAdmittedProgramCatalog, SkillIdentityCatalog, SkillPreparationCatalog,
        UniqueRequirementCatalog, bundled_package_bytes, bundled_package_sha256, bundled_snapshot,
    },
};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    alloc::{GlobalAlloc, Layout, System, handle_alloc_error},
    error::Error,
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering::SeqCst},
    },
    time::Instant,
};

struct RequestedAllocator;
#[global_allocator]
static ALLOCATOR: RequestedAllocator = RequestedAllocator;
static ALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static ZEROED_CALLS: AtomicU64 = AtomicU64::new(0);
static REALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static DEALLOC_CALLS: AtomicU64 = AtomicU64::new(0);
static FAILED_CALLS: AtomicU64 = AtomicU64::new(0);
static REQUESTED: AtomicU64 = AtomicU64::new(0);
static RELEASED: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static INTERVAL_PEAK: AtomicU64 = AtomicU64::new(0);
static INVALID: AtomicBool = AtomicBool::new(false);

// Hooks never format, allocate, lock, or panic. Overflow invalidates the report.
fn add(counter: &AtomicU64, amount: u64) -> u64 {
    let old = counter.fetch_add(amount, SeqCst);
    match old.checked_add(amount) {
        Some(value) => value,
        None => {
            INVALID.store(true, SeqCst);
            0
        }
    }
}
fn subtract_live(amount: u64) {
    if LIVE.fetch_sub(amount, SeqCst) < amount {
        INVALID.store(true, SeqCst);
    }
}
fn increase_live(amount: u64) {
    let current = add(&LIVE, amount);
    PEAK.fetch_max(current, SeqCst);
    INTERVAL_PEAK.fetch_max(current, SeqCst);
}
fn acquired(size: usize, calls: &AtomicU64) {
    add(calls, 1);
    add(&REQUESTED, size as u64);
    increase_live(size as u64);
}

// SAFETY: every call forwards the original pointer/layout/size to System exactly
// once. Only successful returned allocations change ownership accounting.
unsafe impl GlobalAlloc for RequestedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let result = unsafe { System.alloc(layout) };
        if result.is_null() {
            add(&FAILED_CALLS, 1);
        } else {
            acquired(layout.size(), &ALLOC_CALLS);
        }
        result
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let result = unsafe { System.alloc_zeroed(layout) };
        if result.is_null() {
            add(&FAILED_CALLS, 1);
        } else {
            acquired(layout.size(), &ZEROED_CALLS);
        }
        result
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        add(&DEALLOC_CALLS, 1);
        add(&RELEASED, layout.size() as u64);
        subtract_live(layout.size() as u64);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(ptr, layout, new_size) };
        if result.is_null() {
            add(&FAILED_CALLS, 1);
        } else {
            add(&REALLOC_CALLS, 1);
            add(&REQUESTED, new_size as u64);
            add(&RELEASED, layout.size() as u64);
            if new_size >= layout.size() {
                increase_live((new_size - layout.size()) as u64);
            } else {
                subtract_live((layout.size() - new_size) as u64);
            }
        }
        result
    }
}

#[derive(Clone, Copy, Serialize)]
struct Counters {
    alloc_calls: u64,
    alloc_zeroed_calls: u64,
    realloc_calls: u64,
    dealloc_calls: u64,
    failed_calls: u64,
    requested_bytes: u64,
    released_bytes: u64,
    live_requested_bytes: u64,
    process_peak_requested_bytes: u64,
}
impl Counters {
    fn read() -> Self {
        Self {
            alloc_calls: ALLOC_CALLS.load(SeqCst),
            alloc_zeroed_calls: ZEROED_CALLS.load(SeqCst),
            realloc_calls: REALLOC_CALLS.load(SeqCst),
            dealloc_calls: DEALLOC_CALLS.load(SeqCst),
            failed_calls: FAILED_CALLS.load(SeqCst),
            requested_bytes: REQUESTED.load(SeqCst),
            released_bytes: RELEASED.load(SeqCst),
            live_requested_bytes: LIVE.load(SeqCst),
            process_peak_requested_bytes: PEAK.load(SeqCst),
        }
    }
}
#[derive(Serialize)]
struct Phase {
    name: &'static str,
    elapsed_ms: f64,
    before: Counters,
    after: Counters,
    successful_alloc_calls: u64,
    successful_alloc_zeroed_calls: u64,
    successful_realloc_calls: u64,
    dealloc_calls: u64,
    failed_calls: u64,
    requested_traffic_bytes: u64,
    released_traffic_bytes: u64,
    live_delta_bytes: i128,
    peak_live_requested_bytes: u64,
    peak_growth_above_start_bytes: u64,
}
fn measure<T>(name: &'static str, operation: impl FnOnce() -> T) -> (T, Phase) {
    let before = Counters::read();
    INTERVAL_PEAK.store(before.live_requested_bytes, SeqCst);
    let start = Instant::now();
    let result = black_box(operation());
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let after = Counters::read();
    let peak = INTERVAL_PEAK.load(SeqCst);
    assert!(
        !INVALID.load(SeqCst),
        "allocation counter overflow or underflow"
    );
    let phase = Phase {
        name,
        elapsed_ms,
        before,
        after,
        successful_alloc_calls: after.alloc_calls - before.alloc_calls,
        successful_alloc_zeroed_calls: after.alloc_zeroed_calls - before.alloc_zeroed_calls,
        successful_realloc_calls: after.realloc_calls - before.realloc_calls,
        dealloc_calls: after.dealloc_calls - before.dealloc_calls,
        failed_calls: after.failed_calls - before.failed_calls,
        requested_traffic_bytes: after.requested_bytes - before.requested_bytes,
        released_traffic_bytes: after.released_bytes - before.released_bytes,
        live_delta_bytes: i128::from(after.live_requested_bytes)
            - i128::from(before.live_requested_bytes),
        peak_live_requested_bytes: peak,
        peak_growth_above_start_bytes: peak - before.live_requested_bytes,
    };
    (result, phase)
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

// Stack-only witness survives teardown; every temporary Vec/String is dropped by
// this function before the next measured phase. No snapshot/owner is retained.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Witness {
    schema: u32,
    parser_owner_sha256: [u8; 64],
}
fn verify(snapshot: &GameDataSnapshot) -> Result<Witness, Box<dyn Error>> {
    let bytes = bundled_package_bytes();
    let package = snapshot.package();
    let identity = snapshot.identity();
    identity.validate()?;
    assert_eq!(hash(bytes), bundled_package_sha256());
    assert_eq!(identity.content_sha256, bundled_package_sha256());
    assert_eq!(identity.game, package.manifest.game);
    assert_eq!(identity.release, package.manifest.release);
    assert_eq!(identity.schema_version, package.manifest.schema_version);
    assert_eq!(
        identity.semantics_version,
        package.manifest.semantics_version
    );
    match snapshot.trust() {
        DataTrust::Reviewed { expected_sha256 } => {
            assert_eq!(expected_sha256, bundled_package_sha256())
        }
        _ => return Err("bundled data is not reviewed".into()),
    }
    {
        let canonical = package.canonical_bytes()?;
        assert_eq!(canonical.as_slice(), bytes);
    }
    assert!(snapshot.configuration().data() == &package.configuration);
    assert!(snapshot.skill_identities().data() == &package.skill_identities);
    assert!(snapshot.skill_preparation().data() == &package.skill_preparation);
    assert!(snapshot.item_loading().data() == &package.item_loading);
    assert!(snapshot.item_scalability().data() == &package.item_scalability);
    assert!(snapshot.modifier_parser().data() == &package.modifier_parser);
    assert!(snapshot.item_assembly().data() == &package.item_assembly);
    assert!(snapshot.unique_requirements().data() == &package.unique_requirements);
    assert!(
        snapshot
            .parser_programs()
            .is_bound_to(snapshot.modifier_parser())
    );
    let owner_digest = snapshot.modifier_parser().data().definition_sha256()?;
    assert_eq!(snapshot.parser_programs().owner_sha256(), owner_digest);
    let mut digest = [0; 64];
    digest.copy_from_slice(owner_digest.as_bytes());
    Ok(Witness {
        schema: identity.schema_version,
        parser_owner_sha256: digest,
    })
}

struct Owners {
    configuration: ConfigDefinitionCatalog,
    skill_identities: SkillIdentityCatalog,
    skill_preparation: SkillPreparationCatalog,
    item_loading: ItemLoadingCatalog,
    item_scalability: ItemScalabilityCatalog,
    modifier_parser: ModifierParserCatalog,
    item_assembly: ItemAssemblyCatalog,
    unique_requirements: UniqueRequirementCatalog,
    parser_programs: ParserAdmittedProgramCatalog,
}
impl Owners {
    fn retain(snapshot: &GameDataSnapshot) -> Self {
        Self {
            configuration: snapshot.configuration().clone(),
            skill_identities: snapshot.skill_identities().clone(),
            skill_preparation: snapshot.skill_preparation().clone(),
            item_loading: snapshot.item_loading().clone(),
            item_scalability: snapshot.item_scalability().clone(),
            modifier_parser: snapshot.modifier_parser().clone(),
            item_assembly: snapshot.item_assembly().clone(),
            unique_requirements: snapshot.unique_requirements().clone(),
            parser_programs: snapshot.parser_programs().clone(),
        }
    }
    fn addresses(&self) -> [usize; 8] {
        [
            self.configuration.data() as *const _ as usize,
            self.skill_identities.data() as *const _ as usize,
            self.skill_preparation.data() as *const _ as usize,
            self.item_loading.data() as *const _ as usize,
            self.item_scalability.data() as *const _ as usize,
            self.modifier_parser.data() as *const _ as usize,
            self.item_assembly.data() as *const _ as usize,
            self.unique_requirements.data() as *const _ as usize,
        ]
    }
    fn verify_shared(&self, snapshot: &GameDataSnapshot) {
        assert!(std::ptr::eq(
            self.configuration.data(),
            snapshot.configuration().data()
        ));
        assert!(std::ptr::eq(
            self.skill_identities.data(),
            snapshot.skill_identities().data()
        ));
        assert!(std::ptr::eq(
            self.skill_preparation.data(),
            snapshot.skill_preparation().data()
        ));
        assert!(std::ptr::eq(
            self.item_loading.data(),
            snapshot.item_loading().data()
        ));
        assert!(std::ptr::eq(
            self.item_scalability.data(),
            snapshot.item_scalability().data()
        ));
        assert!(std::ptr::eq(
            self.modifier_parser.data(),
            snapshot.modifier_parser().data()
        ));
        assert!(std::ptr::eq(
            self.item_assembly.data(),
            snapshot.item_assembly().data()
        ));
        assert!(std::ptr::eq(
            self.unique_requirements.data(),
            snapshot.unique_requirements().data()
        ));
        assert!(self.parser_programs.is_bound_to(snapshot.modifier_parser()));
        assert!(self.parser_programs.is_bound_to(&self.modifier_parser));
        assert!(std::ptr::eq(
            self.parser_programs.programs().data(),
            snapshot.parser_programs().programs().data()
        ));
        assert_eq!(
            self.parser_programs.owner_sha256(),
            snapshot.parser_programs().owner_sha256()
        );
    }
}

fn self_check() -> Result<(), Box<dyn Error>> {
    let small = Layout::from_size_align(32, 8)?;
    let zero = Layout::from_size_align(16, 8)?;
    let grown = Layout::from_size_align(80, 8)?;
    let shrunk = Layout::from_size_align(24, 8)?;
    let (_, phase) = measure("counter_self_check", || {
        // SAFETY: direct calls exercise the same global wrapper, preserve valid
        // layouts/alignment, and free each final allocation exactly once.
        unsafe {
            let p = ALLOCATOR.alloc(small);
            if p.is_null() {
                handle_alloc_error(small);
            }
            let z = ALLOCATOR.alloc_zeroed(zero);
            if z.is_null() {
                handle_alloc_error(zero);
            }
            assert!(std::slice::from_raw_parts(z, 16).iter().all(|x| *x == 0));
            let p = ALLOCATOR.realloc(p, small, grown.size());
            if p.is_null() {
                handle_alloc_error(grown);
            }
            let p = ALLOCATOR.realloc(p, grown, shrunk.size());
            if p.is_null() {
                handle_alloc_error(shrunk);
            }
            ALLOCATOR.dealloc(p, shrunk);
            ALLOCATOR.dealloc(z, zero);
        }
    });
    assert_eq!(phase.successful_alloc_calls, 1);
    assert_eq!(phase.successful_alloc_zeroed_calls, 1);
    assert_eq!(phase.successful_realloc_calls, 2);
    assert_eq!(phase.dealloc_calls, 2);
    assert_eq!(phase.failed_calls, 0);
    assert_eq!(phase.requested_traffic_bytes, 152);
    assert_eq!(phase.released_traffic_bytes, 152);
    assert_eq!(phase.live_delta_bytes, 0);
    assert_eq!(phase.peak_growth_above_start_bytes, 96);
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1, "status": "counter_self_check_passed", "phase": phase,
            "scope": "successful System alloc/zeroed/realloc grow/shrink/dealloc; no failure injection or DATA loading"
        }))?
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let self_check_mode = {
        let mut args = std::env::args().skip(1);
        let mode = args.next();
        if args.next().is_some() || mode.as_deref().is_some_and(|x| x != "--self-check") {
            return Err("Usage: profile_allocations [--self-check]".into());
        }
        mode.is_some()
    };
    if self_check_mode {
        return self_check();
    }
    // No DATA load, content hashing, or witness buffers precede this interval.
    let (loaded, cold) = measure("cold_bundled_snapshot", bundled_snapshot);
    let snapshot = loaded?;
    let witness = verify(&snapshot)?;
    let (owned_copy, clone_owned) =
        measure("owned_snapshot_clone_api_contrast", || snapshot.clone());
    assert!(verify(&owned_copy)? == witness);
    assert!(!std::ptr::eq(owned_copy.package(), snapshot.package()));
    assert!(
        owned_copy
            .modifier_parser()
            .is_same_owner(snapshot.modifier_parser())
    );
    let (_, drop_owned) = measure("drop_owned_snapshot_clone", || drop(owned_copy));
    let (shared, wrap_arc) = measure("wrap_snapshot_in_arc", || Arc::new(snapshot));
    let (handle, clone_arc) = measure("clone_shared_snapshot_arc", || Arc::clone(&shared));
    assert!(Arc::ptr_eq(&shared, &handle));
    assert_eq!(Arc::strong_count(&shared), 2);
    let (_, drop_handle) = measure("drop_shared_snapshot_arc_clone", || drop(handle));
    assert_eq!(Arc::strong_count(&shared), 1);
    let (owners, clone_catalogs) = measure("clone_eight_catalogs_and_parser_programs", || {
        Owners::retain(&shared)
    });
    owners.verify_shared(&shared);
    let addresses = owners.addresses();
    let (_, drop_snapshot) = measure("drop_final_snapshot_arc_owners_retained", || drop(shared));
    // These handles are still usable and preserve the original owners. The saved
    // addresses are integers, never dereferenced after ownership has been dropped.
    assert_eq!(owners.addresses(), addresses);
    assert!(owners.parser_programs.is_bound_to(&owners.modifier_parser));
    assert_eq!(
        owners.parser_programs.owner_sha256().as_bytes(),
        witness.parser_owner_sha256
    );
    let (_, drop_owners) = measure("drop_all_retained_catalog_and_parser_owners", || {
        drop(owners)
    });
    let (loaded, later) = measure("later_bundled_snapshot_same_process", bundled_snapshot);
    let later_snapshot = loaded?;
    assert!(verify(&later_snapshot)? == witness);
    let (_, drop_later) = measure("drop_later_snapshot_without_extra_owners", || {
        drop(later_snapshot)
    });
    let final_before_output = Counters::read();
    assert!(!INVALID.load(SeqCst));
    // JSON, source fingerprinting, and printing occur only after all intervals.
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1, "status": "requested_allocation_profile_passed",
            "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
            "debug_assertions": cfg!(debug_assertions),
            "input_bytes": bundled_package_bytes().len(), "input_sha256": bundled_package_sha256(),
            "data_schema_version": witness.schema, "source_fingerprint": data::implementation_fingerprint(),
            "example_sha256": hash(include_bytes!("profile_allocations.rs")),
            "witnesses": {"canonical_bytes_exact": true, "data_identity_and_reviewed_trust": true,
                "all_eight_catalog_contents": true, "parser_owner_binding_and_digest": true,
                "parser_owner_sha256": std::str::from_utf8(&witness.parser_owner_sha256)?,
                "owned_clone_and_later_load_same_content": true, "catalog_data_pointer_sharing": true,
                "snapshot_arc_pointer_sharing": true, "retained_owners_survive_snapshot_drop": true},
            "phases": [cold, clone_owned, drop_owned, wrap_arc, clone_arc, drop_handle,
                clone_catalogs, drop_snapshot, drop_owners, later, drop_later],
            "final_counters_before_output": final_before_output,
            "protocol": "Fresh process; first load; content witnesses; optional owned-clone contrast/drop; Arc sharing; retain catalog/parser owners; drop snapshot; drop owners; later load/witness/drop.",
            "limitations": [
                "Bytes are successful GlobalAlloc requested layouts, not RSS, committed memory, usable allocator sizes, fragmentation, headers, stacks, static bundled bytes, or native allocations bypassing this wrapper.",
                "Realloc traffic counts full new requested size and full released old size; live and peak use only their delta. Internal temporary copies during realloc are not observed.",
                "All allocations are accounted from process start, including runtime, witnesses, and output. Interval deltas exclude between-interval witnesses; cumulative totals include them.",
                "Phase records and retained witnesses use stack-only storage. Canonical serialization and digest buffers are dropped before teardown, but their allocator/cache effects remain.",
                "Residual requested bytes can include process statics/caches and runtime state; a nonzero remainder alone is not evidence of a DATA leak or attribution to a particular cache.",
                "The owned snapshot clone is a separate API contrast, not the production worker ownership path. Arc/catalog clones share owners; parser-view clones may copy small metadata.",
                "This executable starts no worker threads. Counters include any concurrent process allocations; multi-field snapshots and interval peak resets require quiescent boundaries.",
                "Atomic counting and timing add observer overhead. Elapsed times are instrumented lifecycle timings, not an uninstrumented throughput comparison.",
                "Cold means first DATA load in this process, not cold OS caches. Later load follows full witnesses and allocator reuse. No internal loader-phase attribution or full-build parity claim."
            ]
        }))?
    );
    Ok(())
}
