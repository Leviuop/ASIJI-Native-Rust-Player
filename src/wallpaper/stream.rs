use anyhow::Result;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

struct Shared {
    frame: Mutex<Arc<Vec<u8>>>,
    stopped: AtomicBool,
    token: String,
    period: Duration,
}
pub struct FrameServer {
    shared: Arc<Shared>,
    listener: Option<JoinHandle<()>>,
    address: std::net::SocketAddr,
}
impl FrameServer {
    pub fn start(fps: u32) -> Result<Self> {
        let mut random = [0u8; 32];
        getrandom::fill(&mut random)
            .map_err(|e| anyhow::anyhow!("Не удалось создать ключ сеанса: {e}"))?;
        let token: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
        listener.set_nonblocking(true)?;
        let address = listener.local_addr()?;
        let shared = Arc::new(Shared {
            frame: Mutex::new(Arc::new(super::frame::bmp(
                &[0; 6],
                (1, 1),
                crate::render::Mode::Blocks,
                true,
            )?)),
            stopped: AtomicBool::new(false),
            token,
            period: Duration::from_secs_f64(1.0 / fps.clamp(10, 60) as f64),
        });
        let state = shared.clone();
        let handle = thread::spawn(move || {
            let mut workers: Vec<JoinHandle<()>> = Vec::new();
            while !state.stopped.load(Ordering::Relaxed) {
                workers.retain(|worker| !worker.is_finished());
                match listener.accept() {
                    Ok((socket, _)) if workers.len() < 16 => {
                        let state = state.clone();
                        workers.push(thread::spawn(move || {
                            let _ = serve(socket, &state);
                        }));
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10))
                    }
                    Err(_) => break,
                }
            }
            for worker in workers {
                let _ = worker.join();
            }
        });
        Ok(Self {
            shared,
            listener: Some(handle),
            address,
        })
    }
    pub fn url(&self) -> String {
        format!("http://{}/{}", self.address, self.shared.token)
    }
    pub fn publish(&self, frame: Vec<u8>) {
        *self.shared.frame.lock().unwrap() = Arc::new(frame);
    }
}
impl Drop for FrameServer {
    fn drop(&mut self) {
        self.shared.stopped.store(true, Ordering::Relaxed);
        if let Some(listener) = self.listener.take() {
            let _ = listener.join();
        }
    }
}
fn serve(mut socket: TcpStream, state: &Shared) -> std::io::Result<()> {
    socket.set_read_timeout(Some(Duration::from_secs(1)))?;
    socket.set_write_timeout(Some(Duration::from_secs(1)))?;
    socket.set_nodelay(true)?;
    let mut request = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    // No general-purpose file serving, CORS, command API, or unbounded request bodies.
    while request.len() < 4096 && std::time::Instant::now() < deadline {
        let mut byte = [0];
        if socket.read(&mut byte)? == 0 {
            return Ok(());
        }
        request.push(byte[0]);
        if request.ends_with(b"\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&request);
    let mut line = text.lines().next().unwrap_or("").split_whitespace();
    let method = line.next();
    let path = line.next().unwrap_or("").split('?').next().unwrap_or("");
    let prefix = format!("/{}/", state.token);
    let endpoint = path.strip_prefix(&prefix);
    if method != Some("GET") || !matches!(endpoint, Some("frame.bmp" | "stream.bmp")) {
        return socket.write_all(
            b"HTTP/1.0 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
    }
    let streaming = endpoint == Some("stream.bmp");
    let frame = state.frame.lock().unwrap().clone();
    write!(
        socket,
        "HTTP/1.0 200 OK\r\nContent-Type: image/bmp\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n"
    )?;
    if !streaming {
        write!(socket, "Content-Length: {}\r\n", frame.len())?;
    }
    socket.write_all(b"\r\n")?;
    socket.write_all(&frame)?;
    while streaming && !state.stopped.load(Ordering::Relaxed) {
        thread::sleep(state.period);
        let frame = state.frame.lock().unwrap().clone();
        socket.write_all(&frame)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        time::{Duration, Instant},
    };
    #[test]
    #[ignore = "requires FFmpeg"]
    fn bmp_stream_decodes_with_ffmpeg() -> Result<()> {
        let server = FrameServer::start(30)?;
        server.publish(crate::wallpaper::frame::bmp(
            &[200; 80 * 44 * 3],
            (80, 22),
            crate::render::Mode::Blocks,
            true,
        )?);
        let tools = crate::media::Tools::find(std::path::Path::new(env!("CARGO_MANIFEST_DIR")));
        let mut child = crate::media::command(&tools.ffmpeg)
            .args([
                "-v",
                "error",
                "-f",
                "bmp_pipe",
                "-framerate",
                "30",
                "-probesize",
                "32",
                "-analyzeduration",
                "0",
            ])
            .arg("-i")
            .arg(format!("{}/stream.bmp", server.url()))
            .args(["-frames:v", "6", "-f", "null", "-"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = child.try_wait()? {
                anyhow::ensure!(status.success(), "FFmpeg rejected BMP stream");
                break;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                anyhow::bail!("FFmpeg stream stalled");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
    #[test]
    fn private_http_frame_and_shutdown() {
        let server = FrameServer::start(30).unwrap();
        let url = server.url();
        let (address, token) = url
            .strip_prefix("http://")
            .unwrap()
            .split_once('/')
            .unwrap();
        let picture =
            crate::wallpaper::frame::bmp(&[255; 6], (1, 1), crate::render::Mode::Blocks, true)
                .unwrap();
        server.publish(picture.clone());
        let request = |path: &str| {
            let mut client = TcpStream::connect(address).unwrap();
            client
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            write!(client, "GET {path} HTTP/1.0\r\n\r\n").unwrap();
            let mut out = Vec::new();
            client.read_to_end(&mut out).unwrap();
            out
        };
        let frame = request(&format!("/{token}/frame.bmp?t=1"));
        assert!(frame.starts_with(b"HTTP/1.0 200"));
        assert!(frame.ends_with(&picture));
        assert!(request("/wrong/frame.bmp").starts_with(b"HTTP/1.0 404"));
        assert!(request(&format!("/{token}/../../config.toml")).starts_with(b"HTTP/1.0 404"));
        let stalled = TcpStream::connect(address).unwrap();
        let start = Instant::now();
        drop(server);
        assert!(start.elapsed() < Duration::from_secs(3));
        drop(stalled);
        assert!(TcpStream::connect(address).is_err());
    }
}
