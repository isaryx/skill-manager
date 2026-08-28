use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct StatusJson<'a> {
    pub agent: &'a str,
    pub skills_path: String,
    pub profile: Option<&'a str>,
    pub skills: Vec<StatusSkillJson>,
    pub conflicts: Vec<StatusConflictJson>,
}

#[derive(Debug, Serialize)]
pub struct StatusSkillJson {
    pub name: String,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct StatusConflictJson {
    pub name: String,
    pub store_id: String,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct LsJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<Vec<String>>,
}

pub fn write_json<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}
