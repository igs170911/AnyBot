use std::net::{Ipv6Addr, TcpListener as StdTcpListener};

pub fn bind_v4() -> Result<(StdTcpListener, u16), String> {
    let listener = StdTcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("failed to bind an ephemeral IPv4 port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to read the IPv4 port: {e}"))?
        .port();
    Ok((listener, port))
}

pub fn bind_v6() -> Result<(StdTcpListener, u16), String> {
    let listener = StdTcpListener::bind((Ipv6Addr::LOCALHOST, 0))
        .map_err(|e| format!("failed to bind an ephemeral IPv6 port: {e}"))?;
    let port = listener
        .local_addr()
        .map_err(|e| format!("failed to read the IPv6 port: {e}"))?
        .port();
    Ok((listener, port))
}

pub fn bind_pair() -> Result<(StdTcpListener, StdTcpListener, u16), String> {
    let (v4, port) = bind_v4()?;
    let v6 = StdTcpListener::bind((Ipv6Addr::LOCALHOST, port))
        .map_err(|e| format!("failed to bind IPv6 port {port}: {e}"))?;
    Ok((v4, v6, port))
}

pub fn free_dual_port() -> Result<u16, String> {
    let (v4, v6, port) = bind_pair()?;
    drop(v4);
    drop(v6);
    Ok(port)
}

pub fn first_lan_ipv4() -> Result<String, String> {
    let text = lan_listing()?;
    text.lines()
        .map(str::trim)
        .find(|ip| !ip.is_empty() && !ip.starts_with("127."))
        .map(str::to_owned)
        .ok_or("no non-loopback IPv4 interface".to_owned())
}

#[cfg(unix)]
fn lan_listing() -> Result<String, String> {
    let helper = lan_helper()?;
    let output = std::process::Command::new(&helper)
        .output()
        .map_err(|e| format!("list interfaces failed: {e}"))?;
    if !output.status.success() {
        return Err("list interfaces was rejected".to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(windows)]
fn lan_listing() -> Result<String, String> {
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-NetIPAddress -AddressFamily IPv4 | ForEach-Object { $_.IPAddress }",
        ])
        .output()
        .map_err(|e| format!("list interfaces failed: {e}"))?;
    if !output.status.success() {
        return Err("list interfaces was rejected".to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(unix)]
fn lan_helper() -> Result<std::path::PathBuf, String> {
    let source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lan_ipv4.c");
    let out_dir = crate::scratch::repo_root()
        .join("target")
        .join("tests")
        .join("signal-helper");
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("create lan helper dir failed: {e}"))?;
    let binary = out_dir.join("lan_ipv4");
    let built = std::process::Command::new("cc")
        .args(["-O2", "-o"])
        .arg(&binary)
        .arg(&source)
        .status()
        .map_err(|e| format!("build lan helper failed: {e}"))?;
    if !built.success() {
        return Err("build lan helper was rejected".to_owned());
    }
    Ok(binary)
}

#[must_use]
pub fn port_is_free(port: u16) -> bool {
    StdTcpListener::bind(("127.0.0.1", port)).is_ok()
        && StdTcpListener::bind((Ipv6Addr::LOCALHOST, port)).is_ok()
}
