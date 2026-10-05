//! WINDOWS_BETA_BLOCKER регресс: startup-путь CLI должен помещаться в 1MB стек —
//! размер main-thread по умолчанию на windows. До фикса derive-функция
//! `<Commands as clap::Subcommand>::augment_subcommands` для монолитного enum
//! на 25 вариантов имела debug-кадр ~1.8–2MB и роняла процесс с
//! STATUS_STACK_OVERFLOW (0xC00000FD) на ЛЮБОЙ команде, включая `--help`.
//!
//! Тест детерминированный: поднимает startup в потоке с фиксированным стеком
//! через `thread::Builder::stack_size`, переполнение убивает процесс (SIGSEGV),
//! так что «тихий зелёный» невозможен. Работает на всех платформах; на windows
//! это ровно нативное условие main-потока.

use std::thread::Builder;

/// 1 MiB — дефолтный main-thread стек на windows (на unix обычно 8 MiB).
const WIN_MAIN_THREAD_STACK: usize = 1024 * 1024;

#[test]
fn startup_fits_one_megabyte_stack() {
    let handle = Builder::new()
        .name("startup-1mb".into())
        .stack_size(WIN_MAIN_THREAD_STACK)
        .spawn(|| {
            // --version: построение дерева команд + парс (раньше падало здесь)
            rimloc_cli::smoke_startup(&["rimloc-cli", "--version"])
                .expect("--version must parse within 1MB stack");
            // --help: построение + рендер локализованного help
            rimloc_cli::smoke_startup(&["rimloc-cli", "--help"])
                .expect("--help must parse within 1MB stack");
            // подкоманда: augment внутри flatten-группы + локализация её аргументов
            rimloc_cli::smoke_startup(&["rimloc-cli", "scan", "--help"])
                .expect("scan --help must parse within 1MB stack");
        })
        .expect("failed to spawn 1MB-stack thread");

    // Переполнение стека в потоке — SIGSEGV процесса, то есть падение теста;
    // нормальное завершение значит startup уложился в 1MB.
    handle.join().expect("startup must not overflow 1MB stack");
}
