use std::{fs, path::PathBuf, process::Command, time::Duration};

use serde_json::{Value, json};

fn isolation_violation(configuration: &Value) -> Option<&'static str> {
    let app = &configuration["services"]["app"];
    if !app["ports"].is_null()
        && app["ports"]
            .as_array()
            .is_none_or(|ports| !ports.is_empty())
    {
        return Some("app must not publish host ports");
    }
    if !app["network_mode"].is_null() {
        return Some("app must not override its network namespace");
    }
    if !app["networks"]
        .as_object()
        .is_some_and(|networks| networks.len() == 1 && networks.contains_key("backend"))
    {
        return Some("app must join only the backend network");
    }

    let backend = &configuration["networks"]["backend"];
    if backend["internal"] != true {
        return Some("backend must be internal");
    }
    if !backend["external"].is_null() && backend["external"] != false {
        return Some("backend must not reuse an external network");
    }
    if !backend["driver"].is_null() && backend["driver"] != "bridge" {
        return Some("backend must use the bridge driver");
    }
    if !backend["driver_opts"].is_null()
        && backend["driver_opts"]
            .as_object()
            .is_none_or(|options| !options.is_empty())
    {
        return Some("backend driver options require a separate isolation review");
    }

    let proxy = &configuration["services"]["proxy"];
    if !proxy["networks"]
        .as_object()
        .is_some_and(|networks| networks.contains_key("backend"))
    {
        return Some("proxy must share the backend network");
    }
    let edge = &configuration["networks"]["edge"];
    if !proxy["networks"]
        .as_object()
        .is_some_and(|networks| networks.contains_key("edge"))
        || !edge.is_object()
        || (!edge["internal"].is_null() && edge["internal"] != false)
    {
        return Some("proxy must join a non-internal edge network for published ports");
    }
    if proxy["environment"]["APP_UPSTREAM"] != "app:8080" {
        return Some("proxy must target app:8080");
    }
    None
}

fn isolated_configuration() -> Value {
    json!({
        "services": {
            "app": {"networks": {"backend": {}}},
            "proxy": {
                "networks": {"backend": {}, "edge": {}},
                "environment": {"APP_UPSTREAM": "app:8080"}
            }
        },
        "networks": {"backend": {"internal": true}, "edge": {"internal": false}}
    })
}

#[test]
fn accepts_private_backend_configuration() {
    assert_eq!(isolation_violation(&isolated_configuration()), None);
}

#[test]
fn rejects_published_ports_including_loopback() {
    for host_ip in ["0.0.0.0", "127.0.0.1", "::"] {
        let mut configuration = isolated_configuration();
        configuration["services"]["app"]["ports"] =
            json!([{"target": 8080, "published": "8080", "host_ip": host_ip}]);
        assert_eq!(
            isolation_violation(&configuration),
            Some("app must not publish host ports")
        );
    }
}

#[test]
fn rejects_alternate_app_networks() {
    for network_mode in ["host", "service:proxy", "container:other"] {
        let mut configuration = isolated_configuration();
        configuration["services"]["app"]["network_mode"] = json!(network_mode);
        assert!(isolation_violation(&configuration).is_some());
    }
    let mut configuration = isolated_configuration();
    configuration["services"]["app"]["networks"]["default"] = json!({});
    assert!(isolation_violation(&configuration).is_some());
}

#[test]
fn rejects_unsafe_backend_options() {
    for (setting, value) in [
        ("internal", json!(false)),
        ("external", json!(true)),
        ("driver", json!("macvlan")),
        (
            "driver_opts",
            json!({"com.docker.network.bridge.gateway_mode_ipv4": "nat-unprotected"}),
        ),
    ] {
        let mut configuration = isolated_configuration();
        configuration["networks"]["backend"][setting] = value;
        assert!(isolation_violation(&configuration).is_some(), "{setting}");
    }
}

#[test]
fn rejects_missing_or_disconnected_services() {
    assert!(isolation_violation(&json!({})).is_some());
    let mut configuration = isolated_configuration();
    configuration["services"]["proxy"]["networks"] = json!({"default": {}});
    assert!(isolation_violation(&configuration).is_some());
    let mut configuration = isolated_configuration();
    configuration["services"]["proxy"]["networks"] = json!({"backend": {}});
    assert!(isolation_violation(&configuration).is_some());
    let mut configuration = isolated_configuration();
    configuration["networks"]["edge"]["internal"] = json!(true);
    assert!(isolation_violation(&configuration).is_some());
    let mut configuration = isolated_configuration();
    configuration["services"]["proxy"]["environment"]["APP_UPSTREAM"] = json!("127.0.0.1:8080");
    assert!(isolation_violation(&configuration).is_some());
}

fn compose_configuration() -> Result<Value, Box<dyn std::error::Error>> {
    let output = Command::new("docker")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "compose",
            "--project-name",
            "boundary-check",
            "-f",
            "docker-compose.yml",
            "config",
            "--format",
            "json",
        ])
        .env_remove("COMPOSE_ENV_FILES")
        .env("COMPOSE_DISABLE_ENV_FILE", "1")
        .env("APP_ADMIN_PASSWORD_HASH", "boundary-test-not-a-real-hash")
        .env("APP_ADMIN_ORIGIN", "https://invite.example.com")
        .env("CADDY_ADMIN_PASSWORD_HASH", "boundary-test-not-a-real-hash")
        .env("APP_LDAP_HTTP_URL", "http://directory:17170")
        .env("APP_LDAP_LDAP_URL", "ldap://directory:3890")
        .env("APP_LDAP_BASE_DN", "dc=example,dc=com")
        .env("APP_LDAP_USERNAME", "boundary-test")
        .env("APP_LDAP_PASSWORD", "boundary-test-not-a-real-password")
        .env("APP_LDAP_USE_TLS", "false")
        .env("APP_LDAP_TLS_INSECURE_SKIP_VERIFY", "false")
        .output()?;
    assert!(output.status.success(), "Docker Compose config failed");
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
#[ignore = "requires the Docker Compose CLI, but no running Docker daemon"]
fn compose_keeps_backend_private() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(isolation_violation(&compose_configuration()?), None);
    Ok(())
}

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn command_stdout(command: &mut Command) -> TestResult<String> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "test command failed ({}): {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

struct LiveDeployment {
    directory: PathBuf,
    project: String,
}

impl LiveDeployment {
    fn compose(&self) -> Command {
        let mut command = Command::new("docker");
        command
            .args(["compose", "--project-name", &self.project, "-f"])
            .arg(self.directory.join("compose.json"))
            .env_remove("COMPOSE_ENV_FILES")
            .env("COMPOSE_DISABLE_ENV_FILE", "1");
        command
    }

    fn cleanup(&self) -> TestResult {
        if self.directory.join("compose.json").is_file() {
            command_stdout(self.compose().args([
                "--profile",
                "probe",
                "down",
                "--volumes",
                "--remove-orphans",
            ]))?;
        }
        if self.directory.is_dir() {
            fs::remove_dir_all(&self.directory)?;
        }
        Ok(())
    }
}

impl Drop for LiveDeployment {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("cleanup failed for {}: {error}", self.project);
        }
    }
}

#[tokio::test]
#[ignore = "requires Docker, OpenSSL, and a freshly built BOUNDARY_APP_IMAGE"]
async fn live_proxy_and_backend_isolation() -> TestResult {
    let image = std::env::var("BOUNDARY_APP_IMAGE")?;
    let openssl = std::env::var_os("BOUNDARY_OPENSSL").unwrap_or_else(|| "openssl".into());
    let project = format!("boundary-{}", uuid::Uuid::new_v4().simple());
    let deployment = LiveDeployment {
        directory: std::env::temp_dir().join(&project),
        project,
    };
    fs::create_dir(&deployment.directory)?;
    let certificate_path = deployment.directory.join("fullchain.pem");
    command_stdout(
        Command::new(openssl)
            .args([
                "req", "-x509", "-newkey", "rsa:2048", "-nodes", "-days", "1",
            ])
            .args(["-subj", "/CN=invite.example.com"])
            .args(["-addext", "subjectAltName=DNS:invite.example.com"])
            .args(["-addext", "basicConstraints=critical,CA:FALSE"])
            .arg("-keyout")
            .arg(deployment.directory.join("privkey.pem"))
            .arg("-out")
            .arg(&certificate_path),
    )?;
    let proxy_password = uuid::Uuid::new_v4().to_string();
    let proxy_hash = command_stdout(Command::new("docker").args([
        "run",
        "--rm",
        "caddy:2.8",
        "caddy",
        "hash-password",
        "--plaintext",
        &proxy_password,
    ]))?;
    let mut configuration = compose_configuration()?;
    assert_eq!(isolation_violation(&configuration), None);
    configuration["name"] = json!(deployment.project);
    configuration["volumes"] = json!({"app-data": {}, "caddy-data": {}, "caddy-config": {}});
    configuration["networks"] = json!({
        "backend": {"internal": true}, "edge": {"internal": false}, "untrusted": {}
    });
    let app = &mut configuration["services"]["app"];
    app.as_object_mut()
        .ok_or("missing app service")?
        .remove("build");
    app["image"] = json!(image);
    app["volumes"] = json!([{"type": "volume", "source": "app-data", "target": "/app/data"}]);
    app["environment"]["APP__ADMIN__PASSWORD_HASH"] = json!(
        rust_invite_system::admin_auth::hash_password("throwaway boundary admin password")?
    );
    let proxy = &mut configuration["services"]["proxy"];
    proxy["environment"]["CADDY_ADMIN_PASSWORD_HASH"] = json!(proxy_hash);
    proxy["ports"] =
        json!([{"target": 443, "published": "0", "host_ip": "127.0.0.1", "protocol": "tcp"}]);
    for volume in proxy["volumes"]
        .as_array_mut()
        .ok_or("missing proxy mounts")?
    {
        if volume["target"] == "/etc/caddy/certs" {
            volume["source"] = json!(deployment.directory);
        }
    }
    configuration["services"]["probe"] = json!({
        "image": "curlimages/curl:8.12.1",
        "profiles": ["probe"],
        "networks": {"untrusted": {}}
    });
    fs::write(
        deployment.directory.join("compose.json"),
        serde_json::to_string(&configuration)?.replace('$', "$$"),
    )?;
    command_stdout(deployment.compose().args([
        "up",
        "-d",
        "--wait",
        "--wait-timeout",
        "60",
        "app",
        "proxy",
    ]))?;

    let app_id = command_stdout(deployment.compose().args(["ps", "-q", "app"]))?;
    let inspected: Value = serde_json::from_str(&command_stdout(
        Command::new("docker").args(["inspect", &app_id]),
    )?)?;
    let runtime = &inspected[0];
    let bindings = &runtime["HostConfig"]["PortBindings"];
    assert!(bindings.is_null() || bindings.as_object().is_some_and(|ports| ports.is_empty()));
    assert!(
        runtime["NetworkSettings"]["Ports"]
            .as_object()
            .ok_or("missing runtime ports")?
            .values()
            .all(Value::is_null)
    );
    let networks = runtime["NetworkSettings"]["Networks"]
        .as_object()
        .ok_or("missing runtime networks")?;
    assert_eq!(networks.len(), 1);
    let (backend_name, backend_endpoint) = networks.iter().next().ok_or("no backend network")?;
    let backend: Value = serde_json::from_str(&command_stdout(Command::new("docker").args([
        "network",
        "inspect",
        backend_name,
    ]))?)?;
    assert_eq!(backend[0]["Internal"], true);
    assert_eq!(backend[0]["Driver"], "bridge");
    assert_eq!(
        backend[0]["Containers"]
            .as_object()
            .ok_or("no network members")?
            .len(),
        2
    );
    assert_eq!(
        command_stdout(deployment.compose().args(["exec", "-T", "app", "id", "-u"]))?,
        "10001"
    );

    let published = command_stdout(deployment.compose().args(["port", "proxy", "443"]))?;
    let address: std::net::SocketAddr = published
        .parse()
        .map_err(|_| format!("unexpected proxy port mapping: {published:?}"))?;
    assert!(address.ip().is_loopback());
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(5))
        .add_root_certificate(reqwest::Certificate::from_pem(&fs::read(
            certificate_path,
        )?)?)
        .resolve("invite.example.com", address)
        .build()?;
    let origin = format!("https://invite.example.com:{}", address.port());
    for path in ["/admin", "/admin/login"] {
        let response = client.get(format!("{origin}{path}")).send().await?;
        assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert!(response.headers().contains_key("www-authenticate"));
        let forged = client
            .get(format!("{origin}{path}"))
            .header("X-Forwarded-User", "admin")
            .header("Remote-User", "admin")
            .header("X-Forwarded-For", "127.0.0.1")
            .send()
            .await?;
        assert_eq!(forged.status(), reqwest::StatusCode::UNAUTHORIZED);
        assert!(forged.headers().contains_key("www-authenticate"));
    }
    let login = client
        .get(format!("{origin}/admin/login"))
        .basic_auth("admin", Some(&proxy_password))
        .send()
        .await?;
    assert_eq!(login.status(), reqwest::StatusCode::OK);
    assert_eq!(login.headers()["cache-control"], "no-store");
    assert_eq!(login.headers()["referrer-policy"], "no-referrer");
    assert!(login.text().await?.contains("password"));
    let admin = client
        .get(format!("{origin}/admin"))
        .basic_auth("admin", Some(&proxy_password))
        .send()
        .await?;
    assert_eq!(admin.status(), reqwest::StatusCode::UNAUTHORIZED);
    assert!(!admin.headers().contains_key("www-authenticate"));

    let backend_ip = backend_endpoint["IPAddress"]
        .as_str()
        .ok_or("missing backend IP")?;
    let backend_url = format!("http://{backend_ip}:8080/admin/login");
    let probe_args = [
        "--noproxy",
        "*",
        "--connect-timeout",
        "3",
        "--max-time",
        "5",
        "--silent",
        "--output",
        "/dev/null",
        "--write-out",
        "%{http_code}",
        &backend_url,
    ];
    let trusted = command_stdout(
        Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network",
                backend_name,
                "curlimages/curl:8.12.1",
            ])
            .args(probe_args),
    )?;
    assert_eq!(trusted, "200", "trusted probe must reach the live app");
    let untrusted = deployment
        .compose()
        .args(["run", "--rm", "--no-deps", "probe"])
        .args(probe_args)
        .output()?;
    assert!(
        matches!(untrusted.status.code(), Some(7 | 28)),
        "expected connection refusal or timeout"
    );
    assert_eq!(String::from_utf8(untrusted.stdout)?.trim(), "000");
    println!(
        "Verified TLS, proxy challenges, forged-header rejection, application auth, UID 10001, no app port mappings, and cross-network IPv4 isolation. External-machine and IPv6 probes remain separate."
    );
    deployment.cleanup()?;
    Ok(())
}
