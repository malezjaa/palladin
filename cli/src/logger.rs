pub struct ColoredLogger;

impl log::Log for ColoredLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let target = metadata.target();

        target.starts_with("palladin") || target == "server" || metadata.level() <= log::Level::Warn
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = record.level();
        let msg = record.args();
        let target = record.target();

        let color = match level {
            log::Level::Info => "\x1b[36m",  // Cyan
            log::Level::Warn => "\x1b[33m",  // Yellow
            log::Level::Error => "\x1b[31m", // Red
            log::Level::Debug => "\x1b[35m", // Magenta
            log::Level::Trace => "\x1b[37m", // White
        };

        let reset = "\x1b[0m";
        let bold = "\x1b[1m";

        println!(
            "{color}{bold}[{target}]{reset} {msg}",
            target = target,
            msg = msg
        );
    }

    fn flush(&self) {}
}

pub static LOGGER: ColoredLogger = ColoredLogger;
