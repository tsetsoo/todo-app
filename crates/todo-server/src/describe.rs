use actix_web::HttpResponse;
use serde_json::json;

#[allow(clippy::too_many_lines)]
pub async fn describe() -> HttpResponse {
    let description = json!({
        "app": "TODO List",
        "version": "2.0.0",
        "description": "Personal TODO list with 4 sections: Sp (Spiritual), I (Intellectual), Si (Social), P (Physical). Each todo has a priority level and optional due date.",
        "base_url": "/api",
        "quick_start": {
            "create_a_todo": "curl -X POST /api/todos -H 'Content-Type: application/json' -d '{\"section\":\"P\",\"title\":\"Go for a run\",\"importance\":\"high\",\"due_date\":\"2026-02-21\"}'",
            "list_all_todos": "curl /api/todos",
            "list_by_section": "curl /api/todos?section=P",
            "list_by_importance": "curl /api/todos?sort=importance",
            "toggle_complete": "curl -X POST /api/todos/{id}/toggle",
            "delete_todo": "curl -X DELETE /api/todos/{id}"
        },
        "sections": {
            "Sp": "Spiritual - tasks related to spiritual growth and mindfulness",
            "I": "Intellectual - tasks related to learning, reading, and mental development",
            "Si": "Social - tasks related to relationships, community, and social activities",
            "P": "Physical - tasks related to health, fitness, and physical well-being"
        },
        "importance_levels": ["low", "medium", "high", "critical"],
        "endpoints": [
            {
                "method": "GET",
                "path": "/api/describe",
                "description": "Returns this self-describing API documentation"
            },
            {
                "method": "GET",
                "path": "/api/sections",
                "description": "List all sections with their todo counts",
                "example_response": [
                    {"section": "Sp", "total": 3, "completed": 1},
                    {"section": "I", "total": 5, "completed": 2},
                    {"section": "Si", "total": 2, "completed": 0},
                    {"section": "P", "total": 4, "completed": 3}
                ]
            },
            {
                "method": "GET",
                "path": "/api/todos",
                "description": "List all todos, optionally filtered by section and/or sorted",
                "parameters": [
                    {
                        "name": "section",
                        "in": "query",
                        "required": false,
                        "type": "string",
                        "enum": ["Sp", "I", "Si", "P"],
                        "description": "Filter todos by section"
                    },
                    {
                        "name": "sort",
                        "in": "query",
                        "required": false,
                        "type": "string",
                        "enum": ["importance", "due_date"],
                        "description": "Sort order. Default is newest first. 'importance' sorts critical first. 'due_date' sorts earliest due first."
                    }
                ],
                "example_request": "GET /api/todos?section=P&sort=importance",
                "example_response": [
                    {
                        "id": "550e8400-e29b-41d4-a716-446655440000",
                        "section": "P",
                        "title": "Go for a run",
                        "completed": false,
                        "importance": "high",
                        "due_date": "2026-02-21",
                        "created_at": "2026-01-15 10:30:00",
                        "updated_at": "2026-01-15 10:30:00"
                    }
                ]
            },
            {
                "method": "GET",
                "path": "/api/todos/{id}",
                "description": "Get a single todo by ID",
                "parameters": [
                    {"name": "id", "in": "path", "required": true, "type": "string (UUID)"}
                ]
            },
            {
                "method": "POST",
                "path": "/api/todos",
                "description": "Create a new todo",
                "request_body": {
                    "content_type": "application/json",
                    "fields": {
                        "section": {"type": "string", "required": true, "enum": ["Sp", "I", "Si", "P"]},
                        "title": {"type": "string", "required": true, "description": "The todo title (non-empty)"},
                        "importance": {"type": "string", "required": false, "enum": ["low", "medium", "high", "critical"], "default": "medium"},
                        "due_date": {"type": "string", "required": false, "format": "YYYY-MM-DD", "description": "Optional due date"}
                    }
                },
                "example_request": {"section": "P", "title": "Go for a run", "importance": "high", "due_date": "2026-02-21"},
                "example_response": {
                    "id": "550e8400-e29b-41d4-a716-446655440000",
                    "section": "P",
                    "title": "Go for a run",
                    "completed": false,
                    "importance": "high",
                    "due_date": "2026-02-21",
                    "created_at": "2026-01-15 10:30:00",
                    "updated_at": "2026-01-15 10:30:00"
                },
                "response_status": 201
            },
            {
                "method": "PATCH",
                "path": "/api/todos/{id}",
                "description": "Partially update a todo. Only include fields you want to change.",
                "parameters": [
                    {"name": "id", "in": "path", "required": true, "type": "string (UUID)"}
                ],
                "request_body": {
                    "content_type": "application/json",
                    "fields": {
                        "title": {"type": "string", "required": false},
                        "completed": {"type": "boolean", "required": false},
                        "section": {"type": "string", "required": false, "enum": ["Sp", "I", "Si", "P"]},
                        "importance": {"type": "string", "required": false, "enum": ["low", "medium", "high", "critical"]},
                        "due_date": {"type": "string or null", "required": false, "format": "YYYY-MM-DD", "description": "Set to null to clear the due date"}
                    }
                },
                "example_request": {"title": "Go for a long run", "importance": "critical"}
            },
            {
                "method": "DELETE",
                "path": "/api/todos/{id}",
                "description": "Delete a todo",
                "parameters": [
                    {"name": "id", "in": "path", "required": true, "type": "string (UUID)"}
                ],
                "example_response": {"deleted": "550e8400-e29b-41d4-a716-446655440000"}
            },
            {
                "method": "POST",
                "path": "/api/todos/{id}/toggle",
                "description": "Toggle the completed status of a todo",
                "parameters": [
                    {"name": "id", "in": "path", "required": true, "type": "string (UUID)"}
                ]
            }
        ],
        "data_model": {
            "Todo": {
                "id": "string (UUID v4) - unique identifier",
                "section": "string enum: Sp | I | Si | P",
                "title": "string - the todo title",
                "completed": "boolean - whether the todo is done",
                "importance": "string enum: low | medium | high | critical",
                "due_date": "string (YYYY-MM-DD) or null - optional due date",
                "created_at": "string (datetime) - when the todo was created",
                "updated_at": "string (datetime) - when the todo was last modified"
            }
        },
        "usage_notes": [
            "All request/response bodies use JSON (Content-Type: application/json)",
            "Section values are case-sensitive: Sp, I, Si, P",
            "Importance values are lowercase: low, medium, high, critical",
            "If importance is omitted on create, it defaults to 'medium'",
            "due_date format is YYYY-MM-DD",
            "The toggle endpoint is a convenience for flipping completed status",
            "Todos are returned in reverse chronological order by default",
            "The PATCH endpoint only updates fields that are present in the request body",
            "Empty or whitespace-only titles are rejected with 400"
        ]
    });

    HttpResponse::Ok().json(description)
}
