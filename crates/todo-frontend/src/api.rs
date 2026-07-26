use gloo_net::http::Request;
use todo_shared::{
    CreateDailyRequest, CreateTodoRequest, DailyStatus, DeleteResponse, SetTodoDailyRequest, Todo,
    UpdateTodoRequest,
};

fn api_base() -> String {
    let location = web_sys::window().unwrap().location();
    let origin = location.origin().unwrap();
    format!("{origin}/api")
}

/// Local calendar date YYYY-MM-DD in the browser timezone.
pub fn local_today() -> String {
    format_js_date(&js_sys::Date::new_0())
}

fn format_js_date(date: &js_sys::Date) -> String {
    let y = date.get_full_year();
    let m = date.get_month() + 1;
    let d = date.get_date();
    format!("{y:04}-{m:02}-{d:02}")
}

/// Shift a YYYY-MM-DD date by `delta` calendar days (browser local TZ).
pub fn shift_date(date: &str, delta: i32) -> String {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return local_today();
    }
    let y: u32 = parts[0].parse().unwrap_or(1970);
    let m: i32 = parts[1].parse::<i32>().unwrap_or(1);
    let d: i32 = parts[2].parse().unwrap_or(1);
    // JS Date months are 0-based; noon avoids DST edge flips.
    let js = js_sys::Date::new_with_year_month_day_hr_min_sec_milli(y, m - 1, d, 12, 0, 0, 0);
    let next = i32::try_from(js.get_date()).unwrap_or(d).saturating_add(delta);
    js.set_date(next as u32);
    format_js_date(&js)
}

pub async fn fetch_todos(section: Option<&str>, sort: Option<&str>, show: Option<&str>) -> Result<Vec<Todo>, String> {
    let base = api_base();
    let mut params = Vec::new();
    if let Some(s) = section {
        params.push(format!("section={s}"));
    }
    if let Some(s) = sort {
        params.push(format!("sort={s}"));
    }
    if let Some(s) = show {
        params.push(format!("show={s}"));
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

pub async fn fetch_daily_status(local_today: &str) -> Result<DailyStatus, String> {
    let url = format!("{}/daily/status?local_today={local_today}", api_base());
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn fetch_daily(date: &str) -> Result<Vec<Todo>, String> {
    let url = format!("{}/daily?date={date}", api_base());
    let resp = Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn create_daily(for_day: &str) -> Result<(), String> {
    let url = format!("{}/daily/create", api_base());
    let req = CreateDailyRequest {
        local_today: local_today(),
        for_day: for_day.to_string(),
    };
    let resp = Request::post(&url)
        .json(&req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() >= 400 {
        let body = resp.text().await.unwrap_or_default();
        return Err(body);
    }
    Ok(())
}

pub async fn set_todo_daily(id: &str, date: Option<&str>) -> Result<Todo, String> {
    let url = format!("{}/todos/{id}/daily", api_base());
    let req = SetTodoDailyRequest {
        date: date.map(str::to_string),
    };
    let resp = Request::post(&url)
        .json(&req)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status() == 409 {
        return Err("Create todos for today first".into());
    }
    resp.json().await.map_err(|e| e.to_string())
}

