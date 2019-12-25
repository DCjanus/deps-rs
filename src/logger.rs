use fern::colors::{Color, ColoredLevelConfig};
use log::LevelFilter;

pub fn init_logger() -> anyhow::Result<()> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            let color_config = ColoredLevelConfig::new()
                .info(Color::Green)
                .debug(Color::Magenta);
            out.finish(format_args!(
                "{} {} [{}:{}] {}",
                chrono::Local::now().format("%F %H:%M:%S %:z"),
                color_config.color(record.level()),
                record.target(),
                record.line().unwrap(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}
