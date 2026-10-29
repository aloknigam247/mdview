use std::io;
use std::process::{ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

#[cfg(windows)]
use process_wrap::tokio::JobObject;
use process_wrap::tokio::{CommandWrap, KillOnDrop};
use tokio::process::Command;

pub async fn run(mut command: Command, limit: Duration) -> io::Result<(Output, Duration)> {
    let stderr_file = tempfile::NamedTempFile::new()?;
    let stdout_file = tempfile::NamedTempFile::new()?;
    // Inherited output handles must not keep a pipe reader waiting for EOF.
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout_file.reopen()?))
        .stderr(Stdio::from(stderr_file.reopen()?));
    let mut command = CommandWrap::from(command);
    command.wrap(KillOnDrop);
    #[cfg(windows)]
    command.wrap(JobObject);

    let started = Instant::now();
    let mut child = command.spawn()?;
    let pid = child.id();
    let parent = tokio::time::timeout(limit, async {
        loop {
            // The job outlives the daemonizing parent.
            if let Some(status) = child.inner_mut().try_wait()? {
                return Ok::<ExitStatus, io::Error>(status);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;
    let elapsed = started.elapsed();

    child.start_kill()?;
    tokio::time::timeout(Duration::from_secs(10), child.wait())
        .await
        .map_err(|_| {
            io::Error::new(io::ErrorKind::TimedOut, "owned process cleanup timed out")
        })??;

    let stderr = std::fs::read(stderr_file.path())?;
    let stdout = std::fs::read(stdout_file.path())?;
    let status = match parent {
        Ok(result) => result?,
        Err(_) => {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "process {pid:?} exceeded {limit:?}; stdout: {}; stderr: {}",
                    String::from_utf8_lossy(&stdout),
                    String::from_utf8_lossy(&stderr)
                ),
            ));
        }
    };
    Ok((
        Output {
            status,
            stderr,
            stdout,
        },
        elapsed,
    ))
}
