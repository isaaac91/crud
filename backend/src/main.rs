use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePoolOptions, FromRow, SqlitePool};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

// Modelos de datos
#[derive(Serialize, Deserialize, Debug, FromRow)]
struct Categoria {
    id: i64,
    nombre: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Tarea {
    id: i64,
    titulo: String,
    descripcion: String,
    categoria: Categoria,
    completada: bool,
}

// Estructura auxiliar para el mapeo SQLx
#[derive(FromRow)]
struct TareaQuery {
    id: i64,
    titulo: String,
    descripcion: String,
    completada: bool,
    categoria_id: i64,
    categoria_nombre: String,
}

impl From<TareaQuery> for Tarea {
    fn from(query: TareaQuery) -> Self {
        Tarea {
            id: query.id,
            titulo: query.titulo,
            descripcion: query.descripcion,
            completada: query.completada,
            categoria: Categoria {
                id: query.categoria_id,
                nombre: query.categoria_nombre,
            },
        }
    }
}

// Para creación de tareas
#[derive(Deserialize, Debug)]
struct NuevaTarea {
    titulo: String,
    descripcion: String,
    categoria_id: i64,
}

// Para actualización de tareas
#[derive(Deserialize)]
struct ActualizarTarea {
    titulo: Option<String>,
    descripcion: Option<String>,
    categoria_id: Option<i64>,
    completada: Option<bool>,
}

// Estado de la aplicación
struct AppState {
    db: SqlitePool,
}

// Respuesta de error personalizada
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> axum::response::Response {
        let body = Json(self);
        (StatusCode::BAD_REQUEST, body).into_response()
    }
}

#[tokio::main]
async fn main() {
    // Configuración de la base de datos
    let db_url = "sqlite:tareas.db";
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await
        .expect("Error al conectar con la base de datos");

    // Crear tablas si no existen
    sqlx::migrate!()
        .run(&pool)
        .await
        .expect("Error en migraciones");

    // Insertar categorías iniciales si no existen
    init_categorias(&pool).await;

    let state = Arc::new(AppState { db: pool });

    // Configuración CORS para permitir todas las conexiones del frontend
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([header::CONTENT_TYPE]);

    let app = Router::new()
        .route("/categorias", get(listar_categorias))
        .route("/tareas", get(listar_tareas).post(crear_tarea))
        .route("/tareas/:id", patch(actualizar_tarea).delete(borrar_tarea))
        .layer(cors)
        .with_state(state);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Servidor ejecutándose en http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

// Inicializar categorías por defecto
async fn init_categorias(pool: &SqlitePool) {
    let categorias = vec!["Compras", "Trabajo", "Estudio", "Personal", "Otros"];
    
    for nombre in categorias {
        sqlx::query(
            "INSERT OR IGNORE INTO categorias (nombre) VALUES (?)"
        )
        .bind(nombre)
        .execute(pool)
        .await
        .expect("Error al insertar categorías iniciales");
    }
}

// Controlador para listar categorías
async fn listar_categorias(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Categoria>>, ErrorResponse> {
    let categorias = sqlx::query_as::<_, Categoria>("SELECT id, nombre FROM categorias ORDER BY nombre")
        .fetch_all(&state.db)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Error al obtener categorías: {}", e),
        })?;
    
    Ok(Json(categorias))
}

// Controlador para listar tareas
async fn listar_tareas(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Tarea>>, ErrorResponse> {
    let tareas_query = sqlx::query_as::<_, TareaQuery>(
        r#"
        SELECT t.id, t.titulo, t.descripcion, t.completada,
               c.id AS categoria_id, c.nombre AS categoria_nombre
        FROM tareas t
        INNER JOIN categorias c ON t.categoria_id = c.id
        ORDER BY t.completada, t.id DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ErrorResponse {
        error: format!("Error al obtener tareas: {}", e),
    })?;

    let tareas = tareas_query.into_iter().map(Tarea::from).collect();
    
    Ok(Json(tareas))
}

// Controlador para crear tarea
async fn crear_tarea(
    State(state): State<Arc<AppState>>,
    Json(nueva_tarea): Json<NuevaTarea>,
    
) -> Result<Json<Tarea>, ErrorResponse> {
    // Validar datos de entrada
    println!("Recibiendo  nueva tarea: {:?}", nueva_tarea);

    if nueva_tarea.titulo.trim().is_empty() {
        return Err(ErrorResponse {
            error: "El título no puede estar vacío".to_string(),
        });
    }

    if nueva_tarea.descripcion.trim().is_empty() {
        return Err(ErrorResponse {
            error: "La descripción no puede estar vacía".to_string(),
        });
    }

    // Verificar que existe la categoría
    let categoria_existe = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM categorias WHERE id = ?"
    )
    .bind(nueva_tarea.categoria_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ErrorResponse {
        error: format!("Error al verificar categoría: {}", e),
    })?;

    if categoria_existe.is_none() {
        return Err(ErrorResponse {
            error: format!("La categoría con ID {} no existe", nueva_tarea.categoria_id),
        });
    }

    let id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO tareas (titulo, descripcion, categoria_id, completada)
        VALUES (?, ?, ?, ?)
        RETURNING id
        "#,
    )
    .bind(&nueva_tarea.titulo)
    .bind(&nueva_tarea.descripcion)
    .bind(nueva_tarea.categoria_id)
    .bind(false)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ErrorResponse {
        error: format!("Error al crear tarea: {}", e),
    })?;

    let tarea = obtener_tarea_completa(&state.db, id).await?;
    Ok(Json(tarea))
}

// Controlador para actualizar tarea
async fn actualizar_tarea(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Json(actualizacion): Json<ActualizarTarea>,
) -> Result<Json<Tarea>, ErrorResponse> {
    // Verificar que existe la tarea
    let tarea_existe = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM tareas WHERE id = ?"
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ErrorResponse {
        error: format!("Error al verificar tarea: {}", e),
    })?;

    if tarea_existe.is_none() {
        return Err(ErrorResponse {
            error: format!("La tarea con ID {} no existe", id),
        });
    }

    // Verificar categoría si se está actualizando
    if let Some(categoria_id) = actualizacion.categoria_id {
        let categoria_existe = sqlx::query_scalar::<_, i64>(
            "SELECT 1 FROM categorias WHERE id = ?"
        )
        .bind(categoria_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Error al verificar categoría: {}", e),
        })?;

        if categoria_existe.is_none() {
            return Err(ErrorResponse {
                error: format!("La categoría con ID {} no existe", categoria_id),
            });
        }
    }

    // Construir consulta dinámica
    let mut query = sqlx::QueryBuilder::new("UPDATE tareas SET ");
    let mut first = true;

    if let Some(titulo) = &actualizacion.titulo {
        if titulo.trim().is_empty() {
            return Err(ErrorResponse {
                error: "El título no puede estar vacío".to_string(),
            });
        }
        if !first {
            query.push(", ");
        }
        query.push("titulo = ").push_bind(titulo);
        first = false;
    }
    
    if let Some(descripcion) = &actualizacion.descripcion {
        if descripcion.trim().is_empty() {
            return Err(ErrorResponse {
                error: "La descripción no puede estar vacía".to_string(),
            });
        }
        if !first {
            query.push(", ");
        }
        query.push("descripcion = ").push_bind(descripcion);
        first = false;
    }
    
    if let Some(categoria_id) = actualizacion.categoria_id {
        if !first {
            query.push(", ");
        }
        query.push("categoria_id = ").push_bind(categoria_id);
        first = false;
    }
    
    if let Some(completada) = actualizacion.completada {
        if !first {
            query.push(", ");
        }
        query.push("completada = ").push_bind(completada);
    }
    
    query.push(" WHERE id = ").push_bind(id);
    
    query
        .build()
        .execute(&state.db)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Error al actualizar tarea: {}", e),
        })?;

    let tarea = obtener_tarea_completa(&state.db, id).await?;
    Ok(Json(tarea))
}

// Controlador para borrar tarea
async fn borrar_tarea(
    Path(id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<StatusCode, ErrorResponse> {
    let result = sqlx::query("DELETE FROM tareas WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| ErrorResponse {
            error: format!("Error al borrar tarea: {}", e),
        })?;
    
    if result.rows_affected() > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ErrorResponse {
            error: format!("La tarea con ID {} no existe", id),
        })
    }
}

// Función auxiliar para obtener tarea completa con categoría
async fn obtener_tarea_completa(
    db: &SqlitePool,
    id: i64,
) -> Result<Tarea, ErrorResponse> {
    let query = sqlx::query_as::<_, TareaQuery>(
        r#"
        SELECT t.id, t.titulo, t.descripcion, t.completada,
               c.id AS categoria_id, c.nombre AS categoria_nombre
        FROM tareas t
        INNER JOIN categorias c ON t.categoria_id = c.id
        WHERE t.id = ?
        "#,
    )
    .bind(id)
    .fetch_one(db)
    .await
    .map_err(|e| ErrorResponse {
        error: format!("Error al obtener tarea: {}", e),
    })?;

    Ok(Tarea::from(query))
}