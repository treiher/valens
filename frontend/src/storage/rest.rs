use async_trait::async_trait;
use chrono::NaiveDate;
use gloo_net::http::Request;
use serde_json::{json, Map};

use super::{
    BodyFat, BodyWeight, Exercise, ExerciseMuscle, Period, Routine, RoutinePart, TrainingSession,
    TrainingSessionElement, User,
};

pub struct Storage;

#[async_trait(?Send)]
impl super::Storage for Storage {
    async fn request_session(&self, user_id: u32) -> Result<User, String> {
        fetch(
            Request::post("api/session")
                .json(&json!({ "id": user_id }))
                .expect("serialization failed"),
        )
        .await
    }

    async fn initialize_session(&self) -> Result<User, String> {
        fetch(Request::get("api/session").build().unwrap()).await
    }

    async fn delete_session(&self) -> Result<(), String> {
        fetch_no_content(Request::delete("api/session").build().unwrap(), ()).await
    }

    async fn read_version(&self) -> Result<String, String> {
        fetch(Request::get("api/version").build().unwrap()).await
    }

    async fn read_users(&self) -> Result<Vec<User>, String> {
        fetch(Request::get("api/users").build().unwrap()).await
    }
    async fn create_user(&self, name: String, sex: u8) -> Result<User, String> {
        fetch(
            Request::post("api/users")
                .json(&json!({
                    "name": name,
                    "sex": sex,
                }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn replace_user(&self, user: User) -> Result<User, String> {
        fetch(
            Request::put(&format!("api/users/{}", user.id))
                .json(&json!({
                    "name": user.name,
                    "sex": user.sex,
                }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_user(&self, id: u32) -> Result<u32, String> {
        fetch_no_content(
            Request::delete(&format!("api/users/{id}")).build().unwrap(),
            id,
        )
        .await
    }

    async fn read_body_weight(&self) -> Result<Vec<BodyWeight>, String> {
        fetch(Request::get("api/body_weight").build().unwrap()).await
    }
    async fn create_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, String> {
        fetch(
            Request::post("api/body_weight")
                .json(&body_weight)
                .expect("serialization failed"),
        )
        .await
    }
    async fn replace_body_weight(&self, body_weight: BodyWeight) -> Result<BodyWeight, String> {
        fetch(
            Request::put(&format!("api/body_weight/{}", body_weight.date))
                .json(&json!({ "weight": body_weight.weight }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_body_weight(&self, date: NaiveDate) -> Result<NaiveDate, String> {
        fetch_no_content(
            Request::delete(&format!("api/body_weight/{date}"))
                .build()
                .unwrap(),
            date,
        )
        .await
    }

    async fn read_body_fat(&self) -> Result<Vec<BodyFat>, String> {
        fetch(Request::get("api/body_fat").build().unwrap()).await
    }
    async fn create_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, String> {
        fetch(
            Request::post("api/body_fat")
                .json(&body_fat)
                .expect("serialization failed"),
        )
        .await
    }
    async fn replace_body_fat(&self, body_fat: BodyFat) -> Result<BodyFat, String> {
        fetch(
            Request::put(&format!("api/body_fat/{}", body_fat.date))
                .json(&json!({
                    "chest": body_fat.chest,
                    "abdominal": body_fat.abdominal,
                    "thigh": body_fat.thigh,
                    "tricep": body_fat.tricep,
                    "subscapular": body_fat.subscapular,
                    "suprailiac": body_fat.suprailiac,
                    "midaxillary": body_fat.midaxillary,
                }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_body_fat(&self, date: NaiveDate) -> Result<NaiveDate, String> {
        fetch_no_content(
            Request::delete(&format!("api/body_fat/{date}"))
                .build()
                .unwrap(),
            date,
        )
        .await
    }

    async fn read_period(&self) -> Result<Vec<Period>, String> {
        fetch(Request::get("api/period").build().unwrap()).await
    }
    async fn create_period(&self, period: Period) -> Result<Period, String> {
        fetch(
            Request::post("api/period")
                .json(&period)
                .expect("serialization failed"),
        )
        .await
    }
    async fn replace_period(&self, period: Period) -> Result<Period, String> {
        fetch(
            Request::put(&format!("api/period/{}", period.date))
                .json(&json!({ "intensity": period.intensity }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_period(&self, date: NaiveDate) -> Result<NaiveDate, String> {
        fetch_no_content(
            Request::delete(&format!("api/period/{date}"))
                .build()
                .unwrap(),
            date,
        )
        .await
    }

    async fn read_exercises(&self) -> Result<Vec<Exercise>, String> {
        fetch(Request::get("api/exercises").build().unwrap()).await
    }
    async fn create_exercise(
        &self,
        name: String,
        muscles: Vec<ExerciseMuscle>,
    ) -> Result<Exercise, String> {
        fetch(
            Request::post("api/exercises")
                .json(&json!({ "name": name, "muscles": muscles }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn replace_exercise(&self, exercise: Exercise) -> Result<Exercise, String> {
        fetch(
            Request::put(&format!("api/exercises/{}", exercise.id))
                .json(&exercise)
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_exercise(&self, id: u32) -> Result<u32, String> {
        fetch_no_content(
            Request::delete(&format!("api/exercises/{id}"))
                .build()
                .unwrap(),
            id,
        )
        .await
    }

    async fn read_routines(&self) -> Result<Vec<Routine>, String> {
        fetch(Request::get("api/routines").build().unwrap()).await
    }
    async fn create_routine(
        &self,
        name: String,
        sections: Vec<RoutinePart>,
    ) -> Result<Routine, String> {
        fetch(
            Request::post("api/routines")
                .json(&json!({
                    "name": name,
                    "notes": "",
                    "archived": false,
                    "sections": sections
                }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn modify_routine(
        &self,
        id: u32,
        name: Option<String>,
        archived: Option<bool>,
        sections: Option<Vec<RoutinePart>>,
    ) -> Result<Routine, String> {
        let mut content = Map::new();
        if let Some(name) = name {
            content.insert("name".into(), json!(name));
        }
        if let Some(archived) = archived {
            content.insert("archived".into(), json!(archived));
        }
        if let Some(sections) = sections {
            content.insert("sections".into(), json!(sections));
        }
        fetch(
            Request::patch(&format!("api/routines/{id}"))
                .json(&content)
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_routine(&self, id: u32) -> Result<u32, String> {
        fetch_no_content(
            Request::delete(&format!("api/routines/{id}"))
                .build()
                .unwrap(),
            id,
        )
        .await
    }

    async fn read_training_sessions(&self) -> Result<Vec<TrainingSession>, String> {
        fetch(Request::get("api/workouts").build().unwrap()).await
    }
    async fn create_training_session(
        &self,
        routine_id: Option<u32>,
        date: NaiveDate,
        notes: String,
        elements: Vec<TrainingSessionElement>,
    ) -> Result<TrainingSession, String> {
        fetch(
            Request::post("api/workouts")
                .json(&json!({
                    "routine_id": routine_id,
                    "date": date,
                    "notes": notes,
                    "elements": elements
                }))
                .expect("serialization failed"),
        )
        .await
    }
    async fn modify_training_session(
        &self,
        id: u32,
        notes: Option<String>,
        elements: Option<Vec<TrainingSessionElement>>,
    ) -> Result<TrainingSession, String> {
        let mut content = Map::new();
        if let Some(notes) = notes {
            content.insert("notes".into(), json!(notes));
        }
        if let Some(elements) = elements {
            content.insert("elements".into(), json!(elements));
        }
        fetch(
            Request::patch(&format!("api/workouts/{id}"))
                .json(&content)
                .expect("serialization failed"),
        )
        .await
    }
    async fn delete_training_session(&self, id: u32) -> Result<u32, String> {
        fetch_no_content(
            Request::delete(&format!("api/workouts/{id}"))
                .build()
                .unwrap(),
            id,
        )
        .await
    }
}

async fn fetch<'a, T>(request: Request) -> Result<T, String>
where
    T: 'static + for<'de> serde::Deserialize<'de>,
{
    match request.send().await {
        Ok(response) => {
            if response.ok() {
                match response.json::<T>().await {
                    Ok(data) => Ok(data),
                    Err(error) => Err(format!("deserialization failed: {error:?}")),
                }
            } else {
                Err(format!("{} {}", response.status(), response.status_text()))
            }
        }
        Err(_) => Err("no connection".into()),
    }
}

async fn fetch_no_content<'a, T>(request: Request, result: T) -> Result<T, String> {
    match request.send().await {
        Ok(response) => {
            if response.ok() {
                Ok(result)
            } else {
                Err(format!("{} {}", response.status(), response.status_text()))
            }
        }
        Err(_) => Err("no connection".into()),
    }
}
