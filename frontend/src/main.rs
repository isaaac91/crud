use std::fmt;
use yew::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use gloo::net::http::Request;
use gloo::console;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Categoria {
    id: i64,
    nombre: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Tarea {
    id: i64,
    titulo: String,
    descripcion: String,
    categoria: Categoria,
    completada: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct NuevaTarea {
    titulo: String,
    descripcion: String,
    categoria_id: i64,
}

#[function_component(App)]
fn app() -> Html {
    let tareas = use_state(Vec::<Tarea>::new);
    let categorias = use_state(Vec::<Categoria>::new);
    let titulo = use_state(String::new);
    let descripcion = use_state(String::new);
    let categoria_id = use_state(|| 1i64);
    let error_msg = use_state(String::new);

    // Cargar datos iniciales
    {
        let tareas = tareas.clone();
        let categorias = categorias.clone();
        let categoria_id = categoria_id.clone();

        use_effect_with_deps(
            move |_| {
                let tareas = tareas.clone();
                let categorias = categorias.clone();
                let categoria_id = categoria_id.clone();

                spawn_local(async move {
                    // Cargar categorías
                    match Request::get("http://localhost:3000/categorias").send().await {
                        Ok(response) => {
                            if response.ok() {
                                match response.json::<Vec<Categoria>>().await {
                                    Ok(fetched_cats) => {
                                        categorias.set(fetched_cats.clone());
                                        if !fetched_cats.is_empty() {
                                            categoria_id.set(fetched_cats[0].id);
                                        }
                                        if let Ok(json_str) = serde_json::to_string(&fetched_cats) {
                                            console::log!("Categorías que llegaron del backend:", &json_str);
                                        } else {
                                            console::log!("Error al convertir categorías a JSON");
                                        }

                                        console::log!("Categoría seleccionada:", fetched_cats[0].id);

                                    }
                                    Err(e) => console::error!(format!("Error parsing categorías: {:?}", e)),
                                }
                            } else {
                                console::error!(format!("Error al cargar categorías: {}", response.status()));
                            }
                        }
                        Err(e) => console::error!(format!("Error de red al cargar categorías: {:?}", e)),
                    }

                    // Cargar tareas
                    match Request::get("http://localhost:3000/tareas").send().await {
                        Ok(response) => {
                            if response.ok() {
                                match response.json::<Vec<Tarea>>().await {
                                    Ok(fetched_tasks) => tareas.set(fetched_tasks),
                                    Err(e) => console::error!(format!("Error parsing tareas: {:?}", e)),
                                }
                            } else {
                                console::error!(format!("Error al cargar tareas: {}", response.status()));
                            }
                        }
                        Err(e) => console::error!(format!("Error de red al cargar tareas: {:?}", e)),
                    }
                });

                || ()
            },
            (),
        );
    }

    // Handler para cambios en el título
    let on_titulo_change = {
        let titulo = titulo.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<HtmlInputElement>().unwrap();
            titulo.set(input.value());
        })
    };

    // Handler para cambios en la descripción
    let on_descripcion_change = {
        let descripcion = descripcion.clone();
        Callback::from(move |e: Event| {
            let input = e.target_dyn_into::<HtmlInputElement>().unwrap();
            descripcion.set(input.value());
        }) 
    };

    // Handler para cambios en la categoría (corregido)
    let on_categoria_change = {
        let categoria_id_clone = categoria_id.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target_dyn_into::<HtmlSelectElement>() {
                if let Ok(value) = select.value().parse::<i64>() {
                    categoria_id_clone.set(value);
                }
            }
        })
    };

    // Función para agregar tarea
    let on_agregar = {
    let tareas = tareas.clone();
    let titulo = titulo.clone();
    let descripcion = descripcion.clone();
    let categoria_id = categoria_id.clone();
    let error_msg = error_msg.clone();

    Callback::from(move |_| {
    // Clonamos las variables que necesitamos mover al async block
    let tareas_clone = tareas.clone();
    let titulo_clone = titulo.clone();
    let descripcion_clone = descripcion.clone();
    let error_msg_clone = error_msg.clone();
    let categoria_id_clone = categoria_id.clone();

    // Validación (igual que antes)
    if titulo_clone.trim().is_empty() {
        error_msg_clone.set("El título no puede estar vacío".to_string());
        return;
    }

    if descripcion_clone.trim().is_empty() {
        error_msg_clone.set("La descripción no puede estar vacía".to_string());
        return;
    }

    // Creamos la nueva tarea
    let nueva_tarea = NuevaTarea {
        titulo: titulo_clone.to_string(),
        descripcion: descripcion_clone.to_string(),
        categoria_id: *categoria_id_clone,
    };

    spawn_local(async move {
        match Request::post("http://localhost:3000/tareas")
            .header("Content-Type", "application/json")
            .json(&nueva_tarea)
            .unwrap()
            .send()
            .await 
        {
            Ok(response) => {
                if response.ok() {
                    match response.json::<Tarea>().await {
                        Ok(tarea_creada) => {
                            // Usamos el clone de tareas
                            let mut nuevas_tareas = (*tareas_clone).clone();
                            nuevas_tareas.push(tarea_creada);
                            tareas_clone.set(nuevas_tareas);
                            
                            // Limpiamos usando los clones
                            titulo_clone.set(String::new());
                            descripcion_clone.set(String::new());
                            error_msg_clone.set(String::new());
                        }
                        Err(e) => {
                            error_msg_clone.set("Error al procesar la respuesta".to_string());
                            console::error!(format!("Error parsing response: {:?}", e));
                        }
                    }
                } else {
                    error_msg_clone.set(format!("Error del servidor: {}", response.status()));
                    console::error!("Error en la respuesta del servidor");
                }
            }
            Err(e) => {
                error_msg_clone.set("Error de conexión".to_string());
                console::error!(format!("Error en la solicitud: {:?}", e));
            }
        }
    });
})
};

    // Función para borrar tarea (corregido)
    let on_borrar = {
        let tareas = tareas.clone();
        Callback::from(move |id: i64| {
            let tareas = tareas.clone();
            spawn_local(async move {
                match Request::delete(&format!("http://localhost:3000/tareas/{}", id))
                    .send()
                    .await 
                {
                    Ok(response) => {
                        if response.ok() {
                            let mut nuevas_tareas = (*tareas).clone();
                            nuevas_tareas.retain(|t| t.id != id);
                            tareas.set(nuevas_tareas);
                        } else {
                            console::error!(format!("Error al borrar tarea: {}", response.status()));
                        }
                    }
                    Err(e) => console::error!(format!("Error de red al borrar tarea: {:?}", e)),
                }
            });
        })
    };

    // Función para alternar estado de completada (corregido)
    let on_toggle_completada = {
        let tareas = tareas.clone();
        Callback::from(move |(id, completada): (i64, bool)| {
            let tareas_clone = tareas.clone();
            spawn_local(async move {
                match Request::patch(&format!("http://localhost:3000/tareas/{}", id))
                    .header("Content-Type", "application/json")
                    .body(serde_json::json!({ "completada": !completada }).to_string())
                {
                    Ok(request) => {
                        match request.send().await {
                            Ok(response) => {
                                if response.ok() {
                                    match response.json::<Tarea>().await {
                                        Ok(tarea_actualizada) => {
                                            let mut nuevas_tareas = (*tareas_clone).clone();
                                            if let Some(index) = nuevas_tareas.iter().position(|t| t.id == id) {
                                                nuevas_tareas[index] = tarea_actualizada;
                                                tareas_clone.set(nuevas_tareas);
                                            }
                                        }
                                        Err(e) => console::error!(format!("Error parsing response: {:?}", e)),
                                    }
                                } else {
                                    console::error!(format!("Error al actualizar tarea: {}", response.status()));
                                }
                            }
                            Err(e) => console::error!(format!("Error de red al actualizar tarea: {:?}", e)),
                        }
                    }
                    Err(e) => console::error!(format!("Error al crear la solicitud: {:?}", e)),
                }
            });
        })
    };

    // Renderizar categorías como opciones
    let categorias_options = (*categorias).iter().map(|cat| {
        html! {
            <option value={cat.id.to_string()}>{&cat.nombre}</option>
        }
    }).collect::<Html>();

    // Renderizar lista de tareas (corregido)
    let lista_tareas = (*tareas).iter().map(|tarea| {
        let id = tarea.id;
        let completada = tarea.completada;
        let on_toggle_clone = on_toggle_completada.clone();
        let on_borrar_clone = on_borrar.clone();
        
        html! {
            <div class="border p-4 mb-2 rounded shadow" key={id}>
                <div class="flex justify-between items-center">
                    <div>
                        <h3 class={classes!("text-lg", "font-bold", completada.then(|| "line-through"))}>
                            {&tarea.titulo}
                        </h3>
                        <p class={classes!(completada.then(|| "line-through"))}>{&tarea.descripcion}</p>
                        <span class="inline-block bg-gray-200 rounded-full px-3 py-1 text-sm font-semibold text-gray-700">
                            {&tarea.categoria.nombre}
                        </span>
                    </div>
                    <div class="flex space-x-2">
                        <button 
                            onclick={move |_| on_toggle_clone.emit((id, completada))}
                            class={classes!(
                                "px-3", "py-1", "rounded", 
                                if completada { "bg-yellow-500 hover:bg-yellow-600" } 
                                else { "bg-green-500 hover:bg-green-600" },
                                "text-white"
                            )}
                        >
                            {if completada { "Reactivar" } else { "Completar" }}
                        </button>
                        <button 
                            onclick={move |_| on_borrar_clone.emit(id)}
                            class="px-3 py-1 bg-red-500 hover:bg-red-600 text-white rounded"
                        >
                            {"Borrar"}
                        </button>
                    </div>
                </div>
            </div>
        }
    }).collect::<Html>();

    html! {
        <div class="container mx-auto p-4 max-w-2xl">
            <h1 class="text-2xl font-bold mb-4">{"Planificador de Tareas"}</h1>
            
            // Formulario para agregar tareas
            <div class="bg-white p-4 rounded shadow mb-6">
                <h2 class="text-xl font-semibold mb-3">{"Agregar Nueva Tarea"}</h2>
                
                if !error_msg.is_empty() {
                    <div class="mb-3 p-2 bg-red-100 text-red-700 rounded">
                        {&*error_msg}
                    </div>
                }
                
                <div class="mb-3">
                    <label class="block text-gray-700 mb-1">{"Título"}</label>
                    <input 
                        type="text" 
                        value={(*titulo).clone()} 
                        onchange={on_titulo_change}
                        class="w-full p-2 border rounded"
                    />
                </div>
                
                <div class="mb-3">
                    <label class="block text-gray-700 mb-1">{"Descripción"}</label>
                    <input 
                        type="text" 
                        value={(*descripcion).clone()} 
                        onchange={on_descripcion_change}
                        class="w-full p-2 border rounded"
                    />
                </div>
                
                <div class="mb-3">
                    <label class="block text-gray-700 mb-1">{"Categoría"}</label>
                    <select 
                        value={categoria_id.to_string()} 
                        onchange={on_categoria_change}
                        class="w-full p-2 border rounded"
                    >
                        {(*categorias).iter().map(|cat| {
                            html! {
                                <option value={cat.id.to_string()} selected={*categoria_id == cat.id}>
                                    {&cat.nombre}
                                </option>
                            }
                        }).collect::<Html>()}
                    </select>
                </div>
                
                <button 
                    onclick={on_agregar}
                    class="w-full bg-blue-500 hover:bg-blue-600 text-white py-2 px-4 rounded"
                >
                    {"Agregar Tarea"}
                </button>
            </div>
            
            // Lista de tareas
            <div>
                <h2 class="text-xl font-semibold mb-3">{"Tareas"}</h2>
                if (*tareas).is_empty() {
                    <p class="text-gray-500">{"No hay tareas aún. ¡Agrega una!"}</p>
                } else {
                    <div>{lista_tareas}</div>
                }
            </div>
        </div>
    }
}


fn main() {
    yew::Renderer::<App>::new().render();
    
}