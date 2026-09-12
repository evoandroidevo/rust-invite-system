use std::{env, time::Duration};

use ldap3::{LdapConnAsync, exop::PasswordModify};
use serde::{Deserialize, Serialize};
use testcontainers::{
    GenericImage, ImageExt,
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
};

const HTTP_PORT: u16 = 17170;
const LDAP_PORT: u16 = 3890;

#[derive(Deserialize)]
struct LoginResponse {
    token: String,
}

#[derive(Deserialize)]
struct GraphqlResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphqlError>>,
}

#[derive(Deserialize)]
struct GraphqlError {
    message: String,
}

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

#[derive(Serialize)]
struct GraphqlRequest<'a> {
    query: &'a str,
    variables: serde_json::Value,
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
    let client = reqwest::Client::new();
    let base_url = format!("http://127.0.0.1:{http_port}");
    let test_user_dn = format!("uid={test_user},ou=people,{base_dn}");

    let login = client
        .post(format!("{base_url}/auth/simple/login"))
        .json(&serde_json::json!({
            "username": user_dn,
            "password": user_password,
        }))
        .send()
        .await
        .expect("LLDAP login request failed")
        .error_for_status()
        .expect("LLDAP login was rejected")
        .json::<LoginResponse>()
        .await
        .expect("LLDAP login response was invalid");

    let create_query = r#"
        mutation CreateUser($user: CreateUserInput!) {
            createUser(user: $user) { id email firstName lastName }
        }
    "#;
    let created = graphql::<CreateUserData>(
        &client,
        &base_url,
        &login.token,
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
    graphql::<serde_json::Value>(
        &client,
        &base_url,
        &login.token,
        delete_query,
        serde_json::json!({ "id": test_user }),
    )
    .await
    .expect("LLDAP deleteUser mutation failed");
}

async fn graphql<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    base_url: &str,
    token: &str,
    query: &str,
    variables: serde_json::Value,
) -> Result<T, String> {
    let response = client
        .post(format!("{base_url}/api/graphql"))
        .bearer_auth(token)
        .json(&GraphqlRequest { query, variables })
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("GraphQL request returned {status}: {body}"));
    }
    let response = serde_json::from_str::<GraphqlResponse<T>>(&body)
        .map_err(|error| format!("GraphQL response decoding failed: {error}; body: {body}"))?;

    if let Some(errors) = response.errors {
        return Err(errors
            .into_iter()
            .map(|error| error.message)
            .collect::<Vec<_>>()
            .join("; "));
    }

    response
        .data
        .ok_or_else(|| "GraphQL response had no data".to_owned())
}
