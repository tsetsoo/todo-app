use actix_web::{web, HttpResponse};
use rusqlite::params;
use todo_shared::{CreateTodoRequest, DeleteResponse, Section, SectionCount, UpdateTodoRequest};
use uuid::Uuid;

use crate::db::DbPool;
use crate::models::row_to_todo;

const SELECT_COLS: &str = "id, section, title, completed, importance, due_date, created_at, updated_at";

pub async fn get_sections(pool: web::Data<DbPool>) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let mut counts: Vec<SectionCount> = Vec::new();
    for section in Section::all() {
        let s = section.as_str();
        let total: usize = conn
            .query_row("SELECT COUNT(*) FROM todos WHERE section = ?1", params![s], |row| row.get(0))
            .unwrap_or(0);
        let completed: usize = conn
            .query_row("SELECT COUNT(*) FROM todos WHERE section = ?1 AND completed = 1", params![s], |row| row.get(0))
            .unwrap_or(0);
        counts.push(SectionCount { section: *section, total, completed });
    }

    HttpResponse::Ok().json(counts)
}

#[derive(serde::Deserialize)]
pub struct TodosQuery {
    pub section: Option<String>,
    pub sort: Option<String>,
}

pub async fn list_todos(pool: web::Data<DbPool>, query: web::Query<TodosQuery>) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let order = match query.sort.as_deref() {
        Some("importance") => {
            "CASE importance WHEN 'critical' THEN 0 WHEN 'high' THEN 1 WHEN 'medium' THEN 2 WHEN 'low' THEN 3 END ASC, created_at DESC"
        }
        Some("due_date") => "due_date IS NULL, due_date ASC, created_at DESC",
        _ => "created_at DESC",
    };

    let todos = if let Some(ref section_str) = query.section {
        if Section::parse(section_str).is_none() {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Invalid section"}));
        }
        let sql = format!("SELECT {} FROM todos WHERE section = ?1 ORDER BY {}", SELECT_COLS, order);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map(params![section_str], row_to_todo)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>()
    } else {
        let sql = format!("SELECT {} FROM todos ORDER BY {}", SELECT_COLS, order);
        let mut stmt = conn.prepare(&sql).unwrap();
        stmt.query_map([], row_to_todo)
            .unwrap()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>()
    };

    HttpResponse::Ok().json(todos)
}

pub async fn get_todo(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let sql = format!("SELECT {} FROM todos WHERE id = ?1", SELECT_COLS);
    let result = conn.query_row(&sql, params![id], row_to_todo);

    match result {
        Ok(todo) => HttpResponse::Ok().json(todo),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            HttpResponse::NotFound().json(serde_json::json!({"error": "Todo not found"}))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn create_todo(pool: web::Data<DbPool>, body: web::Json<CreateTodoRequest>) -> HttpResponse {
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let title = body.title.trim();
    if title.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "Title cannot be empty"}));
    }

    let id = Uuid::new_v4().to_string();
    let section_str = body.section.as_str();
    let importance = body.importance.unwrap_or_default();
    let due_date = body.due_date.as_deref();

    let result = conn.execute(
        "INSERT INTO todos (id, section, title, importance, due_date) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, section_str, title, importance.as_str(), due_date],
    );

    match result {
        Ok(_) => {
            let sql = format!("SELECT {} FROM todos WHERE id = ?1", SELECT_COLS);
            let todo = conn.query_row(&sql, params![id], row_to_todo).unwrap();
            HttpResponse::Created().json(todo)
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn update_todo(
    pool: web::Data<DbPool>,
    path: web::Path<String>,
    body: web::Json<UpdateTodoRequest>,
) -> HttpResponse {
    let id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let exists: bool = conn
        .query_row("SELECT COUNT(*) FROM todos WHERE id = ?1", params![id], |row| row.get::<_, i32>(0))
        .map(|c| c > 0)
        .unwrap_or(false);

    if !exists {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Todo not found"}));
    }

    if let Some(ref title) = body.title {
        let title = title.trim();
        if title.is_empty() {
            return HttpResponse::BadRequest().json(serde_json::json!({"error": "Title cannot be empty"}));
        }
        conn.execute("UPDATE todos SET title = ?1, updated_at = datetime('now') WHERE id = ?2", params![title, id]).unwrap();
    }

    if let Some(completed) = body.completed {
        conn.execute("UPDATE todos SET completed = ?1, updated_at = datetime('now') WHERE id = ?2", params![completed as i32, id]).unwrap();
    }

    if let Some(section) = body.section {
        conn.execute("UPDATE todos SET section = ?1, updated_at = datetime('now') WHERE id = ?2", params![section.as_str(), id]).unwrap();
    }

    if let Some(importance) = body.importance {
        conn.execute("UPDATE todos SET importance = ?1, updated_at = datetime('now') WHERE id = ?2", params![importance.as_str(), id]).unwrap();
    }

    if let Some(ref due_date_opt) = body.due_date {
        conn.execute("UPDATE todos SET due_date = ?1, updated_at = datetime('now') WHERE id = ?2", params![due_date_opt.as_deref(), id]).unwrap();
    }

    let sql = format!("SELECT {} FROM todos WHERE id = ?1", SELECT_COLS);
    let todo = conn.query_row(&sql, params![id], row_to_todo).unwrap();
    HttpResponse::Ok().json(todo)
}

pub async fn delete_todo(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let affected = conn.execute("DELETE FROM todos WHERE id = ?1", params![id]).unwrap_or(0);

    if affected == 0 {
        HttpResponse::NotFound().json(serde_json::json!({"error": "Todo not found"}))
    } else {
        HttpResponse::Ok().json(DeleteResponse { deleted: id })
    }
}

pub async fn toggle_todo(pool: web::Data<DbPool>, path: web::Path<String>) -> HttpResponse {
    let id = path.into_inner();
    let conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()})),
    };

    let affected = conn
        .execute("UPDATE todos SET completed = 1 - completed, updated_at = datetime('now') WHERE id = ?1", params![id])
        .unwrap_or(0);

    if affected == 0 {
        return HttpResponse::NotFound().json(serde_json::json!({"error": "Todo not found"}));
    }

    let sql = format!("SELECT {} FROM todos WHERE id = ?1", SELECT_COLS);
    let todo = conn.query_row(&sql, params![id], row_to_todo).unwrap();
    HttpResponse::Ok().json(todo)
}
