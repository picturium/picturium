use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum Frame {
    Time(String),
    Index(u32),
}

impl Frame {
    pub(super) fn variant(&self) -> String {
        match self {
            Self::Time(time) => format!("t-{}", time.replace([':', '.'], "-")),
            Self::Index(index) => format!("f-{index}"),
        }
    }
}

pub(super) async fn extract_frame(source_path: &Path, output_path: &str, frame: &Frame, extraction_timeout: Duration) -> Result<()> {
    let mut command = Command::new("ffmpeg");
    command.arg("-nostdin").arg("-v").arg("error");

    if let Frame::Time(time) = frame {
        command.arg("-ss").arg(time);
    }

    command
        .arg("-i")
        .arg(source_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-an")
        .arg("-sn")
        .arg("-dn");

    if let Frame::Index(index) = frame {
        command
            .arg("-vf")
            .arg(format!("select=eq(n\\,{index})"))
            .arg("-fps_mode")
            .arg("passthrough");
    }

    let mut child = command
        .arg("-frames:v")
        .arg("1")
        .arg("-f")
        .arg("image2")
        .arg("-c:v")
        .arg("png")
        .arg("-update")
        .arg("1")
        .arg("-y")
        .arg(output_path)
        .spawn()
        .with_context(|| "Failed to spawn ffmpeg command")?;

    let status = match timeout(extraction_timeout, child.wait()).await {
        Ok(status) => status.with_context(|| "Failed to wait on ffmpeg process")?,
        Err(_) => {
            child.kill().await.ok();
            return Err(anyhow!(
                "ffmpeg frame extraction timed out after {} seconds",
                extraction_timeout.as_secs()
            ));
        }
    };

    if !status.success() {
        return Err(anyhow!("ffmpeg failed with exit code: {:?}", status.code()));
    }

    let written = tokio::fs::metadata(output_path).await
        .is_ok_and(|metadata| metadata.len() > 0);

    match written {
        true => Ok(()),
        false => Err(anyhow!(
            "ffmpeg produced no frame for {frame:?}, the video is likely shorter than that"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variant_is_flat_enough_to_read_in_the_cache_tree() {
        assert_eq!(Frame::Time("00:01:30.5".into()).variant(), "t-00-01-30-5");
        assert_eq!(Frame::Time("5".into()).variant(), "t-5");
        assert_eq!(Frame::Index(0).variant(), "f-0");
    }

    #[test]
    fn different_selections_never_share_a_variant() {
        assert_ne!(Frame::Time("5".into()).variant(), Frame::Index(5).variant());
    }
}
