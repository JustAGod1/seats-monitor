use std::collections::HashMap;
use std::hash::Hash;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use serde_json::Value;
use teloxide::Bot;
use teloxide::prelude::*;

#[tokio::main]
async fn main() {
    loop {
        println!("Starting");
        if let Err(e) = run().await {
            eprintln!("{}", e);
        }

        sleep(Duration::from_micros(500));
    }
}

trait ErrToString {
    type R;

    fn err_to_string(self) -> Result<Self::R, String>;
}

impl<A, E> ErrToString for Result<A, E> where E: ToString {
    type R = A;

    fn err_to_string(self) -> Result<Self::R, String> {
        self.map_err(|a| a.to_string())
    }
}

#[derive(Eq, PartialEq, Hash, Debug)]
struct CourseId {
    stream_id: u64,
    block_id: u64,
    course_id: u64,
}

fn get_course_seats() -> Result<HashMap<CourseId, u64>, String> {
    let output = Command::new("./info.sh").output().err_to_string()?.stdout;
    let output = String::from_utf8(output).err_to_string()?;

    let obj: Value = serde_json::from_str(output.as_str()).err_to_string()?;

    let data = obj.as_object().ok_or("Expected obj")?
        .get("seats").ok_or("Expected seats")?
        .as_array().ok_or("Expected data arr")?;

    let mut result = HashMap::<CourseId, u64>::new();

    for datum in data {
        let seat = datum.as_object().ok_or("expected seat obj")?;

        let stream_pk = seat.get("streamPk").ok_or("expected streamPk")?
            .as_u64().ok_or("expected streamPk u64")?;

        let block_pk = seat.get("blockPk").ok_or("expected blockPk")?
            .as_u64().ok_or("expected blockPk u64")?;

        let course_pk = seat.get("coursePk").ok_or("expected coursePk")?
            .as_u64().ok_or("expected coursePk u64")?;

        let free = seat.get("free").ok_or("expected free")?
            .as_u64().ok_or("expected free u64")?;


        result.insert(CourseId {
            stream_id: stream_pk,
            block_id: block_pk,
            course_id: course_pk,
        }, free);
    }

    Ok(result)
}

fn get_course_names() -> Result<HashMap<CourseId, String>, String> {
    let output = Command::new("./studentData.sh").output().err_to_string()?.stdout;
    let output = String::from_utf8(output).err_to_string()?;

    let obj: Value = serde_json::from_str(output.as_str()).err_to_string()?;

    let data = obj.as_object().ok_or("Expected obj")?
        .get("data").ok_or("Expected data")?
        .as_object().ok_or("Expected data obj")?;

    let courses = data.get("courses").ok_or("expected courses")?
        .as_array().ok_or("expected courses arr")?;

    let mut course_to_name = HashMap::<u64, String>::new();

    for course in courses {
        let course = course.as_object().ok_or("expected course obj")?;

        let pk = course.get("pk").ok_or("expected course pk")?
            .as_u64().ok_or("expected course pk u64")?;

        let title = course.get("title").ok_or("expected course title")?
            .as_str().ok_or("expected course title string")?;

        let id = course.get("id").ok_or("expected course id")?
            .as_str().ok_or("expected course id string")?;

        let name = format!("{}: {}", id, title);

        course_to_name.insert(pk, name);
    }

    let streams = data.get("streams").ok_or("expected streams")?
        .as_array().ok_or("expected streams arr")?;

    let mut result = HashMap::<CourseId, String>::new();

    for stream in streams {
        let stream = stream.as_object().ok_or("expected stream obj")?;
        let stream_pk = stream.get("pk").ok_or("expected stream pk")?
            .as_u64().ok_or("expected stream pk u64")?;

        let blocks = stream.get("blocks").ok_or("expected stream blocks")?
            .as_array().ok_or("expected stream blocks arr")?;


        for block in blocks {
            let block = block.as_object().ok_or("expected block obj")?;

            let block_pk = block.get("pk").ok_or("expected block pk")?
                .as_u64().ok_or("expected block pk u64")?;

            let courses = block.get("blockCourses").ok_or("expected block blockCourses")?
                .as_array().ok_or("expected block blockCourses arr")?;

            for course in courses {
                let course = course.as_object().ok_or("expected course obj")?;

                let course_pk = course.get("coursePk").ok_or("expected course coursePk")?
                    .as_u64().ok_or("expected course pk u64")?;

                let id = CourseId {
                    stream_id: stream_pk,
                    block_id: block_pk,
                    course_id: course_pk,
                };

                let name = course_to_name.get(&course_pk).ok_or("expected course name")?;

                result.insert(id, name.clone());
            }
        }
    }


    Ok(result)
}

async fn run() -> Result<(), String> {
    let bot = Bot::from_env().auto_send();

    let mut seats = get_course_seats()?;

    loop {
        let names = get_course_names()?;

        let new_seats = get_course_seats()?;

        for (id, free) in &new_seats {
            if !seats.contains_key(&id) || *seats.get(&id).unwrap() == 0u64 && *free != 0 {
                let name = names.get(&id).map(|a| a.as_str()).unwrap_or("unknown");
                bot.send_message(
                    ChatId(504208153),
                    format!("{}\n{:?} -> {}", name, seats.get(&id), free),
                ).await.err_to_string()?;
            }
        }

        seats = new_seats;
        sleep(Duration::from_secs(15))
    }
}
