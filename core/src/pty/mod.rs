use anyhow::Result;
use portable_pty::{native_pty_system, Child, CommandBuilder, PtyPair, PtySize};
use std::io::Read;
use std::path::PathBuf;
use tokio::sync::mpsc;

pub struct PtySession {
    pub pair: PtyPair,
    pub child: Box<dyn Child + Send + Sync>,
}

impl PtySession {
    pub fn new(command: &str, cwd: Option<PathBuf>) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg(command);
        if let Some(cwd) = cwd {
            cmd.cwd(cwd);
        }

        let child = pair.slave.spawn_command(cmd)?;

        Ok(Self { pair, child })
    }

    pub async fn run(&self, tx: mpsc::Sender<Vec<u8>>) -> Result<()> {
        let mut reader = self.pair.master.try_clone_reader()?;
        
        tokio::task::spawn_blocking(move || {
            let mut buf = [0u8; 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if tx.blocking_send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        Ok(())
    }

    pub fn kill(&mut self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<()> {
        self.pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}
