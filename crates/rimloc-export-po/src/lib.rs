use rimloc_core::TransUnit;
use color_eyre::eyre::Result;

/// Заготовка. Позже подключим crate для работы с .po и реализуем экспорт.
pub fn write_po(_path: &std::path::Path, _units: &[TransUnit]) -> Result<()> {
    unimplemented!("PO export will be implemented with a dedicated crate");
}
