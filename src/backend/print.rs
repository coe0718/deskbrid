//! CUPS printer management via CLI wrappers.
//! Uses lpstat, lpadmin, cancel, lp — standard on all distros with CUPS installed.

use crate::protocol::{PrintJob, PrintPrinter};

/// List all printers (lpstat -v + lpstat -d).
pub async fn print_list() -> anyhow::Result<Vec<PrintPrinter>> {
    let output = match tokio::process::Command::new("lpstat")
        .args(["-v"])
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };
    let printers_raw = String::from_utf8_lossy(&output.stdout);

    let default_output = tokio::process::Command::new("lpstat")
        .args(["-d"])
        .output()
        .await
        .ok();
    let default_raw = default_output
        .as_ref()
        .map(|o| String::from_utf8_lossy(&o.stdout))
        .unwrap_or_default();
    let default_name = default_raw
        .strip_prefix("system default destination: ")
        .map(|s| s.trim().to_string());

    let mut printers = Vec::new();
    for line in printers_raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Format: "device for PrinterName: uri"
        let (name, uri) = if let Some(rest) = line.strip_prefix("device for ") {
            if let Some((n, u)) = rest.split_once(':') {
                (n.trim().to_string(), Some(u.trim().to_string()))
            } else {
                (rest.trim().to_string(), None)
            }
        } else {
            continue;
        };

        // Get printer status
        let status = get_printer_status(&name).await;

        printers.push(PrintPrinter {
            is_default: default_name.as_ref() == Some(&name),
            location: String::new(),
            status,
            name,
            uri,
        });
    }
    Ok(printers)
}

async fn get_printer_status(name: &str) -> String {
    let output = tokio::process::Command::new("lpstat")
        .args(["-p", name])
        .output()
        .await
        .ok();
    if let Some(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        let first_line = s.lines().next().unwrap_or("");
        // "printer PrinterName is idle." or "printer PrinterName disabled since ..."
        if first_line.contains("is idle") {
            "idle".into()
        } else if first_line.contains("disabled") {
            "disabled".into()
        } else if first_line.contains("now printing") {
            "printing".into()
        } else if first_line.contains("is ready") {
            "ready".into()
        } else {
            "unknown".into()
        }
    } else {
        "unknown".into()
    }
}

/// Get or set default printer.
pub async fn print_default(printer: Option<&str>) -> anyhow::Result<PrintPrinter> {
    if let Some(name) = printer {
        // Set default printer
        let status = tokio::process::Command::new("lpadmin")
            .args(["-d", name])
            .status()
            .await?;
        if !status.success() {
            anyhow::bail!("lpadmin -d {name} failed");
        }
    }

    // Get current default
    let output = tokio::process::Command::new("lpstat")
        .args(["-d"])
        .output()
        .await?;
    let raw = String::from_utf8_lossy(&output.stdout);
    let default_name = raw
        .strip_prefix("system default destination: ")
        .unwrap_or("")
        .trim()
        .to_string();

    let printers = print_list().await?;
    printers
        .into_iter()
        .find(|p| p.name == default_name)
        .ok_or_else(|| {
            anyhow::anyhow!("default printer '{default_name}' not found in printer list")
        })
}

/// List all print jobs (lpstat -o).
pub async fn print_jobs() -> anyhow::Result<Vec<PrintJob>> {
    let output = tokio::process::Command::new("lpstat")
        .args(["-o"])
        .output()
        .await?;
    let raw = String::from_utf8_lossy(&output.stdout);

    let mut jobs = Vec::new();
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        // Format: "PrinterName-JobID  user  size  submitted  status"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        // First part is "PrinterName-JobID"
        let (printer, job_id) = if let Some((p, j)) = parts[0].rsplit_once('-') {
            (p.to_string(), j.to_string())
        } else {
            continue;
        };
        let user = parts[1].to_string();
        let size = if parts.len() > 3 {
            Some(parts[2].to_string())
        } else {
            None
        };
        let status = if parts.len() > 4 {
            parts[4..].join(" ")
        } else {
            parts.get(3).map(|s| s.to_string()).unwrap_or_default()
        };

        jobs.push(PrintJob {
            id: job_id,
            printer,
            user,
            name: String::new(),
            size,
            status,
            submitted: None,
        });
    }
    Ok(jobs)
}

/// Cancel a print job.
pub async fn print_job_cancel(job_id: &str) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("cancel")
        .arg(job_id)
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("cancel {job_id} failed");
    }
    Ok(())
}

/// Pause a print job.
pub async fn print_job_pause(job_id: &str) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("lp")
        .args(["-i", job_id, "-H", "hold"])
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("lp -i {job_id} -H hold failed");
    }
    Ok(())
}

/// Resume a print job.
pub async fn print_job_resume(job_id: &str) -> anyhow::Result<()> {
    let status = tokio::process::Command::new("lp")
        .args(["-i", job_id, "-H", "resume"])
        .status()
        .await?;
    if !status.success() {
        anyhow::bail!("lp -i {job_id} -H resume failed");
    }
    Ok(())
}

/// Send a file to a printer (lp -d <printer> <path>).
/// Caller is responsible for path sandboxing before calling this function.
pub async fn print_file(printer: &str, path: &str) -> anyhow::Result<PrintJob> {
    let output = tokio::process::Command::new("lp")
        .args(["-d", printer, path])
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("lp -d {printer} {path} failed: {stderr}");
    }
    // lp outputs "request id is PrinterName-JobID (1 file(s))"
    let stdout = String::from_utf8_lossy(&output.stdout);
    let job_id = stdout
        .strip_prefix("request id is ")
        .and_then(|s| s.split_whitespace().next())
        .and_then(|s| s.rsplit_once('-').map(|(_, id)| id.to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    Ok(PrintJob {
        id: job_id,
        printer: printer.to_string(),
        user: std::env::var("USER").unwrap_or_else(|_| "unknown".into()),
        name: path.to_string(),
        size: None,
        status: "submitted".to_string(),
        submitted: None,
    })
}
