// Template strings for code generation
// Currently using inline templates, but can be extended to use Handlebars templates

#[allow(dead_code)]
pub const PLUGIN_FILTER_TEMPLATE: &str = r#"
#[no_mangle]
pub extern "C" fn on_event(source_id: u32, seq_no: u64) -> i32 {
    // Your filter logic here
    1 // Accept
}
"#;
