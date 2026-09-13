use std::{env, time::Duration};

use ldap3::{LdapConnAsync, exop::PasswordModify};
use rust_invite_system::{configuration::LldapConfig, lldap::LldapClient};
use serde::Deserialize;
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

const HTTP_PORT: u16 = 17170;
const LDAP_PORT: u16 = 3890;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatedUser {
    id: String,
    email: String,
    first_name: String,
    last_name: String,
}

#[derive(Deserialize)]
struct CreateUserData {
    #[serde(rename = "createUser")]
    create_user: CreatedUser,
}

#[tokio::test]
#[ignore = "requires a local Docker daemon and pulls lldap/lldap:latest"]
async fn latest_lldap_container_serves_http() {
    dotenvy::from_filename("dev.env").expect("dev.env must exist at the repository root");

    let user_dn = env::var("LLDAP_LDAP_USER_DN").expect("LLDAP_LDAP_USER_DN is required");
    let user_password = env::var("LLDAP_LDAP_USER_PASS").expect("LLDAP_LDAP_USER_PASS is required");
    let base_dn = env::var("LLDAP_LDAP_BASE_DN").expect("LLDAP_LDAP_BASE_DN is required");
    let jwt_secret = env::var("LLDAP_JWT_SECRET").expect("LLDAP_JWT_SECRET is required");

    let container = GenericImage::new("lldap/lldap", "latest")
        .with_exposed_port(HTTP_PORT.tcp())
        .with_wait_for(WaitFor::seconds(1))
        .with_env_var("LLDAP_LDAP_USER_DN", user_dn)
        .with_env_var("LLDAP_LDAP_USER_PASS", user_password)
        .with_env_var("LLDAP_LDAP_BASE_DN", base_dn)
        .with_env_var("LLDAP_JWT_SECRET", jwt_secret)
        .with_env_var("LLDAP_HTTP_PORT", HTTP_PORT.to_string())
        .with_env_var("LLDAP_LDAP_PORT", "3890")
        .start()
        .await
        .expect("failed to start lldap/lldap:latest");

    let host_port = container
        .get_host_port_ipv4(HTTP_PORT.tcp())
        .await
        .expect("LLDAP HTTP port was not mapped");
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{host_port}/");

    for _ in 0..60 {
        if let Ok(response) = client.get(&url).send().await {
            assert!(!response.status().is_server_error());
            return;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    panic!("LLDAP did not respond at {url} within 30 seconds");
}

#[tokio::test]
#[ignore = "requires a local Docker daemon and pulls lldap/lldap:latest"]
async fn latest_lldap_can_create_user_and_set_password() {
    dotenvy::from_filename("dev.env").expect("dev.env must exist at the repository root");

    let user_dn = env::var("LLDAP_LDAP_USER_DN").expect("LLDAP_LDAP_USER_DN is required");
    let user_password = env::var("LLDAP_LDAP_USER_PASS").expect("LLDAP_LDAP_USER_PASS is required");
    let base_dn = env::var("LLDAP_LDAP_BASE_DN").expect("LLDAP_LDAP_BASE_DN is required");
    let jwt_secret = env::var("LLDAP_JWT_SECRET").expect("LLDAP_JWT_SECRET is required");
    let admin_dn = format!("cn=admin,ou=people,{base_dn}");
    let test_user = format!("invite-test-{}", std::process::id());
    let test_password = "InviteTest-password-2026!";

    let container = GenericImage::new("lldap/lldap", "latest")
        .with_exposed_port(HTTP_PORT.tcp())
        .with_exposed_port(LDAP_PORT.tcp())
        .with_wait_for(WaitFor::seconds(1))
        .with_env_var("LLDAP_LDAP_USER_DN", user_dn.clone())
        .with_env_var("LLDAP_LDAP_USER_PASS", user_password.clone())
        .with_env_var("LLDAP_LDAP_BASE_DN", base_dn.clone())
        .with_env_var("LLDAP_JWT_SECRET", jwt_secret)
        .with_env_var("LLDAP_HTTP_PORT", HTTP_PORT.to_string())
        .with_env_var("LLDAP_LDAP_PORT", LDAP_PORT.to_string())
        .start()
        .await
        .expect("failed to start lldap/lldap:latest");

    let http_port = container
        .get_host_port_ipv4(HTTP_PORT.tcp())
        .await
        .expect("LLDAP HTTP port was not mapped");
    let ldap_port = container
        .get_host_port_ipv4(LDAP_PORT.tcp())
        .await
        .expect("LLDAP LDAP port was not mapped");
    let base_url = format!("http://127.0.0.1:{http_port}");
    let test_user_dn = format!("uid={test_user},ou=people,{base_dn}");

    let lldap = LldapClient::new(LldapConfig {
        http_url: base_url.clone(),
        ldap_url: format!("ldap://127.0.0.1:{ldap_port}"),
        use_tls: false,
        tls_insecure_skip_verify: false,
        tls_ca_file: None,
        base_dn: base_dn.clone(),
        username: user_dn.clone(),
        password: user_password.clone(),
    });

    let login = lldap
        .login(&user_dn, &user_password)
        .await
        .expect("LLDAP login request failed");

    let create_query = r#"
        mutation CreateUser($user: CreateUserInput!) {
            createUser(user: $user) { id email firstName lastName }
        }
    "#;
    let created = lldap
        .graphql::<CreateUserData>(
            &login,
            create_query,
            serde_json::json!({
                "user": {
                    "id": test_user,
                    "email": "invite-test@example.com",
                    "firstName": "Invite",
                    "lastName": "Test",
                    "displayName": "Invite Test"
                }
            }),
        )
        .await
        .expect("LLDAP createUser mutation failed");
    assert_eq!(created.create_user.id, test_user);
    assert_eq!(created.create_user.email, "invite-test@example.com");
    assert_eq!(created.create_user.first_name, "Invite");
    assert_eq!(created.create_user.last_name, "Test");

    let ldap_url = format!("ldap://127.0.0.1:{ldap_port}");
    let (connection, mut ldap) = LdapConnAsync::new(&ldap_url)
        .await
        .expect("LDAP connection failed");
    ldap3::drive!(connection);
    ldap.simple_bind(&admin_dn, &env::var("LLDAP_LDAP_USER_PASS").unwrap())
        .await
        .expect("LDAP bind request failed")
        .success()
        .expect("LDAP bind was rejected");
    ldap.extended(PasswordModify {
        user_id: Some(&test_user_dn),
        old_pass: None,
        new_pass: Some(test_password),
    })
    .await
    .expect("LDAP password modification request failed")
    .success()
    .expect("LLDAP rejected the password modification");
    ldap.unbind().await.expect("LDAP unbind failed");

    let delete_query = r#"
        mutation DeleteUser($id: String!) { deleteUser(userId: $id) { ok } }
    "#;
    lldap
        .graphql::<serde_json::Value>(&login, delete_query, serde_json::json!({ "id": test_user }))
        .await
        .expect("LLDAP deleteUser mutation failed");
}
