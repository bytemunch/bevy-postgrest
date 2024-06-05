use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_gotrue::{is_logged_in, AuthCreds, AuthPlugin, Client as AuthClient};
use bevy_http_client::{
    prelude::{HttpTypedRequestTrait, TypedRequest, TypedResponse, TypedResponseError},
    HttpClient, HttpClientPlugin,
};
use bevy_postgrest::{Client, PostgrestPlugin};
use chrono::DateTime;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Event, Debug, Deserialize)]
pub struct TodoTaskList(Vec<TodoTask>);

#[derive(Debug, Deserialize)]
struct TodoTask {
    id: i8,
    inserted_at: DateTime<chrono::Local>,
    is_complete: bool,
    task: String,
    user_id: Uuid,
}

fn main() {
    let endpoint = "http://127.0.0.1:54321/rest/v1".into();
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_plugins(HttpClientPlugin)
        .add_plugins(PostgrestPlugin { endpoint })
        .add_plugins(AuthPlugin {
            endpoint: "http://127.0.0.1:54321/auth/v1".into(),
        })
        .add_systems(Startup, (setup,))
        .add_systems(
            Update,
            (
                send_every_second
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(is_logged_in),
                postgres_recv,
                postgres_err,
            ),
        )
        .register_request_type::<TodoTaskList>();

    app.run()
}

fn setup(mut commands: Commands, auth: Res<AuthClient>) {
    auth.sign_in(
        &mut commands,
        AuthCreds {
            id: "test@example.com".into(),
            password: "password".into(),
        },
    );
}

fn send_every_second(
    client: Res<Client>,
    mut evw: EventWriter<TypedRequest<TodoTaskList>>,
    auth: Option<Res<AuthClient>>,
) {
    let mut req = client.from("todos").select("*");

    if let Some(auth) = auth {
        if let Some(token) = auth.access_token.clone() {
            req = req.auth(token);
        }
    }

    let req = req.build();

    let req = HttpClient::new().request(req).with_type::<TodoTaskList>();

    evw.send(req);
}

fn postgres_recv(mut evr: EventReader<TypedResponse<TodoTaskList>>) {
    for ev in evr.read() {
        for task in &ev.0 {
            println!(
                "[TASK] {} {} {} {} {}",
                task.id, task.task, task.is_complete, task.inserted_at, task.user_id
            );
        }
    }
    println!("\n");
}

fn postgres_err(mut evr: EventReader<TypedResponseError<TodoTaskList>>) {
    for ev in evr.read() {
        println!("[ERR] {:?}", ev);
        if let Some(res) = &ev.response {
            if let Ok(body) = std::str::from_utf8(res.bytes.as_slice()) {
                println!("[BODY] {:?}", body);
            }
        }
    }
}
