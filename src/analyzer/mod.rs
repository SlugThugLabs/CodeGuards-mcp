//! Analyzer coordinator module.

pub mod metrics;
pub mod tokenizer;
pub mod walker;

pub use metrics::{
    FanOut, FunctionSpan, LiteralKind, LiteralSite, MarkerKind, MarkerSite, NestingSite,
    extract_literals, find_action_markers, find_stub_markers, find_swallowed_errors,
    function_spans, has_doc_comment, has_test_module, is_test_path, module_fan_out, nesting_depth,
    test_region_lines,
};
pub use tokenizer::{
    count_code_lines, extract_imported_modules, find_debug_prints, find_unwrap_expect_calls,
    tokenize_source,
};
pub use walker::{collect_git_diff_files, collect_source_files};
