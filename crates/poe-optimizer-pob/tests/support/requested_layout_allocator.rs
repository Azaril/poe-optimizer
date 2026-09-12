//! Test-executable-only requested Rust layouts, not allocator usable sizes or RSS.
//! The same forwarding implementation serves global accounting and a local self-check.
use serde::Serialize;
use std::{
    alloc::{GlobalAlloc, Layout, System, handle_alloc_error},
    sync::atomic::{AtomicBool, AtomicU64, Ordering::SeqCst},
};

struct RequestedAllocator {
    alloc: AtomicU64,
    zeroed: AtomicU64,
    realloc: AtomicU64,
    dealloc: AtomicU64,
    failed_alloc: AtomicU64,
    failed_zeroed: AtomicU64,
    failed_realloc: AtomicU64,
    requested: AtomicU64,
    released: AtomicU64,
    live: AtomicU64,
    peak: AtomicU64,
    interval_peak: AtomicU64,
    active: AtomicBool,
    invalid: AtomicBool,
}
#[global_allocator]
static GLOBAL: RequestedAllocator = RequestedAllocator::new();

impl RequestedAllocator {
    const fn new() -> Self {
        Self {
            alloc: AtomicU64::new(0),
            zeroed: AtomicU64::new(0),
            realloc: AtomicU64::new(0),
            dealloc: AtomicU64::new(0),
            failed_alloc: AtomicU64::new(0),
            failed_zeroed: AtomicU64::new(0),
            failed_realloc: AtomicU64::new(0),
            requested: AtomicU64::new(0),
            released: AtomicU64::new(0),
            live: AtomicU64::new(0),
            peak: AtomicU64::new(0),
            interval_peak: AtomicU64::new(0),
            active: AtomicBool::new(false),
            invalid: AtomicBool::new(false),
        }
    }
    // Allocator hooks must not allocate, format, lock or panic.
    fn add(&self, counter: &AtomicU64, amount: u64) -> u64 {
        let before = counter.fetch_add(amount, SeqCst);
        match before.checked_add(amount) {
            Some(value) => value,
            None => {
                self.invalid.store(true, SeqCst);
                0
            }
        }
    }
    fn subtract_live(&self, amount: u64) {
        if self.live.fetch_sub(amount, SeqCst) < amount {
            self.invalid.store(true, SeqCst);
        }
    }
    fn increase_live(&self, amount: u64) {
        let live = self.add(&self.live, amount);
        self.peak.fetch_max(live, SeqCst);
        if self.active.load(SeqCst) {
            self.interval_peak.fetch_max(live, SeqCst);
        }
    }
    fn acquired(&self, size: usize, counter: &AtomicU64) {
        self.add(counter, 1);
        self.add(&self.requested, size as u64);
        self.increase_live(size as u64);
    }
    fn read(&self) -> Totals {
        Totals {
            alloc_calls: self.alloc.load(SeqCst),
            alloc_zeroed_calls: self.zeroed.load(SeqCst),
            realloc_calls: self.realloc.load(SeqCst),
            dealloc_calls: self.dealloc.load(SeqCst),
            failed_alloc_calls: self.failed_alloc.load(SeqCst),
            failed_zeroed_calls: self.failed_zeroed.load(SeqCst),
            failed_realloc_calls: self.failed_realloc.load(SeqCst),
            requested_bytes: self.requested.load(SeqCst),
            released_bytes: self.released.load(SeqCst),
            live_requested_bytes: self.live.load(SeqCst),
            process_peak_requested_bytes: self.peak.load(SeqCst),
        }
    }
    fn begin(&self) -> Interval<'_> {
        assert!(
            self.active
                .compare_exchange(false, true, SeqCst, SeqCst)
                .is_ok(),
            "overlapping requested-layout intervals are not supported"
        );
        let before = self.read();
        self.interval_peak
            .store(before.live_requested_bytes, SeqCst);
        Interval {
            allocator: self,
            before,
        }
    }
    fn assert_valid(&self) {
        assert!(
            !self.invalid.load(SeqCst),
            "requested-layout counter overflow or underflow"
        );
    }
}

// SAFETY: every original pointer/layout/size is forwarded to System exactly once.
// Successful calls change accounting; null alloc/realloc never acquires/releases storage.
unsafe impl GlobalAlloc for RequestedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc(layout) };
        if pointer.is_null() {
            self.add(&self.failed_alloc, 1);
        } else {
            self.acquired(layout.size(), &self.alloc);
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = unsafe { System.alloc_zeroed(layout) };
        if pointer.is_null() {
            self.add(&self.failed_zeroed, 1);
        } else {
            self.acquired(layout.size(), &self.zeroed);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) };
        self.add(&self.dealloc, 1);
        self.add(&self.released, layout.size() as u64);
        self.subtract_live(layout.size() as u64);
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let result = unsafe { System.realloc(pointer, layout, new_size) };
        if result.is_null() {
            self.add(&self.failed_realloc, 1);
        } else {
            self.add(&self.realloc, 1);
            self.add(&self.requested, new_size as u64);
            self.add(&self.released, layout.size() as u64);
            if new_size >= layout.size() {
                self.increase_live((new_size - layout.size()) as u64);
            } else {
                self.subtract_live((layout.size() - new_size) as u64);
            }
        }
        result
    }
}

#[derive(Clone, Copy, Serialize)]
pub struct Totals {
    pub alloc_calls: u64,
    pub alloc_zeroed_calls: u64,
    pub realloc_calls: u64,
    pub dealloc_calls: u64,
    pub failed_alloc_calls: u64,
    pub failed_zeroed_calls: u64,
    pub failed_realloc_calls: u64,
    pub requested_bytes: u64,
    pub released_bytes: u64,
    pub live_requested_bytes: u64,
    pub process_peak_requested_bytes: u64,
}
#[derive(Serialize)]
pub struct Usage {
    pub before: Totals,
    pub after: Totals,
    pub alloc_calls: u64,
    pub alloc_zeroed_calls: u64,
    pub realloc_calls: u64,
    pub dealloc_calls: u64,
    pub failed_alloc_calls: u64,
    pub failed_zeroed_calls: u64,
    pub failed_realloc_calls: u64,
    pub requested_traffic_bytes: u64,
    pub released_traffic_bytes: u64,
    pub live_delta_bytes: i128,
    pub peak_live_requested_bytes: u64,
    pub peak_growth_above_start_bytes: u64,
}
pub struct Interval<'a> {
    allocator: &'a RequestedAllocator,
    before: Totals,
}
impl Interval<'_> {
    pub fn finish(self) -> Usage {
        let after = self.allocator.read();
        let peak = self.allocator.interval_peak.load(SeqCst);
        self.allocator.assert_valid();
        let before = self.before;
        Usage {
            before,
            after,
            alloc_calls: after.alloc_calls - before.alloc_calls,
            alloc_zeroed_calls: after.alloc_zeroed_calls - before.alloc_zeroed_calls,
            realloc_calls: after.realloc_calls - before.realloc_calls,
            dealloc_calls: after.dealloc_calls - before.dealloc_calls,
            failed_alloc_calls: after.failed_alloc_calls - before.failed_alloc_calls,
            failed_zeroed_calls: after.failed_zeroed_calls - before.failed_zeroed_calls,
            failed_realloc_calls: after.failed_realloc_calls - before.failed_realloc_calls,
            requested_traffic_bytes: after.requested_bytes - before.requested_bytes,
            released_traffic_bytes: after.released_bytes - before.released_bytes,
            live_delta_bytes: i128::from(after.live_requested_bytes)
                - i128::from(before.live_requested_bytes),
            peak_live_requested_bytes: peak,
            peak_growth_above_start_bytes: peak - before.live_requested_bytes,
        }
    }
}
impl Drop for Interval<'_> {
    fn drop(&mut self) {
        // Also runs if the measured operation unwinds. Global ownership counters
        // remain live; only interval peak recording ends. No allocator recursion.
        self.allocator.active.store(false, SeqCst);
    }
}
pub fn begin() -> Interval<'static> {
    GLOBAL.begin()
}
pub fn snapshot() -> Totals {
    GLOBAL.assert_valid();
    GLOBAL.read()
}

#[derive(Serialize)]
pub struct SelfCheck {
    grow_shrink_free: Usage,
    free_pre_interval_allocation: Usage,
    guard_cancel_and_reopen: bool,
    scope: &'static str,
}
pub fn self_check() -> SelfCheck {
    // Isolated counters avoid unrelated libtest startup traffic in exact checks.
    // These direct System allocations deliberately bypass GLOBAL, and are all freed.
    let allocator = RequestedAllocator::new();
    let small = Layout::from_size_align(32, 8).unwrap();
    let zeroed = Layout::from_size_align(16, 8).unwrap();
    let grown = Layout::from_size_align(80, 8).unwrap();
    let shrunk = Layout::from_size_align(24, 8).unwrap();
    let guard = allocator.begin();
    // SAFETY: all layouts are valid/nonzero, successful realloc transfers the sole
    // pointer, and each live allocation is freed once with its current layout.
    unsafe {
        let pointer = allocator.alloc(small);
        if pointer.is_null() {
            handle_alloc_error(small);
        }
        let zeros = allocator.alloc_zeroed(zeroed);
        if zeros.is_null() {
            handle_alloc_error(zeroed);
        }
        assert!(
            std::slice::from_raw_parts(zeros, zeroed.size())
                .iter()
                .all(|byte| *byte == 0)
        );
        let pointer = allocator.realloc(pointer, small, grown.size());
        if pointer.is_null() {
            handle_alloc_error(grown);
        }
        let pointer = allocator.realloc(pointer, grown, shrunk.size());
        if pointer.is_null() {
            handle_alloc_error(shrunk);
        }
        allocator.dealloc(pointer, shrunk);
        allocator.dealloc(zeros, zeroed);
    }
    let grow_shrink_free = guard.finish();
    assert_eq!(
        (
            grow_shrink_free.alloc_calls,
            grow_shrink_free.alloc_zeroed_calls,
            grow_shrink_free.realloc_calls,
            grow_shrink_free.dealloc_calls
        ),
        (1, 1, 2, 2)
    );
    assert_eq!(
        (
            grow_shrink_free.requested_traffic_bytes,
            grow_shrink_free.released_traffic_bytes
        ),
        (152, 152)
    );
    assert_eq!(grow_shrink_free.live_delta_bytes, 0);
    assert_eq!(grow_shrink_free.peak_growth_above_start_bytes, 96);
    assert_eq!(
        (
            grow_shrink_free.failed_alloc_calls,
            grow_shrink_free.failed_zeroed_calls,
            grow_shrink_free.failed_realloc_calls
        ),
        (0, 0, 0)
    );
    // SAFETY: allocate before the interval, then free the sole pointer with the
    // same valid layout. Ownership accounting must not reset at begin().
    let prior = unsafe { allocator.alloc(small) };
    if prior.is_null() {
        handle_alloc_error(small);
    }
    let guard = allocator.begin();
    unsafe { allocator.dealloc(prior, small) };
    let free_pre_interval_allocation = guard.finish();
    assert_eq!(free_pre_interval_allocation.before.live_requested_bytes, 32);
    assert_eq!(free_pre_interval_allocation.after.live_requested_bytes, 0);
    assert_eq!(free_pre_interval_allocation.requested_traffic_bytes, 0);
    assert_eq!(free_pre_interval_allocation.released_traffic_bytes, 32);
    assert_eq!(free_pre_interval_allocation.live_delta_bytes, -32);
    assert_eq!(
        free_pre_interval_allocation.peak_growth_above_start_bytes,
        0
    );
    assert_eq!(free_pre_interval_allocation.dealloc_calls, 1);
    drop(allocator.begin());
    let reopened = allocator.begin().finish();
    assert_eq!(reopened.live_delta_bytes, 0);
    SelfCheck {
        grow_shrink_free,
        free_pre_interval_allocation,
        guard_cancel_and_reopen: true,
        scope: "Same forwarding/accounting code with stack-local counters; alloc/zeroed/grow/shrink/free and prior-interval free; no forced failure or panic injection",
    }
}
