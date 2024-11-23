// 初始化日志（设置日志格式）
pub fn init_logger() -> Result<(), fern::InitError> {
    fern::Dispatch
        ::new()
        .format(|out, message, record| {
            out.finish(
                format_args!(
                    "{} {:<5}{}",
                    chrono::Local::now().format("%H:%M:%S%.3f"),
                    record.level(),
                    message
                )
            )
        })
        .level(log::LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
