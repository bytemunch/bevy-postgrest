use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use bevy_gotrue::{is_logged_in, AuthCreds, AuthPlugin, Client as AuthClient, Session};
use bevy_http_client::{
    prelude::{HttpTypedRequestTrait, TypedRequest, TypedResponse, TypedResponseError},
    HttpClient, HttpClientPlugin,
};
use bevy_postgrest::{Client, PostgrestPlugin};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[derive(Serialize)]
struct NewTodoTask {
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
                read_every_second
                    .run_if(on_timer(Duration::from_secs(1)))
                    .run_if(is_logged_in),
                postgres_recv,
                postgres_err,
                write_every_three_seconds
                    .run_if(is_logged_in)
                    .run_if(on_timer(Duration::from_secs(3))),
            ),
        )
        .register_request_type::<TodoTaskList>();

    app.run();
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

fn read_every_second(
    client: Res<Client>,
    mut evw: EventWriter<TypedRequest<TodoTaskList>>,
    session: Res<Session>,
) {
    let mut req = client.from("todos").select("*");

    req = req.auth(session.access_token.clone());

    let req = req.build();

    let req = HttpClient::new().request(req).with_type::<TodoTaskList>();

    evw.send(req);
}

fn write_every_three_seconds(
    client: Res<Client>,
    mut evw: EventWriter<TypedRequest<TodoTaskList>>,
    session: Res<Session>,
) {
    let mut req = client.from("todos").insert(
        json!(NewTodoTask {
            is_complete: false,
            task: "this is a new task".into(),
            user_id: session.user.id.parse().unwrap()
        })
        .to_string(),
    );

    req = req.auth(session.access_token.clone());

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
        println!("\n");
    }
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
