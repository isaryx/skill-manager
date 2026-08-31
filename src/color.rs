use std::io::IsTerminal;
use std::sync::OnceLock;

use clap::ValueEnum;

static COLOR_WHEN: OnceLock<ColorWhen> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum ColorWhen {
    #[default]
    Auto,
    Always,
    Never,
}

pub fn init(when: ColorWhen) {
    let _ = COLOR_WHEN.set(when);
}

fn when() -> ColorWhen {
    *COLOR_WHEN.get().unwrap_or(&ColorWhen::Auto)
}

/// Whether to emit ANSI styling on a stream (respects `--color` and env).
pub fn use_color(stream: &impl IsTerminal, when: ColorWhen) -> bool {
    match when {
        ColorWhen::Never => false,
        ColorWhen::Always => true,
        ColorWhen::Auto => auto_color(stream.is_terminal()),
    }
}

pub fn color_stdout() -> bool {
    use_color(&std::io::stdout(), when())
}

pub fn color_stderr() -> bool {
    use_color(&std::io::stderr(), when())
}

fn auto_color(stream_is_terminal: bool) -> bool {
    if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
        return false;
    }
    if std::env::var("CLICOLOR") == Ok("0".into()) {
        return false;
    }
    if std::env::var_os("CLICOLOR_FORCE").is_some_and(|v| !v.is_empty()) {
        return true;
    }
    stream_is_terminal
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn no_color_disables_styling() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::set("NO_COLOR", "1");
        assert!(!auto_color(true));
    }

    #[test]
    fn clicolor_zero_disables_styling() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _no_color = EnvGuard::remove("NO_COLOR");
        let _guard = EnvGuard::set("CLICOLOR", "0");
        assert!(!auto_color(true));
    }

    #[test]
    fn auto_respects_terminal_when_env_unset() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _no_color = EnvGuard::remove("NO_COLOR");
        let _clicolor = EnvGuard::remove("CLICOLOR");
        let _force = EnvGuard::remove("CLICOLOR_FORCE");
        assert!(!auto_color(false));
        assert!(auto_color(true));
    }

    #[test]
    fn explicit_never_and_always() {
        assert!(!use_color(&std::io::stdout(), ColorWhen::Never));
        assert!(use_color(&std::io::stdout(), ColorWhen::Always));
    }

    #[test]
    fn empty_no_color_does_not_disable() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::set("NO_COLOR", "");
        let _clicolor = EnvGuard::remove("CLICOLOR");
        let _force = EnvGuard::remove("CLICOLOR_FORCE");
        assert!(auto_color(true));
    }

    #[test]
    fn clicolor_force_enables_when_not_a_tty() {
        let _lock = ENV_LOCK.lock().unwrap();
        let _no_color = EnvGuard::remove("NO_COLOR");
        let _clicolor = EnvGuard::remove("CLICOLOR");
        let _force = EnvGuard::set("CLICOLOR_FORCE", "1");
        assert!(auto_color(false));
    }

    struct EnvGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            let previous = std::env::var(key).ok();
            // SAFETY: test-only; single-threaded unit tests.
            unsafe { std::env::set_var(key, value) };
            Self { key, previous }
        }

        fn remove(key: &'static str) -> Self {
            let previous = std::env::var(key).ok();
            unsafe { std::env::remove_var(key) };
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }
}
