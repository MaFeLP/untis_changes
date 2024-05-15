#[macro_use]
extern crate rocket;

use anyhow::anyhow;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE, COOKIE};
use reqwest::{Client, Error, Response};
use rocket::log::private::{debug, error, info};
use rocket::serde::json::serde_json::{self, json};
use rocket::serde::json::Json;
use rocket::serde::uuid::Uuid;
use rocket::serde::{Deserialize, Serialize};
use std::cmp::PartialEq;
use std::collections::HashMap;

enum RPCMethods {
    Authenticate,
    Logout,
}

async fn request(
    client: &Client,
    method: RPCMethods,
    params: serde_json::Value,
    jsession_id: Option<&str>,
) -> Result<(Uuid, Response), Error> {
    #[derive(Serialize)]
    #[serde(crate = "rocket::serde")]
    struct Body<'a> {
        id: Uuid,
        method: &'a str,
        params: serde_json::Value,
        jsonrpc: &'static str,
    }

    let uid = Uuid::new_v4();

    let body = Body {
        id: uid,
        method: match method {
            RPCMethods::Authenticate => "authenticate",
            RPCMethods::Logout => "logout",
        },
        jsonrpc: "2.0",
        params,
    };

    let mut request = client
        .post(format!(
            "https://{}/WebUntis/jsonrpc.do?school={}",
            std::env::var("UNTIS_HOST").expect("'UNTIS_HOST' not defined!"),
            std::env::var("UNTIS_SCHOOL").expect("'UNTIS_SCHOOL' not defined!")
        ))
        .json(&body);

    if let Some(id) = jsession_id {
        request = request.header(COOKIE, format!("JSESSIONID={}", id));
    }

    let response = request.send().await?;
    Ok((uid, response))
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct RPCResponse<T> {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Uuid,
    result: Option<T>,
}

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct UserInfo {
    #[serde(rename = "sessionId")]
    session_id: String,
    #[allow(dead_code)]
    #[serde(rename = "personType")]
    person_type: u64,
    #[serde(rename = "personId")]
    person_id: u64,
    #[allow(dead_code)]
    #[serde(rename = "klasseId")]
    klasse_id: u64,
}

async fn login(client: &Client, user: &str, password: &str) -> anyhow::Result<UserInfo> {
    debug!("Logging in to webuntis as {user}");
    let (uid, response) = request(
        client,
        RPCMethods::Authenticate,
        json!({
            "user": user,
            "password": password,
            "client": concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"))
        }),
        None,
    )
    .await?;

    let data: RPCResponse<UserInfo> = response.json().await?;
    assert_eq!(uid, data.id);
    debug!("Log in result: {data:?}");
    match data.result {
        Some(res) => Ok(res),
        None => Err(anyhow!(
            "Result Type is empty! Could not retrieve login information!"
        )),
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
enum ElementState {
    Regular,
    Absent,
    Substituted,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct OriginalRoom {
    id: u64,
    name: String,
    #[serde(rename = "longName")]
    long_name: String,
    displayname: String,
    alternatename: String,
    #[serde(rename = "canViewTimetable")]
    can_view_timetable: bool,
    #[serde(rename = "roomCapacity")]
    room_capacity: u64,
}

impl From<&OriginalRoom> for OriginalRoom {
    fn from(val: &OriginalRoom) -> Self {
        Self {
            id: val.id,
            name: String::from(&val.name),
            long_name: String::from(&val.long_name),
            displayname: String::from(&val.displayname),
            alternatename: String::from(&val.alternatename),
            can_view_timetable: val.can_view_timetable,
            room_capacity: val.room_capacity,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Room {
    id: u64,
    #[serde(rename = "orgId")]
    original_room_id: u64,
    original_room: Option<OriginalRoom>,
    missing: bool,
    state: ElementState,
    name: String,
    #[serde(rename = "longName")]
    long_name: String,
    displayname: String,
    alternatename: String,
    #[serde(rename = "canViewTimetable")]
    can_view_timetable: bool,
    #[serde(rename = "roomCapacity")]
    room_capacity: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct OriginalTeacher {
    id: u64,
    name: String,
    #[serde(rename = "canViewTimetable")]
    can_view_timetable: bool,
    #[serde(rename = "externKey")]
    extern_key: String,
    #[serde(rename = "roomCapacity")]
    room_capacity: u64,
}

impl From<&OriginalTeacher> for OriginalTeacher {
    fn from(val: &OriginalTeacher) -> Self {
        Self {
            id: val.id,
            name: val.name.to_string(),
            can_view_timetable: val.can_view_timetable,
            extern_key: val.extern_key.to_string(),
            room_capacity: val.room_capacity,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Teacher {
    id: u64,
    #[serde(rename = "orgId")]
    original_teacher_id: u64,
    original_teacher: Option<OriginalTeacher>,
    missing: bool,
    state: ElementState,
    name: String,
    #[serde(rename = "canViewTimetable")]
    can_view_timetable: bool,
    #[serde(rename = "externKey")]
    extern_key: String,
    #[serde(rename = "roomCapacity")]
    room_capacity: u64,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct OriginalSubject {
    id: u64,
    name: String,
    #[serde(rename = "longName")]
    long_name: String,
    #[serde(rename = "displayname")]
    display_name: String,
    #[serde(rename = "alternatename")]
    alternate_name: String,
    #[serde(rename = "backColor")]
    back_color: String,
    #[serde(rename = "canViewTimetable")]
    can_view_timetable: bool,
    #[serde(rename = "roomCapacity")]
    room_capacity: u64,
    #[serde(rename = "foreColor")]
    fore_color: Option<String>,
}

impl From<&OriginalSubject> for OriginalSubject {
    fn from(val: &OriginalSubject) -> Self {
        OriginalSubject {
            id: val.id,
            name: String::from(&val.name),
            long_name: String::from(&val.long_name),
            display_name: String::from(&val.display_name),
            alternate_name: String::from(&val.alternate_name),
            back_color: String::from(&val.back_color),
            can_view_timetable: val.can_view_timetable,
            room_capacity: val.room_capacity,
            fore_color: val.fore_color.as_ref().map(String::from),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Subject {
    id: u64,
    #[serde(rename = "orgId")]
    original_subject_id: u64,
    original_subject: Option<OriginalSubject>,
    missing: bool,
    state: ElementState,
    name: String,
    #[serde(rename = "longName")]
    long_name: String,
    #[serde(rename = "displayname")]
    display_name: String,
    #[serde(rename = "alternatename")]
    alternate_name: String,
    #[serde(rename = "backColor")]
    back_color: String,
    #[serde(rename = "canViewTimetable")]
    can_view_timetable: bool,
    #[serde(rename = "roomCapacity")]
    room_capacity: u64,
    #[serde(rename = "foreColor")]
    fore_color: Option<String>,
}

#[derive(Serialize, Debug, PartialEq)]
#[serde(crate = "rocket::serde")]
enum PeriodState {
    Standard,
    Substitution,
    Cancel,
}

#[derive(Serialize, Debug)]
#[serde(crate = "rocket::serde")]
struct Period {
    #[serde(rename = "lessonText")]
    lesson_text: String,
    #[serde(rename = "periodText")]
    text: String,
    #[serde(rename = "periodInfo")]
    info: String,
    #[serde(rename = "substText")]
    substitution_text: String,
    date: chrono::NaiveDate,
    #[serde(rename = "startTime")]
    start_time: chrono::NaiveTime,
    #[serde(rename = "endTime")]
    end_time: chrono::NaiveTime,
    state: PeriodState,
    teacher: Option<Teacher>,
    subject: Option<Subject>,
    room: Option<Room>,
}

impl Period {
    fn speakable_text(&self) -> String {
        match &self.subject {
            Some(subject) => match self.state {
                PeriodState::Cancel => {
                    return format!(
                        "{} fällt zwischen {} und {} Uhr aus!",
                        subject.long_name,
                        self.start_time.format("%H:%M"),
                        self.end_time.format("%H:%M"),
                    )
                }
                PeriodState::Standard => format!(
                    "Im Fach {} zwischen {} und {} Uhr gibt es keine Änderungen!",
                    subject.long_name,
                    self.start_time.format("%H:%M"),
                    self.end_time.format("%H:%M"),
                ),
                PeriodState::Substitution => {
                    let mut out = format!(
                        "Änderung bei {} zwischen {} und {} Uhr: ",
                        subject.long_name,
                        self.start_time.format("%H:%M"),
                        self.end_time.format("%H:%M"),
                    );
                    if let Some(teacher) = &self.teacher {
                        match teacher.state {
                            ElementState::Regular => {}
                            ElementState::Absent => match &teacher.original_teacher {
                                None => {}
                                Some(original_teacher) => out.push_str(&format!(
                                    "Unterricht ohne Lehrer (von '{}'); ",
                                    original_teacher.name,
                                )),
                            },
                            ElementState::Substituted => match &teacher.original_teacher {
                                None => {}
                                Some(original_teacher) => out.push_str(&format!(
                                    "Lehrerwechsel von '{}' zu '{}'; ",
                                    original_teacher.name, teacher.name,
                                )),
                            },
                        };
                    }
                    if let Some(room) = &self.room {
                        match room.state {
                            ElementState::Regular => {}
                            ElementState::Absent => match &room.original_room {
                                None => {}
                                Some(original_room) => out.push_str(&format!(
                                    "Unterricht ohne Raum (von '{}'); ",
                                    original_room.long_name,
                                )),
                            },
                            ElementState::Substituted => match &room.original_room {
                                None => {}
                                Some(original_room) => out.push_str(&format!(
                                    "Raumwechsel von '{}' zu '{}'; ",
                                    original_room.long_name, room.long_name,
                                )),
                            },
                        }
                    }
                    out.push_str(&self.substitution_text);
                    out
                }
            },
            None => String::new(),
        }
    }
}

async fn get_timetable(
    client: &Client,
    session_id: &str,
    person_id: u64,
) -> Result<serde_json::Value, Error> {
    let response = client.get(
        format!("https://ikarus.webuntis.com/WebUntis/api/public/timetable/weekly/data?elementType=5&elementId={}&date={}&formatId=1", person_id, chrono::Local::now().format("%Y-%m-%d")),
    ).header(COOKIE, format!("JSESSIONID={}", session_id))
        .send()
        .await?;
    let data: serde_json::Value = response.json().await?;
    Ok(data)
}

async fn logout(client: &Client, jsession_id: &str) -> Result<(), Error> {
    let (uid, response) = request(
        client,
        RPCMethods::Logout,
        serde_json::Value::Null,
        Some(jsession_id),
    )
    .await?;

    let data: RPCResponse<()> = response.json().await?;
    assert_eq!(uid, data.id);
    Ok(())
}

fn json_value_to_time(value: &serde_json::Value) -> anyhow::Result<chrono::NaiveTime> {
    let time = value
        .as_u64()
        .ok_or(anyhow!("requested time ({value}) is not of type 'u64'"))?
        .to_string();
    let (hours, minutes) = if time.len() == 4 {
        (&time[0..2], &time[2..4])
    } else if time.len() == 3 {
        (&time[0..1], &time[1..3])
    } else {
        return Err(anyhow!("Invalid length for time ({time})"));
    };
    chrono::NaiveTime::from_hms_opt(hours.parse().unwrap(), minutes.parse().unwrap(), 0)
        .ok_or(anyhow!("Invalid time 'start_time' {hours} {minutes}"))
}

fn parse_timetable(timetable: serde_json::Value, person_id: u64) -> anyhow::Result<Vec<Period>> {
    let mut rooms: HashMap<u64, OriginalRoom> = HashMap::new();
    let mut teachers: HashMap<u64, OriginalTeacher> = HashMap::new();
    let mut subjects: HashMap<u64, OriginalSubject> = HashMap::new();

    let data = timetable
        .get("data")
        .ok_or(anyhow!("'.data' field not present in timetable"))?
        .get("result")
        .ok_or(anyhow!("'.data.result' field not present in timetable"))?
        .get("data")
        .ok_or(anyhow!(
            "'.data.result.data' field not present in timetable"
        ))?;
    let elements = data
        .get("elements")
        .ok_or(anyhow!("elements field not present in timetable"))?
        .as_array()
        .ok_or(anyhow!("elements field not of type 'array'"))?;
    for element in elements {
        let element_type = element
            .get("type")
            .ok_or(anyhow!(
                "one of the elements does not have a type associated with it"
            ))?
            .as_u64()
            .ok_or(anyhow!("one of the elements' type is not of type 'u64'"))?;
        let element_id = element
            .get("id")
            .ok_or(anyhow!(
                "one of the elements does not have an id associated with it"
            ))?
            .as_u64()
            .ok_or(anyhow!("one of the elements' id is not of type 'u64'"))?;
        match element_type {
            2 => {
                let teacher: OriginalTeacher = serde_json::from_value(element.clone())?;
                teachers.insert(element_id, teacher);
            }
            3 => {
                let subject: OriginalSubject = serde_json::from_value(element.clone())?;
                subjects.insert(element_id, subject);
            }
            4 => {
                let room: OriginalRoom = serde_json::from_value(element.clone())?;
                rooms.insert(element_id, room);
            }
            _ => error!("Unknown Type '{element_type}' on element {element:?}"),
        };
    }

    let periods = data
        .get("elementPeriods")
        .ok_or(anyhow!("data does not contain elementPeriods!"))?
        .get(format!("{}", person_id).as_str())
        .ok_or(anyhow!("No timetable for logged in user found in data!"))?
        .as_array()
        .ok_or(anyhow!("Periods are not an array!"))?;

    let mut serialized_periods: Vec<Period> = vec![];

    for period in periods {
        let mut room: Option<Room> = None;
        let mut teacher: Option<Teacher> = None;
        let mut subject: Option<Subject> = None;

        let elements = period
            .get("elements")
            .ok_or(anyhow!("No elements specified for period!"))?
            .as_array()
            .ok_or(anyhow!("Elements of period are not an array!"))?;
        for element in elements {
            let type_ = element
                .get("type")
                .ok_or(anyhow!("Element has no type!"))?
                .as_u64()
                .ok_or(anyhow!("Type of element is not of type 'u64'!"))?;
            let id = element
                .get("id")
                .ok_or(anyhow!("Element has no id!"))?
                .as_u64()
                .ok_or(anyhow!("id of element is not of type 'u64'!"))?;
            let original_id = element
                .get("orgId")
                .ok_or(anyhow!("Element has no orgId!"))?
                .as_u64()
                .ok_or(anyhow!("orgId of element is not of type 'u64'!"))?;
            let state = match element
                .get("state")
                .ok_or(anyhow!("field 'state' missing on element"))?
                .as_str()
                .ok_or(anyhow!("field 'state' not of type string"))?
            {
                "ABSENT" => ElementState::Absent,
                "REGULAR" => ElementState::Regular,
                "SUBSTITUTED" => ElementState::Substituted,
                _ => return Err(anyhow!("Unknown type of 'state' on element {element}")),
            };
            match type_ {
                2 => {
                    let teacher_info = teachers
                        .get(&id)
                        .ok_or(anyhow!("Teacher with id {} has not been found!", id))?;
                    teacher = Some(Teacher {
                        id,
                        original_teacher_id: original_id,
                        original_teacher: teachers.get(&original_id).map(|t| t.into()),
                        state,
                        missing: element
                            .get("missing")
                            .ok_or(anyhow!("field 'missing' missing on element"))?
                            .as_bool()
                            .ok_or(anyhow!("field 'missing' not of type boolean"))?,
                        name: teacher_info.name.to_string(),
                        can_view_timetable: teacher_info.can_view_timetable,
                        extern_key: teacher_info.extern_key.to_string(),
                        room_capacity: teacher_info.room_capacity,
                    })
                }
                3 => {
                    let subject_info = subjects
                        .get(&id)
                        .ok_or(anyhow!("Subject with id {} has not been found!", id))?;
                    subject = Some(Subject {
                        id,
                        original_subject_id: original_id,
                        original_subject: subjects.get(&original_id).map(|t| t.into()),
                        missing: element
                            .get("missing")
                            .ok_or(anyhow!("field 'missing' missing on element"))?
                            .as_bool()
                            .ok_or(anyhow!("field 'missing' not of type boolean"))?,
                        state,
                        name: subject_info.name.to_string(),
                        long_name: subject_info.long_name.to_string(),
                        display_name: subject_info.display_name.to_string(),
                        alternate_name: subject_info.alternate_name.to_string(),
                        back_color: match element.get("backColor") {
                            None => subject_info.back_color.to_string(),
                            Some(val) => val
                                .as_str()
                                .ok_or(anyhow!("field 'backColor' not of type 'str'!"))?
                                .to_string(),
                        },
                        can_view_timetable: subject_info.can_view_timetable,
                        room_capacity: subject_info.room_capacity,
                        fore_color: match element.get("foreColor") {
                            None => None,
                            Some(val) => Some(
                                val.as_str()
                                    .ok_or(anyhow!("'foreColor' is not of type 'str'!"))?
                                    .to_string(),
                            ),
                        },
                    })
                }
                4 => {
                    let room_info = rooms
                        .get(&id)
                        .ok_or(anyhow!("Room with id {} has not been found!", id))?;
                    room = Some(Room {
                        id,
                        original_room_id: original_id,
                        original_room: rooms.get(&original_id).map(|t| t.into()),
                        missing: element
                            .get("missing")
                            .ok_or(anyhow!("field 'missing' missing on element"))?
                            .as_bool()
                            .ok_or(anyhow!("field 'missing' not of type boolean"))?,
                        state,
                        name: room_info.name.to_string(),
                        long_name: room_info.long_name.to_string(),
                        displayname: room_info.displayname.to_string(),
                        alternatename: room_info.alternatename.to_string(),
                        can_view_timetable: room_info.can_view_timetable,
                        room_capacity: room_info.room_capacity,
                    })
                }
                _ => return Err(anyhow!("Unknown type!")),
            };
        }

        let period_state = match period
            .get("cellState")
            .ok_or(anyhow!("field 'state' missing on period"))?
            .as_str()
            .ok_or(anyhow!("field 'state' is not of type 'str'"))?
        {
            "CANCEL" => PeriodState::Cancel,
            "STANDARD" => PeriodState::Standard,
            "SUBSTITUTION" => PeriodState::Substitution,
            _ => return Err(anyhow!("Unknown type of 'cellState' {period}")),
        };
        serialized_periods.push(Period {
            lesson_text: period
                .get("lessonText")
                .ok_or(anyhow!("field 'lessonText' missing on period"))?
                .as_str()
                .ok_or(anyhow!("field 'lessonText' is not of type 'str'"))?
                .to_string(),
            text: period
                .get("periodText")
                .ok_or(anyhow!("field 'periodText' missing on period"))?
                .as_str()
                .ok_or(anyhow!("field 'periodText' is not of type 'str'"))?
                .to_string(),
            info: period
                .get("periodInfo")
                .ok_or(anyhow!("field 'periodInfo' missing on period"))?
                .as_str()
                .ok_or(anyhow!("field 'periodInfo' is not of type 'str'"))?
                .to_string(),
            substitution_text: period
                .get("substText")
                .ok_or(anyhow!("field 'substText' missing on period"))?
                .as_str()
                .ok_or(anyhow!("field 'substText' is not of type 'str'"))?
                .to_string(),
            date: chrono::NaiveDate::parse_from_str(
                &period
                    .get("date")
                    .ok_or(anyhow!("field 'date' missing on period"))?
                    .as_u64()
                    .ok_or(anyhow!("field 'date' is not of type 'u64'"))?
                    .to_string(),
                "%Y%m%d",
            )?,
            start_time: json_value_to_time(
                period
                    .get("startTime")
                    .ok_or(anyhow!("field 'startTime' missing on period"))?,
            )?,
            end_time: json_value_to_time(
                period
                    .get("endTime")
                    .ok_or(anyhow!("field 'endTime' missing on period"))?,
            )?,
            state: period_state,
            teacher,
            subject,
            room,
        });
    }

    Ok(serialized_periods)
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[derive(Deserialize)]
#[serde(crate = "rocket::serde")]
struct UsernamePassword {
    username: String,
    password: String,
}

#[post("/speakable", data = "<user>")]
async fn speakable(user: Json<UsernamePassword>) -> String {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let client = Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            "/",
            env!("CARGO_PKG_VERSION")
        ))
        .default_headers(headers)
        .build()
        .unwrap();

    info!("Logging in as {}...", &user.username);
    let userinfo = login(&client, &user.username, &user.password)
        .await
        .unwrap();
    info!("Retrieving timetable...");
    let timetable = get_timetable(&client, &userinfo.session_id, userinfo.person_id)
        .await
        .unwrap();
    info!("Logging out...");
    logout(&client, &userinfo.session_id).await.unwrap();

    info!("Parsing timetable...");
    let mut timetable = parse_timetable(timetable, userinfo.person_id).unwrap();
    timetable.sort_by_key(|period| chrono::NaiveDateTime::new(period.date, period.start_time));
    timetable
        .into_iter()
        .filter(|period| period.state != PeriodState::Standard)
        .filter(|period| period.date == chrono::Local::now().date_naive())
        .map(|period| period.speakable_text())
        .collect::<Vec<String>>()
        .join("\n")
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, speakable])
}
