#[macro_use]
extern crate napi_derive;

mod object;

use crate::object::NapiTimelineObjectKeyframe;
use supertimeline::Cap;
use supertimeline::TimelineObjectInstance;
use std::collections::HashSet;
use supertimeline::ResolvedTimelineObject;
use std::collections::HashMap;
use supertimeline::resolve_timeline;
use supertimeline::Expression;
use napi::JsUnknown;
use supertimeline::TimelineEnable;
use napi::JsBoolean;
use napi::JsString;
use crate::object::NapiTimelineObject;
use supertimeline::ResolveOptions;
use std::convert::TryInto;
use napi::{JsObject, Result, JsNumber, CallContext};

fn parse_options(raw_options: JsObject) -> Result<ResolveOptions> {
  let time: i64 = raw_options.get_named_property::<JsNumber>("time")?.try_into()?;
  let limit_count = if raw_options.has_named_property("limitCount")? { 
    let l: u32 = raw_options.get_named_property::<JsNumber>("limitCount")?.try_into()?;
    Some(l as usize) 
  } else { None };
  let limit_time = if raw_options.has_named_property("limitTime")? { 
    let l: i64 = raw_options.get_named_property::<JsNumber>("limitTime")?.try_into()?;
    Some(l as u64) 
  } else { None };
  let options = ResolveOptions {
    time: time as u64,
    limit_count: limit_count,
    limit_time: limit_time,
  };
  Ok(options)
}

fn parse_expr(obj: JsUnknown) -> Result<Expression> {
  let t = obj.get_type()?;
  match t {
    napi::ValueType::String => {
      let js_str = obj.coerce_to_string()?;
      let s = js_str.into_utf8()?.into_owned()?;
      Ok(Expression::String(s))
    },
    napi::ValueType::Number => {
      let js_num = obj.coerce_to_number()?.try_into()?;
      Ok(Expression::Number(js_num))
    }
    _ => Err(napi::Error {
      status: napi::Status::InvalidArg,
      reason: format!("Expression of type {} not implemented yet", t)
    })
  }
}

fn parse_enable_inner(obj: JsObject) -> Result<TimelineEnable> {
  let mut result = TimelineEnable {
    enable_start: None,
    enable_end: None,
    enable_while: None,
    duration: None,
    repeating: None,
  };

  if obj.has_named_property("start")? {
    let raw_start = obj.get_named_property::<JsUnknown>("start")?;
    result.enable_start = Some(parse_expr(raw_start)?);
  }
  if obj.has_named_property("end")? {
    let raw_end = obj.get_named_property::<JsUnknown>("end")?;
    result.enable_end = Some(parse_expr(raw_end)?);
  }
  if obj.has_named_property("while")? {
    let raw_while = obj.get_named_property::<JsUnknown>("while")?;
    result.enable_while = Some(parse_expr(raw_while)?);
  }
  if obj.has_named_property("duration")? {
    let raw_duration = obj.get_named_property::<JsUnknown>("duration")?;
    result.duration = Some(parse_expr(raw_duration)?);
  }
  if obj.has_named_property("repeating")? {
    let raw_repeating = obj.get_named_property::<JsUnknown>("repeating")?;
    result.repeating = Some(parse_expr(raw_repeating)?);
  }

  Ok(result)
}

fn parse_enable(obj: JsObject) -> Result<Vec<TimelineEnable>> {
  let mut res = Vec::new();

  if obj.is_array()? {
    let len = obj.get_array_length()?;
    for i in 0..len {
      let en = obj.get_element::<JsObject>(i)?;
      res.push(parse_enable_inner(en)?);
    }
  } else {
    res.push(parse_enable_inner(obj)?);
  }

  Ok(res)
}

fn parse_string_array(raw: JsUnknown) -> Result<Option<Vec<String>>> {

  if raw.get_type()? == napi::ValueType::Undefined {
    Ok(None)
  } else {
    let mut res = Vec::new();

    let raw2 = raw.coerce_to_object()?;
    let len = raw2.get_array_length()?;
    for i in 0..len {
      let obj = raw2.get_element::<JsString>(i)?;
      res.push(obj.into_utf8()?.into_owned()?);
    }

    Ok(Some(res))
  }
}

fn parse_keyframes(raw: JsUnknown) -> Result<Option<Vec<NapiTimelineObjectKeyframe>>> {
  if raw.get_type()? == napi::ValueType::Undefined {
    Ok(None)
  } else {
    let mut res = Vec::new();

    let raw2 = raw.coerce_to_object()?;
    let len = raw2.get_array_length()?;
    for i in 0..len {
      let obj = raw2.get_element::<JsObject>(i)?;

      let id = obj.get_named_property::<JsString>("id")?.into_utf8()?.into_owned()?;
      let disabled = if obj.has_named_property("disabled")? {obj.get_named_property::<JsBoolean>("disabled")?.get_value()?} else {false};
      let content = obj.get_named_property::<JsObject>("content")?;
      let enable = parse_enable(obj.get_named_property::<JsObject>("enable")?)?;

      res.push(NapiTimelineObjectKeyframe{
        id,
        enable,
        classes: parse_string_array(obj.get_named_property::<JsUnknown>("classes")?)?,
        disabled,
        content,
      })
    }

    Ok(Some(res))
  }
}

fn parse_timeline(raw_tl: JsObject) -> Result<Vec<NapiTimelineObject>> {
  let mut res = Vec::new();

  let len = raw_tl.get_array_length()?;
  for i in 0..len {
    let obj = raw_tl.get_element::<JsObject>(i)?;
    let id = obj.get_named_property::<JsString>("id")?.into_utf8()?.into_owned()?;
    let layer = obj.get_named_property::<JsString>("layer")?.into_utf8()?.into_owned()?;
    let disabled = if obj.has_named_property("disabled")? {obj.get_named_property::<JsBoolean>("disabled")?.get_value()?} else {false};
    let content = obj.get_named_property::<JsObject>("content")?;
    let priority = if obj.has_named_property("priority")? {obj.get_named_property::<JsNumber>("priority")?.try_into()?} else {0};
    let enable = parse_enable(obj.get_named_property::<JsObject>("enable")?)?;

    let children = if obj.has_named_property("children")? {
      let raw = obj.get_named_property::<JsObject>("children")?;
      Some(parse_timeline(raw)?)
    } else {None};

    // TODO - finish parsing
    res.push(NapiTimelineObject {
      id,
      enable,
      layer,
      keyframes: parse_keyframes(obj.get_named_property::<JsUnknown>("keyframes")?)?,
      classes: parse_string_array(obj.get_named_property::<JsUnknown>("classes")?)?,
      disabled,
      content,
      children: children, 
      priority,
    })
  }
  
  Ok(res)
}

fn build_options(env: &napi::Env, options: &ResolveOptions) -> Result<JsObject> {
  let mut out_options = env.create_object()?;
  out_options.set_named_property("time", env.create_int64(options.time as i64)?)?;

  Ok(out_options)
}

fn build_string_array(env: &napi::Env, vals: &[String]) -> Result<JsObject> {
  let mut arr = env.create_array_with_length(vals.len())?;

  for (i, v) in vals.iter().enumerate() {
    arr.set_element(i as u32, env.create_string(v)?)?;
  }

  Ok(arr)
}
fn build_string_array2(env: &napi::Env, vals: &HashSet<String>) -> Result<JsObject> {
  let mut arr = env.create_array_with_length(vals.len())?;

  for (i, v) in vals.iter().enumerate() {
    arr.set_element(i as u32, env.create_string(v)?)?;
  }

  Ok(arr)
}


fn build_string_array_hashmap(env: &napi::Env, data: &HashMap<String, Vec<String>>) -> Result<JsObject> {
  let mut res = env.create_object()?;

  for (id, vals) in data {
    res.set_named_property(id, build_string_array(env, vals)?)?;
  }

  Ok(res)
}

fn build_caps(env: &napi::Env, caps: &[Cap]) -> Result<JsObject> {
  let mut res = env.create_array_with_length(caps.len())?;

  for (i, cap) in caps.iter().enumerate() {
    let mut res2 = env.create_object()?;
    
    res2.set_named_property("id", env.create_string(&cap.id)?)?;
    res2.set_named_property("start", env.create_int64(cap.start as i64)?)?;
    if let Some(t) = cap.end {
      res2.set_named_property("end", env.create_int64(t as i64)?)?;
    }

    res.set_element(i as u32, res2)?;
  }

  Ok(res)
}

fn build_timeline_object_instances(env: &napi::Env, instances: &[TimelineObjectInstance]) -> Result<JsObject> {
  let mut res = env.create_array_with_length(instances.len())?;

  for (i, instance) in instances.iter().enumerate() {
    let mut res2 = env.create_object()?;

    res2.set_named_property("id", env.create_string(&instance.id)?)?;
    res2.set_named_property("is_first", env.get_boolean(instance.is_first)?)?;
    res2.set_named_property("start", env.create_int64(instance.start as i64)?)?;
    if let Some(t) = instance.end {
      res2.set_named_property("end", env.create_int64(t as i64)?)?;
    }

    if let Some(t) = instance.original_start {
      res2.set_named_property("original_start", env.create_int64(t as i64)?)?;
    }
    if let Some(t) = instance.original_end {
      res2.set_named_property("original_end", env.create_int64(t as i64)?)?;
    }

    res2.set_named_property("references", build_string_array2(env, &instance.references)?)?;

    res2.set_named_property("caps", build_caps(env, &instance.caps)?)?;
    if let Some(t) = &instance.from_instance_id {
      res2.set_named_property("from_instance_id", env.create_string(&t)?)?;
    }

    res.set_element(i as u32, res2)?;
  }

  Ok(res)
}

fn build_expression(env: &napi::Env, expr: &Expression) -> Result<JsUnknown> {
  match expr {
    Expression::Null => Ok(env.get_null()?.into_unknown()),
    Expression::Number(v) => Ok(env.create_int64(*v)?.into_unknown()),
    Expression::String(v) => Ok(env.create_string(v)?.into_unknown()),
    // TODO - fill out
    _ => Err(napi::Error {
      status: napi::Status::InvalidArg,
      reason: format!("Cannot build expression from {}", expr)
    })
  }
}

fn build_timeline_enable(env: &napi::Env, obj: &[TimelineEnable]) -> Result<JsObject> {
  let mut res = env.create_array_with_length(obj.len())?;

  for (i, o) in obj.iter().enumerate() {
    let mut res2 = env.create_object()?;

    if let Some(e) = &o.enable_start {
      res2.set_named_property("start", build_expression(env, e)?)?;
    }
    if let Some(e) = &o.enable_end {
      res2.set_named_property("end", build_expression(env, e)?)?;
    }
    if let Some(e) = &o.enable_while {
      res2.set_named_property("while", build_expression(env, e)?)?;
    }
    if let Some(e) = &o.duration {
      res2.set_named_property("duration", build_expression(env, e)?)?;
    }
    if let Some(e) = &o.repeating {
      res2.set_named_property("repeating", build_expression(env, e)?)?;
    }

    res.set_element(i as u32, res2)?;
  }

  Ok(res)
}

fn build_resolved_timeline_objects(env: &napi::Env, objs: &HashMap<String, ResolvedTimelineObject>) -> Result<JsObject> {
  let mut res = env.create_object()?;

  for (id, obj) in objs {
    let mut res2 = env.create_object()?;

    let mut info = env.create_object()?;
    info.set_named_property("id", env.create_string(&obj.info.id)?)?;
    info.set_named_property("enable", build_timeline_enable(env, &obj.info.enable)?)?;
    info.set_named_property("priority", env.create_int64(obj.info.priority)?)?;
    info.set_named_property("disabled", env.get_boolean(obj.info.disabled)?)?;
    info.set_named_property("layer", env.create_string(&obj.info.layer)?)?;

    info.set_named_property("depth", env.create_int64(obj.info.depth as i64)?)?;
    if let Some(id) = &obj.info.parent_id {
      info.set_named_property("parent_id", env.create_string(&id)?)?;
    }
    info.set_named_property("is_keyframe", env.get_boolean(obj.info.is_keyframe)?)?;

    let mut resolved = env.create_object()?;
    resolved.set_named_property("is_self_referencing", env.get_boolean(obj.resolved.is_self_referencing)?)?;
    resolved.set_named_property("instances", build_timeline_object_instances(env, &obj.resolved.instances)?)?;
    resolved.set_named_property("direct_references", build_string_array2(env, &obj.resolved.direct_references)?)?;

    res2.set_named_property("resolved", resolved)?;
    res2.set_named_property("info", info)?;
    res.set_named_property(id, res2)?;
  }

  Ok(res)
}

#[js_function(2)] // ------> arguments length
fn js_resolve_timeline(ctx: CallContext) -> Result<JsObject> {
  let tl = parse_timeline(ctx.get::<JsObject>(0)?)?;
  let options = parse_options(ctx.get::<JsObject>(1)?)?;

  let res = resolve_timeline(&tl, options).or_else(|e| Err(napi::Error {
    status: napi::Status::GenericFailure,
    reason: format!("{:?}", e),
  }))?;

  let mut result = ctx.env.create_object()?;
  result.set_named_property("options", build_options(ctx.env, &res.options)?)?;
  result.set_named_property("objects", build_resolved_timeline_objects(ctx.env, &res.objects)?)?;
  result.set_named_property("classes", build_string_array_hashmap(ctx.env, &res.classes)?)?;
  result.set_named_property("layers", build_string_array_hashmap(ctx.env, &res.layers)?)?;

  Ok(result)
}

/// `exports` is `module.exports` object in NodeJS
#[module_exports]
fn init(mut exports: JsObject) -> Result<()> {
  exports.create_named_method("resolve_timeline", js_resolve_timeline)?;
  Ok(())
}