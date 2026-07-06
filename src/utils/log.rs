use log::Level;

pub struct Logger;
static LOGGER: Logger = Logger;

impl Logger {
    pub fn init() {
        let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Debug));
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= Level::Debug
    }
    fn log(&self, record: &log::Record) {
        let lwrcsd = record.level().as_str().to_lowercase();
        let colors = match record.level() {
            Level::Info => ("\x1b[37m", "\x1b[36m", "\x1b[37m", "\x1b[37m", "\x1b[0m"),
            Level::Debug => ("\x1b[37m", "\x1b[35m", "\x1b[37m", "\x1b[37m", "\x1b[0m"),
            Level::Trace => ("\x1b[37m", "\x1b[37m", "\x1b[37m", "\x1b[37m", "\x1b[0m"),
            Level::Error => ("\x1b[37m", "\x1b[91m", "\x1b[37m", "\x1b[37m", "\x1b[0m"),
            Level::Warn => ("\x1b[37m", "\x1b[33m", "\x1b[37m", "\x1b[37m", "\x1b[0m"),
        };

        let formatted = format!(
            "{}[{}{}{}] {}({}) {}{}",
            colors.0,
            colors.1,
            lwrcsd,
            colors.2,
            colors.3,
            record.target(),
            colors.4,
            record.args()
        );
        println!("{}", formatted);
    }
    fn flush(&self) {}
}
