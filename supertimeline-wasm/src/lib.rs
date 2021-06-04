mod utils;

use supertimeline::ResolveOptions;
use supertimeline_json::object::JsonTimelineObject;
use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern {
    // fn alert(s: &str);
}

#[wasm_bindgen]
pub fn resolve_timeline(raw_tl: &JsValue, raw_options: &JsValue) -> JsValue {
    // utils::set_panic_hook();


    let tl: Vec<JsonTimelineObject> = raw_tl.into_serde().unwrap();
    let options: ResolveOptions = raw_options.into_serde().unwrap();

    let result = supertimeline::resolve_timeline(&tl, options).unwrap();
    
    JsValue::from_serde(&result).unwrap()
}

// #[wasm_bindgen]
// pub fn resolve_timeline2(raw_tl: &str, raw_options: &JsValue) -> String {
//     utils::set_panic_hook();

    
//     let tl: Vec<JsonTimelineObject> = serde_json::from_str(raw_tl).unwrap();
        
//     let options: ResolveOptions = raw_options.into_serde().unwrap();

//     let result = supertimeline::resolve_timeline(&tl, options).unwrap();
    
//     serde_json::to_string(&result).unwrap()
// }