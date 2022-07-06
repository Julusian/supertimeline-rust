use std::collections::HashMap;
use actix_web::{get, web, App, HttpServer, Responder, HttpResponse};
use supertimeline::{NextEvent, resolve_all_states, resolve_timeline, ResolveOptions, Time, TimelineLayerState, TimelineObjectInstance};
use supertimeline_json::object::JsonTimelineObject;
use serde::{Deserialize, Serialize};
use actix_web::middleware::Logger;
use env_logger::Env;

#[get("/hello/{name}")]
async fn greet(name: web::Path<String>) -> impl Responder {
    format!("Hello {name}!")
}

#[derive(Debug, Serialize, Deserialize)]
struct MyObj {
    objects: Vec<JsonTimelineObject>,
    options: ResolveOptions
}

#[derive(Debug, Serialize, Deserialize)]
struct TimelineLayerState2{
    pub object_id: String,
    pub raw_object: Option<JsonTimelineObject>,
    pub instance_id: Option<String>, // TODO - these both being Option<> is horrible
    pub instance: Option<TimelineObjectInstance>, // TODO - this is a bit heavy now?
    pub keyframes: Vec<TimelineLayerState2Keyframe>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimelineLayerState2Keyframe {
    // Based on ResolvedTimelineObjectInstanceKeyframe
    pub keyframe_id: String,
    pub keyframe_end_time: Option<Time>,
}

fn transform_state(timeline: &HashMap<String,JsonTimelineObject>, st: Option<TimelineLayerState>)-> Option<TimelineLayerState2> {
    if let Some(st) = st {
        let instance = {
            if let Some(inst) = st.instance {
                let a = inst.try_lock().unwrap(); // TODO - eww
                Some(a.clone()) // Clone to avoid holding a lock
            } else {
                None
            }
        };

        let raw_object = timeline.get(&st.object_id).cloned();

        let mut res = TimelineLayerState2{
            object_id: st.object_id,
            raw_object,
            instance_id: st.instance_id,
            instance,
            keyframes: vec!(),
        };

        for keyframe in st.keyframes {
            res.keyframes.push(TimelineLayerState2Keyframe{
                keyframe_id: keyframe.info.id.clone(),
                keyframe_end_time: keyframe.keyframe_end_time,
            });
        }

        Some(res)
    } else {
        None
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Result {
    pub state: HashMap<String, HashMap<Time, Option<TimelineLayerState2>>>,
    pub next_events: Vec<NextEvent>,

    // TODO - some of these below are excessive and need clarifying what they now are
    /** Map of all objects on timeline */
    //pub objects: HashMap<String, ResolvedStatesForObject>,
    // /** Map of all classes on timeline, maps className to object ids */
    // pub classes: HashMap<String, Vec<String>>,
    /** Map of the object ids, per layer */
    pub layers: HashMap<String, Vec<String>>,
}

fn dothing(timeline: &mut HashMap<String,JsonTimelineObject>, obj: &JsonTimelineObject) {
    timeline.insert(obj.id.clone(), obj.clone());

    if let Some(children) = &obj.children {
        for child in children {
            dothing(timeline,child);
        }
    }
}


/// This handler uses json extractor
async fn index(item: web::Json<MyObj>) -> HttpResponse {
    let resolved = resolve_timeline(&item.objects, item.options.clone()).unwrap();

    let states = resolve_all_states(&resolved, None).unwrap();

    let mut timelineObjMap = HashMap::new();

    for obj in item.objects.iter() {
        dothing(&mut timelineObjMap,obj);
    
    }

    let mut result = Result {
        state: HashMap::new(),
        next_events : states.next_events,
        layers :states.layers,
    };

    for (id, val) in states.state {
        let mut res = HashMap::new();

        for (time,st) in val {
            res.insert(time, transform_state(&timelineObjMap,st));
        }

        result.state.insert(id, res);
    }

    HttpResponse::Ok().json(result) // <- send response
}

#[actix_web::main] // or #[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));


    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
            .service(greet)
            .service(web::resource("/").route(web::post().to(index)))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}