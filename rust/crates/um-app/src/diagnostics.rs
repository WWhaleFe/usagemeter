//! 파일·stderr 진단. 서비스 응답이나 인증 정보는 기록하지 않는다.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const MAX_BYTES: u64 = 2 * 1024 * 1024;

struct LogFile {
    path: PathBuf,
    file: Option<File>,
    bytes: u64,
}

impl LogFile {
    fn open(path: PathBuf) -> Self {
        let bytes = std::fs::metadata(&path).map_or(0, |m| m.len());
        let mut log = Self { path, file: None, bytes };
        if bytes > MAX_BYTES { log.rotate(); }
        log.file = OpenOptions::new().create(true).append(true).open(&log.path).ok();
        log
    }

    fn rotate(&mut self) {
        // Windows에서는 열린 파일을 먼저 닫아야 이름을 바꿀 수 있다.
        self.file.take();
        let prev = self.path.with_file_name("usagemeter.prev.log");
        let _ = std::fs::remove_file(&prev);
        if std::fs::rename(&self.path, prev).is_ok() { self.bytes = 0; }
        self.file = OpenOptions::new().create(true).append(true).open(&self.path).ok();
    }

    fn write(&mut self, buf: &[u8]) {
        if self.bytes > MAX_BYTES { self.rotate(); }
        if let Some(file) = &mut self.file {
            if file.write_all(buf).is_ok() { self.bytes += buf.len() as u64; }
            let _ = file.flush();
        }
    }
}

#[derive(Clone)]
struct LogWriter(Arc<Mutex<LogFile>>);

impl Write for LogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let _ = io::stderr().write_all(buf);
        if let Ok(mut log) = self.0.lock() { log.write(buf); }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let _ = io::stderr().flush();
        Ok(())
    }
}

/// 오류의 동적 상세 대신 고정된 패닉 원인만 남긴다.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
    let message = payload.downcast_ref::<&str>().copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("");
    for cause in [
        "called `Result::unwrap()` on an `Err` value",
        "called `Option::unwrap()` on a `None` value",
        "index out of bounds",
        "attempt to divide by zero",
        "UsageMeter 시작 실패",
        "icon size",
        "pixmap size",
    ] {
        if message.starts_with(cause) { return cause; }
    }
    "패닉 메시지(민감 정보 생략)"
}

pub fn init() -> PathBuf {
    let dir = crate::state::data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("usagemeter.log");
    let writer = LogWriter(Arc::new(Mutex::new(LogFile::open(path.clone()))));
    let crash_writer = writer.clone();
    let crash_path = path.clone();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column())).unwrap_or_else(|| "위치 불명".into());
        // 동적으로 조합된 오류에는 응답·URL·토큰이 섞일 수 있어 본문을 남기지 않는다.
        let message = panic_message(info.payload());
        let line = format!("[패닉] {} {location}: {message}\n", chrono::Utc::now().to_rfc3339());
        let _ = io::stderr().write_all(line.as_bytes());
        // 패닉이 로거 잠금 안에서 발생했을 때도 멈추지 않는다.
        if let Ok(mut log) = crash_writer.0.try_lock() {
            log.write(line.as_bytes());
        } else if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&crash_path) {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("usagemeter-crash.log")) {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }));
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_ansi(false)
        .with_writer(move || writer.clone())
        .init();
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 패닉_상세의_민감_정보는_남기지_않는다() {
        let message = String::from("called `Result::unwrap()` on an `Err` value: 비공개 응답");
        assert_eq!(panic_message(&message), "called `Result::unwrap()` on an `Err` value");
        assert_eq!(panic_message(&"비공개 응답"), "패닉 메시지(민감 정보 생략)");
        assert_eq!(panic_message(&String::from("비공개 응답")), "패닉 메시지(민감 정보 생략)");
        assert_eq!(panic_message(&"UsageMeter 시작 실패: 비공개 응답"), "UsageMeter 시작 실패");
    }

    #[test]
    fn 시작과_실행_중에_로그를_회전한다() {
        let dir = std::env::current_dir().unwrap().join("target").join(format!("log-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("usagemeter.log");
        let prev = dir.join("usagemeter.prev.log");
        std::fs::write(&prev, b"old").unwrap();
        std::fs::write(&path, vec![b'x'; MAX_BYTES as usize + 1]).unwrap();
        let mut log = LogFile::open(path.clone());
        assert_eq!(std::fs::metadata(&prev).unwrap().len(), MAX_BYTES + 1);
        log.write(b"new");
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        log.write(&vec![b'y'; MAX_BYTES as usize]);
        log.write(b"next");
        assert_eq!(std::fs::metadata(&prev).unwrap().len(), MAX_BYTES + 3);
        assert_eq!(std::fs::read(&path).unwrap(), b"next");
        drop(log);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
