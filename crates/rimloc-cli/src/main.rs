//! Тонкий бинарный враппер: вся логика CLI живёт в lib-таргете (`lib.rs`).
//! Это позволяет интеграционному тесту (`tests/startup_stack.rs`) запускать
//! startup-путь в потоке с 1MB стеком (windows main-thread) в этом же процессе.

fn main() -> color_eyre::eyre::Result<()> {
    rimloc_cli::main_entry()
}
