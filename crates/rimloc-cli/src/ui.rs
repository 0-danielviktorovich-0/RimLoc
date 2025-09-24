// Macros for localized UI output and translations.

#[macro_export]
macro_rules! tr {
    ($msg:literal $(, $k:ident = $v:expr )* $(,)?) => {{
        let loader = $crate::LANG_LOADER.get().expect("i18n not initialized");
        i18n_embed_fl::fl!(loader, $msg $(, $k = $v )* )
    }};
    ($msg:literal) => {{
        let loader = $crate::LANG_LOADER.get().expect("i18n not initialized");
        i18n_embed_fl::fl!(loader, $msg)
    }}
}

#[macro_export]
macro_rules! ui_ok {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        println!("✔ {}", $crate::tr!($k $(, $n = $v )* ));
    }};
}

#[macro_export]
macro_rules! ui_info {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        eprintln!("ℹ {}", $crate::tr!($k $(, $n = $v )* ));
    }};
}

#[macro_export]
macro_rules! ui_warn {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        let show_icon = std::io::stdout().is_terminal() && std::env::var_os("NO_ICONS").is_none();
        if show_icon {
            eprintln!("⚠ {}", $crate::tr!($k $(, $n = $v )* ));
        } else {
            eprintln!("{}", $crate::tr!($k $(, $n = $v )* ));
        }
    }};
}

#[macro_export]
macro_rules! ui_err {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        eprintln!("✖ {}", $crate::tr!($k $(, $n = $v )* ));
    }};
}

#[macro_export]
macro_rules! ui_out {
    ($k:literal $(, $n:ident = $v:expr )* $(,)?) => {{
        println!("{}", $crate::tr!($k $(, $n = $v )* ));
    }};
}
