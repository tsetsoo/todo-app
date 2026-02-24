use gloo_net::http::Request;
use todo_shared::{CreateTodoRequest, DeleteResponse, Todo, UpdateTodoRequest};

fn api_base() -> String {
    let location = web_sys::window().unwrap().location();
    let origin = location.origin().unwrap();
    format!("{origin}/api")
}

pub async fn fetch_todos(section: Option<&str>, sort: Option<&str>) -> Result<Vec<Todo>, String> {
    let base = api_base();
    let mut params = Vec::new();
    if let Some(s) = section {
        params.push(format!("section={s}"));
    }
    if let Some(s) = sort {
        params.push(format!("sort={s}"));
    }
    let url = if params.is_empty() {
        format!("{base}/todos")
    } else {
        format!("{base}/todos?{}", params.join("&"))
    };
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn create_todo(req: &CreateTodoRequest) -> Result<Todo, String> {
    let url = format!("{}/todos", api_base());
    let resp = Request::post(&url)
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn toggle_todo(id: &str) -> Result<Todo, String> {
    let url = format!("{}/todos/{id}/toggle", api_base());
    let resp = Request::post(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn update_todo(id: &str, req: &UpdateTodoRequest) -> Result<Todo, String> {
    let url = format!("{}/todos/{id}", api_base());
    let resp = Request::patch(&url)
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn delete_todo(id: &str) -> Result<DeleteResponse, String> {
    let url = format!("{}/todos/{id}", api_base());
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

