macro_rules! ui_warn {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        // Print warnings to stderr and do not inject any emoji/symbols here.
        // Symbols, if desired, should live in the FTL values themselves.
        eprintln!("{}", tr!($k $(, $n = $v )* ));
    }};
}