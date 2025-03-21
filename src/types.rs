use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ApiResponse<T> {
    pub data: T,
}

#[derive(Deserialize, Debug)]
pub struct Instance {
    pub id: Option<String>,
    pub instance_type: Option<InstanceType>,
    pub status: Option<String>,
    pub ip: Option<String>,
    pub ssh_key_names: Option<Vec<String>>,
}

#[derive(Deserialize, Debug)]
pub struct LaunchResponse {
    pub instance_ids: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InstanceTypeResponse {
    pub instance_type: InstanceType,
    pub regions_with_capacity_available: Vec<Region>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InstanceType {
    pub name: String,
    pub description: String,
    pub price_cents_per_hour: i32,
    pub specs: InstanceSpecs,
}

#[derive(Deserialize, Debug, Clone)]
pub struct InstanceSpecs {
    pub vcpus: u32,
    pub memory_gib: u32,
    pub storage_gib: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Region {
    pub name: String,
    pub description: String,
}