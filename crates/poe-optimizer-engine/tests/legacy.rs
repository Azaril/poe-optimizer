//! Legacy adapter and source contracts retained during the planned cutover.
//! Allocation-counter contracts remain in separate process-wide binaries.

// The shared corpus uses the same authenticated source loader as both old oracles.
#[cfg(not(target_arch = "wasm32"))]
#[path = "support/condition_producers_corpus.rs"]
mod corpus;
#[cfg(not(target_arch = "wasm32"))]
#[path = "support/condition_producers_oracle.rs"]
mod oracle;

mod captured_table_insert_contract;
mod condition_producers_parity;
mod data_injection;
mod defence_parity;
mod doubled_form_contract;
mod flag_factory_contract;
mod gsub_factory_contract;
mod item_runes;
mod item_runes_source;
mod item_tools_contract;
mod lua_bits_contract;
mod lua_gsub_contract;
mod lua_number_contract;
mod lua_pattern_contract;
mod mace_golden;
mod mixed_conditions_contract;
mod mixed_store_parity;
mod modifier_factory_contract;
mod modifier_parity;
mod modifier_parser_contract;
mod modifier_scan_contract;
mod number_factory_contract;
mod ordinary_factory_contract;
mod ordinary_timing;
mod parser_program_semantics;
mod source_program_closures;
mod source_program_coverage;
mod source_program_sessions;
mod source_program_table_constructors;
mod source_program_table_observations;
mod source_program_values;
mod spark_golden;
