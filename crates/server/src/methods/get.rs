use crate::utils::get_rag_path;
use ragit_fs::{
    exists,
    extension,
    file_name,
    join,
    join3,
    read_bytes,
    read_dir,
    read_string,
};
use serde_json::{Map, Value};
use warp::Reply;
use warp::http::StatusCode;
use warp::reply::{json, with_header, with_status};

pub fn get_index(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let index_path = join(&rag_path, "index.json").unwrap();

    match read_string(&index_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "application/json",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_config(user: String, repo: String, config: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let config_path = join3(
        &rag_path,
        "configs",
        &format!("{config}.json"),
    ).unwrap();

    match read_string(&config_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "application/json",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_prompt(user: String, repo: String, prompt: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let prompt_path = join3(
        &rag_path,
        "prompts",
        &format!("{prompt}.json"),
    ).unwrap();

    match read_string(&prompt_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "text/plain",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_chunk_list(user: String, repo: String) -> Box<dyn Reply> {
    todo!()
}

pub fn get_chunk(user: String, repo: String, chunk_uid: String) -> Box<dyn Reply> {
    todo!()
}

pub fn get_image_list(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let image_path = join(
        &rag_path,
        "images",
    ).unwrap();

    match read_dir(&image_path) {
        Ok(images) => Box::new(json(
            &images.iter().filter_map(
                |image| match extension(image) {
                    Ok(Some(png)) if png == "png" => file_name(image).ok(),
                    _ => None,
                }
            ).collect::<Vec<String>>(),
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_image(user: String, repo: String, image: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let image_path = join3(
        &rag_path,
        "images",
        &format!("{image}.png"),
    ).unwrap();

    match read_bytes(&image_path) {
        Ok(bytes) => Box::new(with_header(
            bytes,
            "Content-Type",
            "image/png",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

pub fn get_image_desc(user: String, repo: String, image: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let image_path = join3(
        &rag_path,
        "images",
        &format!("{image}.json"),
    ).unwrap();

    match read_string(&image_path) {
        Ok(j) => Box::new(with_header(
            j,
            "Content-Type",
            "application/json",
        )),
        Err(_) => Box::new(with_status(
            String::new(),
            StatusCode::from_u16(404).unwrap(),
        )),
    }
}

// NOTE: a `.ragit/` may or may not have `meta.json`
pub fn get_meta(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);

    if !exists(&rag_path) {
        return Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap()));
    }

    let meta_path = join(&rag_path, "meta.json").unwrap();
    let meta_json = read_string(&meta_path).unwrap_or(String::from("{}"));
    Box::new(with_header(
        meta_json,
        "Content-Type",
        "application/json",
    ))
}

pub fn get_version(user: String, repo: String) -> Box<dyn Reply> {
    let rag_path = get_rag_path(&user, &repo);
    let index_path = join(&rag_path, "index.json").unwrap();
    let index_json = read_string(&index_path).unwrap_or(String::from("{}"));
    let index = serde_json::from_str::<Value>(&index_json).unwrap_or(Value::Object(Map::new()));

    match index {
        Value::Object(obj) => match obj.get("ragit_version") {
            Some(v) => match v.as_str() {
                Some(v) => Box::new(with_header(
                    v.to_string(),
                    "Content-Type",
                    "text/plain",
                )),
                None => Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap())),  // TODO: another error code
            },
            None => Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap())),
        },
        _ => Box::new(with_status(String::new(), StatusCode::from_u16(404).unwrap())),  // TODO: another error code
    }
}

pub fn get_server_version() -> Box<dyn Reply> {
    Box::new(with_header(
        ragit::VERSION,
        "Content-Type",
        "text/plain",
    ))
}
