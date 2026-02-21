use gloo_net::http::Request;
use todo_shared::{CreateTodoRequest, DeleteResponse, SectionCount, Todo, UpdateTodoRequest};

fn api_base() -> String {
    let location = web_sys::window().unwrap().location();
    let origin = location.origin().unwrap();
    format!("{}/api", origin)
}

pub async fn fetch_sections() -> Result<Vec<SectionCount>, String> {
    let url = format!("{}/sections", api_base());
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_todos(section: Option<&str>) -> Result<Vec<Todo>, String> {
    let mut url = format!("{}/todos", api_base());
    if let Some(s) = section {
        url.push_str(&format!("?section={}", s));
    }
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
    let url = format!("{}/todos/{}/toggle", api_base(), id);
    let resp = Request::post(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn delete_todo(id: &str) -> Result<DeleteResponse, String> {
    let url = format!("{}/todos/{}", api_base(), id);
    let resp = Request::delete(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_all_by_importance() -> Result<Vec<Todo>, String> {
    let url = format!("{}/todos?sort=importance", api_base());
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn update_todo(id: &str, req: &UpdateTodoRequest) -> Result<Todo, String> {
    let url = format!("{}/todos/{}", api_base(), id);
    let resp = Request::patch(&url)
        .json(req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}
