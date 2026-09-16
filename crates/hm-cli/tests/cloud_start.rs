use rcgen::{
    BasicConstraints, CertificateParams, CertifiedIssuer, ExtendedKeyUsagePurpose, IsCa, KeyPair,
    KeyUsagePurpose,
};
use std::fs;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_hm"));
    for key in [
        "HM_TLS_CERT_PEM",
        "HM_TLS_KEY_PEM",
        "HM_TLS_CLIENT_CA_PEM",
        "HM_CONSOLIDATION_PROVIDER",
        "HM_RECONSTRUCTION_PROVIDER",
        "HM_EMBEDDING_PROVIDER",
    ] {
        command.env_remove(key);
    }
    command
}

struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn certificates() -> (String, String, String) {
    let mut ca = CertificateParams::new(Vec::<String>::new()).unwrap();
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca = CertifiedIssuer::self_signed(ca, KeyPair::generate().unwrap()).unwrap();
    let mut server = CertificateParams::new(vec!["localhost".to_owned()]).unwrap();
    server.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    let key = KeyPair::generate().unwrap();
    let certificate = server.signed_by(&key, &ca).unwrap();
    (certificate.pem(), key.serialize_pem(), ca.pem())
}

#[test]
fn help_and_version_exit_success_and_invalid_arguments_fail() {
    for argument in ["--help", "--version"] {
        assert!(command().arg(argument).output().unwrap().status.success());
    }
    assert!(
        !command()
            .arg("--not-a-real-option")
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn missing_env_and_existing_invalid_config_never_initialize_new_keys() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("hypermind.conf");
    let result = command()
        .args(["serve", "--config"])
        .arg(&config)
        .args([
            "--init-if-missing",
            "--grpc-bind",
            "127.0.0.1:0",
            "--tls-from-env",
        ])
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(!config.exists());
    assert!(!directory.path().join("data").exists());
    fs::write(&config, "invalid existing configuration\n").unwrap();
    let result = command()
        .args(["serve", "--config"])
        .arg(&config)
        .arg("--init-if-missing")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert_eq!(
        fs::read_to_string(&config).unwrap(),
        "invalid existing configuration\n"
    );
    assert!(!directory.path().join("data").exists());
}

#[test]
fn env_tls_first_start_restart_and_sigterm_preserve_real_actor_keys() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("hypermind.conf");
    let (certificate, key, ca) = certificates();
    let mut original = None;
    for _ in 0..2 {
        let child = command()
            .args(["serve", "--config"])
            .arg(&config)
            .args([
                "--init-if-missing",
                "--grpc-bind",
                "127.0.0.1:0",
                "--tls-from-env",
                "--json",
            ])
            .env("HM_TLS_CERT_PEM", &certificate)
            .env("HM_TLS_KEY_PEM", &key)
            .env("HM_TLS_CLIENT_CA_PEM", &ca)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut running = Running(child);
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            assert!(Instant::now() < deadline, "daemon did not become healthy");
            assert!(
                running.0.try_wait().unwrap().is_none(),
                "daemon exited during bootstrap"
            );
            if config.exists()
                && command()
                    .args(["doctor", "--config"])
                    .arg(&config)
                    .args(["--require-healthy", "--json"])
                    .output()
                    .unwrap()
                    .status
                    .success()
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let loaded = hm_serve::config::load(&config).unwrap();
        let identity = (loaded.kek, loaded.admin_token, loaded.actors[0].token);
        if let Some(expected) = original {
            assert_eq!(identity, expected);
        } else {
            original = Some(identity);
        }
        assert!(!directory.path().join("tls.key").exists());
        assert!(
            Command::new("kill")
                .args(["-TERM", &running.0.id().to_string()])
                .status()
                .unwrap()
                .success()
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = running.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(Instant::now() < deadline, "daemon did not stop gracefully");
            std::thread::sleep(Duration::from_millis(25));
        }
    }
}
